// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Open Sound Control, which network cues send, as text that the operator
//! writes and as the packets that go out
//!
//! A destination reads as `host:port` for TCP, with packets framed by SLIP
//! as OSC 1.1 asks of streams, or as `udp://host:port` for UDP, one packet
//! to a datagram.  A message reads as its address, then its arguments
//! after spaces, each an integer, a decimal number, or else a string,
//! which double quotes keep whole across spaces, as in
//! `/scene/recall "Act 1" 2`.
//!
//! The encoding is written by hand, since the `rosc` crate is unmaintained.
//! So is the decoding of the packets that arrive, which [`incoming`] reads,
//! [`remote`] turns into commands, and [`listen`] receives over UDP.

mod incoming;
mod listen;
mod remote;
#[cfg(test)]
mod tests;

use std::fmt;

pub use listen::{Heard, listen};
pub use remote::Remote;

/// Ends a SLIP frame and starts one too, so that a receiver drops any noise
/// before it
const END: u8 = 0xc0;

const ESC: u8 = 0xdb;

/// Bytes that follow ESC in place of an END and an ESC
const ESCAPED_END: u8 = 0xdc;
const ESCAPED_ESC: u8 = 0xdd;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Destination {
    Tcp(String),
    Udp(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Outgoing {
    pub destination: Destination,
    pub packet: Vec<u8>,
}

impl fmt::Display for Destination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tcp(address) => f.write_str(address),
            Self::Udp(address) => write!(f, "udp://{address}"),
        }
    }
}

impl Destination {
    /// Reads `host:port`, with or without a `tcp://` before it, or
    /// `udp://host:port`, and returns none for text of any other form
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let (destination, address): (fn(String) -> Self, _) = match text.strip_prefix("udp://") {
            Some(address) => (Self::Udp, address),
            None => (Self::Tcp, text.strip_prefix("tcp://").unwrap_or(text)),
        };
        let (host, port) = address.rsplit_once(':')?;
        let valid = !host.is_empty()
            && !host.contains(char::is_whitespace)
            && port.parse::<u16>().is_ok_and(|port| port > 0);
        valid.then(|| destination(address.to_owned()))
    }
}

/// Returns the OSC message that `text` reads as, or none unless it starts
/// with an address and holds no null, which OSC reads as the end of a
/// string
pub fn packet(text: &str) -> Option<Vec<u8>> {
    if text.contains('\0') {
        return None;
    }
    let mut words = words(text).into_iter();
    let address = words
        .next()
        .filter(|address| !address.quoted && address.text.starts_with('/'))?;
    let (tags, arguments): (String, Vec<Vec<u8>>) = words.map(argument).unzip();
    Some(
        [padded(&address.text), padded(&format!(",{tags}"))]
            .into_iter()
            .chain(arguments)
            .flatten()
            .collect(),
    )
}

/// Returns `packet` as one SLIP frame, for a stream
pub fn framed(packet: &[u8]) -> Vec<u8> {
    let escaped = packet.iter().flat_map(|&byte| match byte {
        END => vec![ESC, ESCAPED_END],
        ESC => vec![ESC, ESCAPED_ESC],
        byte => vec![byte],
    });
    [END].into_iter().chain(escaped).chain([END]).collect()
}

fn argument(word: Word) -> (char, Vec<u8>) {
    let Word { text, quoted } = word;
    match (quoted, text.parse::<i32>(), text.parse::<f32>()) {
        (false, Ok(integer), _) => ('i', integer.to_be_bytes().to_vec()),
        // Words such as `inf` and `nan` stay strings
        (false, _, Ok(float)) if float.is_finite() => ('f', float.to_be_bytes().to_vec()),
        _ => ('s', padded(&text)),
    }
}

/// OSC string: the bytes, a null, and more nulls to a multiple of four
fn padded(text: &str) -> Vec<u8> {
    let length = (text.len() / 4 + 1) * 4;
    text.bytes()
        .chain(std::iter::repeat(0))
        .take(length)
        .collect()
}

struct Word {
    text: String,
    quoted: bool,
}

fn words(text: &str) -> Vec<Word> {
    text.split('"')
        .enumerate()
        .flat_map(|(part, between)| {
            // Parts alternate between outside and inside the quotes
            let quoted = part % 2 == 1;
            let texts = if quoted {
                vec![between]
            } else {
                between.split_whitespace().collect()
            };
            texts.into_iter().map(move |text| Word {
                text: text.to_owned(),
                quoted,
            })
        })
        .collect()
}
