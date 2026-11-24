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
mod status;
#[cfg(test)]
mod tests;

use std::{
    collections::BTreeMap,
    io,
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use if_addrs::IfAddr;
use socket2::SockRef;
use uuid::Uuid;

use self::{leaving::Leaving, listing::Listing, status::Round};
use super::{Workspace, workspace::Universe};
use crate::warn;

/// Time after which unchanged levels go out again
const REFRESH: Duration = Duration::from_secs(1);

type Universes = BTreeMap<u16, Universe>;

/// Interval at which to look for what the thread tells, while the patch has
/// an instrument
const CHECK: Duration = Duration::from_secs(1);

pub struct Lights {
    sent: Universes,
    sender: mpsc::Sender<Universes>,
    told: mpsc::Receiver<Option<String>>,
    /// Why no light gets out, in words, until a round gets some out
    problem: Option<String>,
}

impl Lights {
    pub fn open() -> Self {
        let (sender, levels) = mpsc::channel();
        let (reports, told) = mpsc::channel();
        thread::spawn(move || send(&levels, &reports));
        Self {
            sent: Universes::new(),
            sender,
            told,
            problem: None,
        }
    }

    pub fn sync(&mut self, workspace: &Workspace) {
        let universes = workspace.universes();
        if universes != self.sent {
            // Thread has gone only where it could not open a socket
            let _ = self.sender.send(universes.clone());
            self.sent = universes;
        }
        if let Some(latest) = self.told.try_iter().last() {
            self.problem = latest;
        }
    }

    /// Returns why no light gets out, while the patch has an instrument,
    /// since a show without one sends nothing
    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref().filter(|_| !self.sent.is_empty())
    }

    /// Returns how often to look for news of the output, which only the
    /// thread knows, while the patch has an instrument
    pub fn checks(&self) -> Option<Duration> {
        (!self.sent.is_empty()).then_some(CHECK)
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
    /// Sends the packets of a universe and returns how each send went
    fn send(
        &self,
        socket: &UdpSocket,
        universe: u16,
        artnet: Option<&[u8]>,
        sacn: &[u8],
    ) -> Vec<io::Result<usize>> {
        let (artnet_to, sacn_to) = if self.loopback {
            let to = SocketAddr::from((self.address, sacn::PORT));
            (Some(self.address), Some(to))
        } else {
            (self.broadcast, sacn::group(universe))
        };
        // Interface gone since, which the next refresh leaves out
        let _ = SockRef::from(socket).set_multicast_if_v4(&self.address);
        let sacn_sent = sacn_to.map(|to| socket.send_to(sacn, to));
        let artnet_sent = artnet
            .zip(artnet_to)
            .map(|(packet, to)| socket.send_to(packet, (to, artnet::PORT)));
        sacn_sent.into_iter().chain(artnet_sent).collect()
    }
}

/// Sends universes as they arrive and the latest again after each quiet
/// second, until the sender is dropped.  A universe that has left the
/// patch goes out as zeros, then as a terminated stream, at the pace of the
/// workspace.  Each change in whether any light gets out goes to `reports`,
/// and once to standard error.
fn send(levels: &mpsc::Receiver<Universes>, reports: &mpsc::Sender<Option<String>>) {
    let opened = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|socket| socket.set_broadcast(true).map(|()| socket));
    let socket = match opened {
        Ok(socket) => socket,
        Err(error) => {
            tell(reports, Some(format!("not sent: {}", warn::words(&error))));
            return;
        }
    };
    // Random for each run, so that receivers tell sources apart
    let cid = Uuid::new_v4();
    let mut latest = Universes::new();
    let mut leaving = Leaving::default();
    let mut listing = Listing::new();
    let mut announced = None::<Instant>;
    let mut problem = None::<String>;
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
        let due = !latest.is_empty() || !leaving.is_empty();
        let everywhere = |universe, artnet: Option<&[u8]>, sacn: &[u8]| {
            interfaces
                .iter()
                .flat_map(|interface| interface.send(&socket, universe, artnet, sacn))
                .collect::<Vec<_>>()
        };
        let steady = latest
            .iter()
            .flat_map(|(&universe, channels)| {
                let artnet = artnet::packet(universe, channels);
                everywhere(
                    universe,
                    artnet.as_deref(),
                    &sacn::packet(cid, universe, sequence, channels, false),
                )
            })
            .collect::<Vec<_>>();
        let ending = leaving
            .frame()
            .flat_map(|(universe, terminated)| {
                let zeros = [0; 512];
                everywhere(
                    universe,
                    artnet::packet(universe, &zeros).as_deref(),
                    &sacn::packet(cid, universe, sequence, &zeros, terminated),
                )
            })
            .collect::<Vec<_>>();
        leaving = leaving.next();
        let listed = if !latest.is_empty()
            && announced.is_none_or(|at| at.elapsed() >= sacn::DISCOVERY_INTERVAL)
        {
            let universes: Vec<u16> = latest.keys().copied().collect();
            announced = Some(Instant::now());
            sacn::discovery(cid, &universes)
                .iter()
                .flat_map(|page| everywhere(sacn::DISCOVERY, None, page))
                .collect()
        } else {
            Vec::new()
        };
        let sends: Vec<_> = steady.into_iter().chain(ending).chain(listed).collect();
        if let Some(now) = Round::of(interfaces.len(), due, &sends).told(&problem) {
            tell(reports, now.clone());
            problem = now;
        }
    }
}

/// Hands the sidebar what changed in whether light gets out, and standard
/// error the problem, once
fn tell(reports: &mpsc::Sender<Option<String>>, problem: Option<String>) {
    if let Some(words) = &problem {
        warn::once(&format!("light output: {words}"));
    }
    // Receiver has gone only as cinq ends
    let _ = reports.send(problem);
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
