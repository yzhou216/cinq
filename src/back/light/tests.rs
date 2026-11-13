// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the packets that go out

use super::packet;

/// Port-Address of an ArtDmx packet, which follows its 8-byte ID, opcode,
/// protocol version, sequence, and physical port
fn port_address(packet: &[u8]) -> u16 {
    u16::from_le_bytes([packet[14], packet[15]])
}

#[test]
fn universes_go_out_as_the_port_address_before_them_as_consoles_send_them() {
    let channels = [0; 512];
    assert_eq!(packet(1, &channels).map(|p| port_address(&p)), Some(0));
    assert_eq!(packet(17, &channels).map(|p| port_address(&p)), Some(16));
    assert_eq!(packet(0, &channels), None);
}
