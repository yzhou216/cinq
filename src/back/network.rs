// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Network output, where the back end has side effects
//!
//! [`Network::send`] hands the messages of network cues to a thread of its
//! own, which passes each to a worker thread of its destination.  A worker
//! sends its messages in order over a TCP connection that lasts through a
//! burst, as [`connection`] tells, or as one UDP datagram each.  Messages
//! to different destinations therefore do not wait for each other, and the
//! host name of a destination is resolved on its own worker.
//!
//! A message that cannot go out within [`PATIENCE`] of being handed over,
//! as one behind a console that does not answer, is dropped rather than
//! sent late, since a console cue that fires late does more harm than one
//! that does not fire.  Why a message to a destination last failed stays
//! for the interface to show until a message gets there or no cue names the
//! destination any more, and warns once on standard error.

mod connection;
#[cfg(test)]
mod tests;

use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, ErrorKind},
    net::{Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket},
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    thread,
    time::{Duration, Instant},
};

use super::osc::{self, Destination, Outgoing};
use crate::warn;

/// Longest that a message may wait to go out, and longest quiet that a TCP
/// connection outlasts
const PATIENCE: Duration = Duration::from_secs(1);

const CHECK: Duration = Duration::from_millis(100);

#[derive(Default)]
struct Shared {
    /// Why the last message to each destination failed, until one gets there
    problems: Mutex<BTreeMap<String, String>>,
    pending: AtomicUsize,
}

impl Shared {
    /// Ignores poisoning, since a panic leaves the problems as they were
    fn problems(&self) -> MutexGuard<'_, BTreeMap<String, String>> {
        self.problems.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub struct Network {
    sender: mpsc::Sender<Vec<(Instant, Outgoing)>>,
    shared: Arc<Shared>,
}

impl Network {
    pub fn open() -> Self {
        let (sender, messages) = mpsc::channel();
        let shared = Arc::<Shared>::default();
        let sending = Arc::clone(&shared);
        thread::spawn(move || dispatch(&messages, &sending));
        Self { sender, shared }
    }

    pub fn send(&self, messages: Vec<Outgoing>) {
        self.send_since(messages, Instant::now());
    }

    /// Hands over `messages` as though at `handed`, for tests of lateness
    fn send_since(&self, messages: Vec<Outgoing>, handed: Instant) {
        if !messages.is_empty() {
            self.shared
                .pending
                .fetch_add(messages.len(), Ordering::Relaxed);
            let stamped = messages
                .into_iter()
                .map(|message| (handed, message))
                .collect();
            // Fails only once the thread has gone, which a panic there causes
            let _ = self.sender.send(stamped);
        }
    }

    /// Returns why messages to the `named` destinations last failed.  A
    /// destination that no cue names any more is forgotten, since fixing a
    /// typo in a cue would otherwise leave the warning of the old one.
    pub fn problems(&self, named: &BTreeSet<String>) -> Vec<String> {
        let mut problems = self.shared.problems();
        problems.retain(|destination, _| named.contains(destination));
        problems
            .iter()
            .map(|(destination, reason)| format!("Network cue to {destination}: {reason}"))
            .collect()
    }

    /// Returns how often the interface should look again while messages are
    /// on their way, since they can change the problems
    pub fn checks(&self) -> Option<Duration> {
        (self.shared.pending.load(Ordering::Relaxed) > 0).then_some(CHECK)
    }
}

/// Queue into the worker of one destination, with each message as handed
/// over
type Queue = mpsc::Sender<(Instant, Outgoing)>;

/// Open TCP connection of a destination, with when it last took a message
type Connection = (Instant, TcpStream);

/// Hands each message to the worker of its destination, which it starts at
/// the first message, so that a destination that does not answer cannot
/// delay the others.  A worker whose destination has been quiet for
/// [`PATIENCE`] is let go: it delivers what is queued, ends, and closes its
/// connection.
fn dispatch(messages: &mpsc::Receiver<Vec<(Instant, Outgoing)>>, shared: &Arc<Shared>) {
    // Each worker's queue by destination, with when it last took a message
    let mut workers = BTreeMap::<String, (Instant, Queue)>::new();
    loop {
        let batch = match messages.recv_timeout(PATIENCE) {
            Ok(batch) => batch,
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        for (handed, outgoing) in batch {
            let destination = outgoing.destination.to_string();
            let queue = workers
                .remove(&destination)
                .map_or_else(|| started(&destination, shared), |(_, queue)| queue);
            // Fails only once the worker has gone, which a panic there causes
            let _ = queue.send((handed, outgoing));
            workers.insert(destination, (Instant::now(), queue));
        }
        workers.retain(|_, (used, _)| used.elapsed() <= PATIENCE);
    }
}

fn started(destination: &str, shared: &Arc<Shared>) -> Queue {
    let (queue, messages) = mpsc::channel();
    let destination = destination.to_owned();
    let shared = Arc::clone(shared);
    thread::spawn(move || work(&destination, &messages, &shared));
    queue
}

/// Delivers the messages to `destination` in order until its queue ends,
/// which closes the connection
fn work(destination: &str, messages: &mpsc::Receiver<(Instant, Outgoing)>, shared: &Shared) {
    messages.iter().fold(None, |kept, (handed, outgoing)| {
        // A burst ends after a quiet second, whenever the dispatcher notices
        let fresh = kept.filter(|(used, _): &Connection| used.elapsed() <= PATIENCE);
        let connection = match deliver(fresh, outgoing, handed) {
            Ok(connection) => {
                drop(shared.problems().remove(destination));
                connection
            }
            Err(error) => {
                warn::once(&format!("network cue to {destination}: {error}"));
                shared
                    .problems()
                    .insert(destination.to_owned(), error.to_string());
                None
            }
        };
        shared.pending.fetch_sub(1, Ordering::Relaxed);
        connection
    });
}

/// Returns the connection of the destination as it stays open, if any
fn deliver(
    kept: Option<Connection>,
    Outgoing {
        destination,
        packet,
    }: Outgoing,
    handed: Instant,
) -> io::Result<Option<Connection>> {
    match destination {
        Destination::Udp(address) => {
            time_left(handed)?;
            let to = ipv4_address(&address)?;
            // Finding the host may have taken the rest of the time
            time_left(handed)?;
            udp_socket()?.send_to(&packet, to).map(|_| None)
        }
        Destination::Tcp(address) => {
            let stream = kept.map(|(_, stream)| stream);
            connection::written(stream, &address, &osc::framed(&packet), handed)
                .map(|stream| Some((Instant::now(), stream)))
        }
    }
}

fn time_left(handed: Instant) -> io::Result<Duration> {
    PATIENCE
        .checked_sub(handed.elapsed())
        .filter(|left| !left.is_zero())
        .ok_or_else(|| {
            io::Error::new(
                ErrorKind::TimedOut,
                "dropped rather than sent more than a second late",
            )
        })
}

/// Network cues go out over IPv4, as the networks of consoles run, while a
/// resolver can give IPv6 first, as `::1` for `localhost`.
fn ipv4_address(address: &str) -> io::Result<SocketAddr> {
    address
        .to_socket_addrs()?
        .find(SocketAddr::is_ipv4)
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "no IPv4 address for the host"))
}

/// Returns a socket that may send to a broadcast address, such as
/// `255.255.255.255`, which the system otherwise refuses with a permission
/// error
fn udp_socket() -> io::Result<UdpSocket> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.set_broadcast(true)?;
    Ok(socket)
}
