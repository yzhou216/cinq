// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of messages going out over loopback

use std::{
    collections::BTreeMap,
    io::{ErrorKind, Read},
    iter,
    net::{TcpListener, TcpStream, UdpSocket},
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant},
};

use super::{Network, ipv4_address, udp_socket};
use crate::back::osc::{Destination, Outgoing, framed};

/// Longest that a test waits, which only a broken test reaches
const PATIENCE: Duration = Duration::from_secs(10);

fn message(destination: Destination, byte: u8) -> Vec<Outgoing> {
    vec![Outgoing {
        destination,
        packet: vec![byte; 4],
    }]
}

fn accepted(listener: &TcpListener) -> TcpStream {
    listener.set_nonblocking(true).unwrap();
    let start = Instant::now();
    let (stream, _) = iter::repeat_with(|| listener.accept())
        .find_map(|accepted| {
            assert!(start.elapsed() < PATIENCE, "no connection arrived");
            thread::sleep(Duration::from_millis(10));
            accepted.ok()
        })
        .unwrap();
    // Taken from a listener that does not block, which some systems pass on
    stream.set_nonblocking(false).unwrap();
    stream.set_read_timeout(Some(PATIENCE)).unwrap();
    stream
}

fn next(mut stream: &TcpStream) -> Vec<u8> {
    let mut frame = vec![0; framed(&[0; 4]).len()];
    stream.read_exact(&mut frame).unwrap();
    frame
}

/// Returns the next frame on a connection of its own
fn frame(listener: &TcpListener) -> Vec<u8> {
    next(&accepted(listener))
}

#[test]
fn tcp_messages_of_a_burst_go_out_framed_over_one_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let destination = Destination::Tcp(listener.local_addr().unwrap().to_string());
    let network = Network::open();
    network.send(message(destination.clone(), 1));
    network.send(message(destination, 2));
    let stream = accepted(&listener);
    assert_eq!(next(&stream), framed(&[1; 4]));
    assert_eq!(next(&stream), framed(&[2; 4]));
}

#[test]
fn tcp_message_after_a_quiet_second_goes_over_a_new_connection_while_others_go_out() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let destination = Destination::Tcp(listener.local_addr().unwrap().to_string());
    let elsewhere = UdpSocket::bind("127.0.0.1:0").unwrap();
    let other = Destination::Udp(elsewhere.local_addr().unwrap().to_string());
    let network = Network::open();
    network.send(message(destination.clone(), 1));
    let mut first = accepted(&listener);
    assert_eq!(next(&first), framed(&[1; 4]));
    // Five pauses of 300 ms outlast the quiet second
    (0..5).for_each(|_| {
        thread::sleep(Duration::from_millis(300));
        network.send(message(other.clone(), 2));
    });
    network.send(message(destination, 3));
    assert_eq!(frame(&listener), framed(&[3; 4]));
    assert_eq!(first.read(&mut [0; 16]).unwrap(), 0);
}

#[test]
fn tcp_message_after_the_destination_closed_goes_over_a_new_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let destination = Destination::Tcp(listener.local_addr().unwrap().to_string());
    let network = Network::open();
    network.send(message(destination.clone(), 1));
    assert_eq!(frame(&listener), framed(&[1; 4]));
    // Time for the close to arrive, well within a burst
    thread::sleep(Duration::from_millis(100));
    network.send(message(destination, 2));
    assert_eq!(frame(&listener), framed(&[2; 4]));
}

#[test]
fn messages_to_a_host_name_go_to_its_ipv4_address() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.set_read_timeout(Some(PATIENCE)).unwrap();
    let port = socket.local_addr().unwrap().port();
    Network::open().send(message(Destination::Udp(format!("localhost:{port}")), 4));
    let mut datagram = [0; 16];
    let length = socket.recv(&mut datagram).unwrap();
    assert_eq!(datagram[..length], [4; 4]);
}

#[test]
fn a_host_that_cannot_be_found_is_told_in_few_words() {
    let error = ipv4_address("no-such-host.invalid:9000").unwrap_err();
    assert_eq!(error.to_string(), "cannot find the host");
    let error = ipv4_address("no port").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
}

#[test]
fn udp_messages_go_out_as_one_datagram_each() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.set_read_timeout(Some(PATIENCE)).unwrap();
    let destination = Destination::Udp(socket.local_addr().unwrap().to_string());
    Network::open().send(message(destination, 3));
    let mut datagram = [0; 16];
    let length = socket.recv(&mut datagram).unwrap();
    assert_eq!(datagram[..length], [3; 4]);
}

fn settled(network: &Network) {
    let start = Instant::now();
    while network.shared.pending.load(Ordering::Relaxed) > 0 && start.elapsed() < PATIENCE {
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn failure_to_reach_a_destination_shows_until_a_message_gets_there() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let destination = Destination::Tcp(address.to_string());
    let named = named(std::slice::from_ref(&destination));
    let network = Network::open();
    network.send(message(destination.clone(), 5));
    settled(&network);
    let problems = network.problems(&named);
    assert_eq!(problems.len(), 1);
    assert_eq!(
        problems[0].1, "message not sent: connection refused",
        "the words without the destination or the error number"
    );
    let listener = TcpListener::bind(address).unwrap();
    network.send(message(destination, 6));
    assert_eq!(frame(&listener), framed(&[6; 4]));
    settled(&network);
    assert!(network.problems(&named).is_empty());
}

#[test]
fn message_that_waited_past_its_second_is_dropped_rather_than_sent_late() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let destinations = [
        Destination::Udp(socket.local_addr().unwrap().to_string()),
        Destination::Tcp(listener.local_addr().unwrap().to_string()),
    ];
    let network = Network::open();
    let handed = Instant::now().checked_sub(Duration::from_secs(2)).unwrap();
    network.send_since(
        destinations
            .iter()
            .cloned()
            .flat_map(|to| message(to, 7))
            .collect(),
        handed,
    );
    settled(&network);
    let problems = network.problems(&named(&destinations));
    assert_eq!(problems.len(), 2);
    assert!(problems.iter().all(|(_, problem)| {
        problem.ends_with(": dropped rather than sent more than a second late")
    }));
    assert!(socket.recv(&mut [0; 16]).is_err());
    listener.set_nonblocking(true).unwrap();
    assert!(
        listener
            .accept()
            .is_err_and(|error| error.kind() == ErrorKind::WouldBlock)
    );
}

#[test]
fn udp_messages_may_go_to_a_broadcast_address() {
    assert!(udp_socket().unwrap().broadcast().unwrap());
}

fn named(destinations: &[Destination]) -> BTreeMap<String, ()> {
    destinations.iter().map(|to| (to.to_string(), ())).collect()
}

#[test]
fn warning_of_a_destination_that_no_cue_names_any_more_goes_away() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    drop(listener);
    let destination = Destination::Tcp(address);
    let network = Network::open();
    network.send(message(destination.clone(), 8));
    settled(&network);
    assert_eq!(
        network
            .problems(&named(std::slice::from_ref(&destination)))
            .len(),
        1
    );
    assert!(network.problems(&BTreeMap::<String, ()>::new()).is_empty());
    assert!(
        network
            .problems(&named(std::slice::from_ref(&destination)))
            .is_empty()
    );
}

/// Returns a listener whose backlog is full, so that a connection to it
/// hangs, with the connections that fill it.  Only Linux drops the
/// handshakes of such a listener, while other systems refuse them at once
/// or take them, so the tests that use it run on Linux only.
#[cfg(target_os = "linux")]
fn unresponsive() -> (socket2::Socket, Vec<TcpStream>) {
    let listener =
        socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None).unwrap();
    listener
        .bind(
            &"127.0.0.1:0"
                .parse::<std::net::SocketAddr>()
                .unwrap()
                .into(),
        )
        .unwrap();
    listener.listen(0).unwrap();
    let address = listener.local_addr().unwrap().as_socket().unwrap();
    let filled: Vec<TcpStream> =
        iter::repeat_with(|| TcpStream::connect_timeout(&address, Duration::from_millis(200)))
            .map_while(Result::ok)
            .take(16)
            .collect();
    assert!(filled.len() < 16, "the backlog did not fill");
    (listener, filled)
}

#[test]
#[cfg(target_os = "linux")]
fn tcp_destination_that_does_not_answer_does_not_delay_messages_to_others() {
    let (silent, _filled) = unresponsive();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let batch = [
        Destination::Tcp(
            silent
                .local_addr()
                .unwrap()
                .as_socket()
                .unwrap()
                .to_string(),
        ),
        Destination::Udp(socket.local_addr().unwrap().to_string()),
    ]
    .into_iter()
    .flat_map(|to| message(to, 8))
    .collect();
    let network = Network::open();
    let alone = message(
        Destination::Udp(socket.local_addr().unwrap().to_string()),
        9,
    );
    network.send(alone);
    assert!(
        socket.recv(&mut [0; 16]).is_ok(),
        "control: a datagram alone arrives"
    );
    network.send(batch);
    assert!(
        socket.recv(&mut [0; 16]).is_ok(),
        "the datagram was dropped as late"
    );
}

#[test]
fn problem_comes_with_the_value_that_names_its_destination() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let destination = Destination::Tcp(listener.local_addr().unwrap().to_string());
    drop(listener);
    let network = Network::open();
    network.send(message(destination.clone(), 9));
    settled(&network);
    let named = BTreeMap::from([(destination.to_string(), 4_u8)]);
    let problems = network.problems(&named);
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].0, 4);
}
