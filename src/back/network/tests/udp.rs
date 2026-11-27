// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of UDP messages, to host names and to broadcast addresses

use super::*;

#[test]
fn udp_messages_go_out_as_one_datagram_each_also_to_a_host_name() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.set_read_timeout(Some(PATIENCE)).unwrap();
    let port = socket.local_addr().unwrap().port();
    let network = Network::open();
    [
        (udp_to(&socket), 3),
        // Name that resolves to the IPv4 address
        (Destination::Udp(format!("localhost:{port}")), 4),
    ]
    .into_iter()
    .for_each(|(destination, byte)| {
        network.send(message(destination, byte));
        let mut datagram = [0; 16];
        let length = socket.recv(&mut datagram).unwrap();
        assert_eq!(datagram[..length], [byte; 4]);
    });
}

#[test]
fn a_host_that_cannot_be_found_is_told_in_few_words() {
    let error = ipv4_address("no-such-host.invalid:9000").unwrap_err();
    assert_eq!(error.to_string(), "cannot find the host");
    let error = ipv4_address("no port").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
}

#[test]
fn udp_messages_may_go_to_a_broadcast_address() {
    assert!(udp_socket().unwrap().broadcast().unwrap());
}
