// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Light output, where the back end has side effects
//!
//! [`Lights::sync`] runs after every change to the workspace, as
//! [`super::Audio::sync`] does, and hands the DMX levels of the light patch
//! to a thread of its own whenever they change.  The thread sends each
//! universe as Art-Net to the broadcast address of every network that the
//! computer is on, and as sACN to the multicast group of the universe on
//! each of them, when it changes and again each second, so that a node that
//! starts late or loses a packet catches up.  Every ten seconds it also
//! lists its universes, as sACN asks of a source.  A show without
//! instruments sends nothing.
//!
//! A universe that leaves the patch, by an edit or by the opening of another
//! show, goes out as zeros for a few frames and then as a stream that says
//! that it has ended, so that no receiver keeps its last levels.
//!
//! When cinq ends, its sACN stream does not say that it has ended, since
//! receivers would drop the lights at once.  They hold the levels instead,
//! as they do for any source that has gone quiet.

mod artnet;
mod leaving;
mod listing;
mod sacn;
#[cfg(test)]
mod tests;

use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use if_addrs::IfAddr;
use socket2::SockRef;
use uuid::Uuid;

use self::{leaving::Leaving, listing::Listing};
use super::{Workspace, workspace::Universe};

/// Time after which unchanged levels go out again
const REFRESH: Duration = Duration::from_secs(1);

type Universes = BTreeMap<u16, Universe>;

pub struct Lights {
    sent: Universes,
    sender: mpsc::Sender<Universes>,
}

impl Lights {
    pub fn open() -> Self {
        let (sender, levels) = mpsc::channel();
        thread::spawn(move || send(&levels));
        Self {
            sent: Universes::new(),
            sender,
        }
    }

    pub fn sync(&mut self, workspace: &Workspace) {
        let universes = workspace.universes();
        if universes != self.sent {
            // Thread has gone only where it could not open a socket
            let _ = self.sender.send(universes.clone());
            self.sent = universes;
        }
    }
}

/// Network interface of the computer by its IPv4 address.  The loopback
/// interface reaches programs on the computer itself, such as visualizers,
/// whose firewall can keep them from hearing broadcasts and multicast.
struct Interface {
    address: Ipv4Addr,
    broadcast: Option<Ipv4Addr>,
    loopback: bool,
}

impl Interface {
    fn send(&self, socket: &UdpSocket, universe: u16, artnet: Option<&[u8]>, sacn: &[u8]) {
        let (artnet_to, sacn_to) = if self.loopback {
            let to = SocketAddr::from((self.address, sacn::PORT));
            (Some(self.address), Some(to))
        } else {
            (self.broadcast, sacn::group(universe))
        };
        // Interface gone since, which the next refresh leaves out
        let _ = SockRef::from(socket).set_multicast_if_v4(&self.address);
        if let Some(to) = sacn_to {
            let _ = socket.send_to(sacn, to);
        }
        if let (Some(packet), Some(to)) = (artnet, artnet_to) {
            let _ = socket.send_to(packet, (to, artnet::PORT));
        }
    }
}

/// Sends universes as they arrive and the latest again after each quiet
/// second, until the sender is dropped.  A universe that has left the
/// patch goes out as zeros, then as a terminated stream, at the pace of the
/// workspace.
fn send(levels: &mpsc::Receiver<Universes>) {
    let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|socket| socket.set_broadcast(true).map(|()| socket))
    else {
        return;
    };
    // Random for each run, so that receivers tell sources apart
    let cid = Uuid::new_v4();
    let mut latest = Universes::new();
    let mut leaving = Leaving::default();
    let mut listing = Listing::new();
    let mut announced = None::<Instant>;
    // One count for each round, since sACN numbers the packets of each
    // universe in order
    for sequence in (0..=u8::MAX).cycle() {
        let wait = if leaving.is_empty() {
            REFRESH
        } else {
            leaving::FRAME
        };
        (latest, leaving) = match levels.recv_timeout(wait) {
            Ok(universes) => {
                let leaving = leaving.changed(&latest, &universes);
                (universes, leaving)
            }
            Err(RecvTimeoutError::Timeout) => (latest, leaving),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        listing = listing.at(Instant::now(), interfaces);
        let interfaces = listing.items();
        let everywhere = |universe, artnet: Option<&[u8]>, sacn: &[u8]| {
            interfaces
                .iter()
                .for_each(|interface| interface.send(&socket, universe, artnet, sacn));
        };
        latest.iter().for_each(|(&universe, channels)| {
            let artnet = artnet::packet(universe, channels);
            everywhere(
                universe,
                artnet.as_deref(),
                &sacn::packet(cid, universe, sequence, channels, false),
            );
        });
        leaving.frame().for_each(|(universe, terminated)| {
            let zeros = [0; 512];
            everywhere(
                universe,
                artnet::packet(universe, &zeros).as_deref(),
                &sacn::packet(cid, universe, sequence, &zeros, terminated),
            );
        });
        leaving = leaving.next();
        if !latest.is_empty() && announced.is_none_or(|at| at.elapsed() >= sacn::DISCOVERY_INTERVAL)
        {
            let universes: Vec<u16> = latest.keys().copied().collect();
            sacn::discovery(cid, &universes)
                .iter()
                .for_each(|page| everywhere(sacn::DISCOVERY, None, page));
            announced = Some(Instant::now());
        }
    }
}

fn interfaces() -> Vec<Interface> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|interface| match interface.addr {
            IfAddr::V4(ref address) => Some(Interface {
                address: address.ip,
                broadcast: address.broadcast,
                loopback: interface.is_loopback(),
            }),
            IfAddr::V6(_) => None,
        })
        .collect()
}
