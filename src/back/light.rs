// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Light output, where the back end has side effects
//!
//! [`Lights::sync`] runs after every change to the workspace, as
//! [`super::Audio::sync`] does, and hands the DMX levels of the light patch
//! to a thread of its own whenever they change.  The thread sends each
//! universe as Art-Net to the broadcast address of every network that the
//! computer is on, once it changes and again each second, as Art-Net asks
//! of a controller, so that a node that starts late or loses a packet
//! catches up.  A show without instruments sends nothing.

#[cfg(test)]
mod tests;

use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};

use artnet_protocol::{ArtCommand, Output};
use if_addrs::IfAddr;

use super::{Workspace, workspace::Universe};

const PORT: u16 = 6454;

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

/// Sends the universes as they arrive, and the latest again each time that
/// none arrive for a while, until the other end goes
fn send(levels: &mpsc::Receiver<Universes>) {
    let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|socket| socket.set_broadcast(true).map(|()| socket))
    else {
        return;
    };
    let mut latest = Universes::new();
    loop {
        latest = match levels.recv_timeout(REFRESH) {
            Ok(universes) => universes,
            Err(RecvTimeoutError::Timeout) => latest,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let networks = broadcasts();
        latest
            .iter()
            .filter_map(|(&universe, channels)| packet(universe, channels))
            .for_each(|packet| {
                networks.iter().for_each(|network| {
                    // Network that has gone since, which the next refresh
                    // leaves out
                    let _ = socket.send_to(&packet, network);
                });
            });
    }
}

/// Returns the ArtDmx packet of `universe`, which goes out as the
/// Port-Address before it, as consoles send universe 1 as Port-Address 0
fn packet(universe: u16, channels: &Universe) -> Option<Vec<u8>> {
    let output = Output {
        port_address: universe.checked_sub(1)?.try_into().ok()?,
        data: channels.to_vec().into(),
        ..Output::default()
    };
    ArtCommand::Output(output).write_to_buffer().ok()
}

/// Returns the broadcast address of every IPv4 network that the computer is
/// on, and its own loopback address, since a firewall can keep a program on
/// the same computer, such as a visualizer, from hearing broadcasts
fn broadcasts() -> Vec<SocketAddr> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|interface| match interface.addr {
            IfAddr::V4(address) if interface.is_loopback() => Some(address.ip),
            IfAddr::V4(address) => address.broadcast,
            IfAddr::V6(_) => None,
        })
        .map(|address| SocketAddr::from((address, PORT)))
        .collect()
}
