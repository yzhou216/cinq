// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of network cues, which send their message once each time that
//! they start, and cannot play without a destination and a message

use std::path::Path;

use super::*;
use crate::back::{Destination, Outgoing, osc_packet};

fn fire(id: u32, pre_wait: f32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Network, "Fire cue 1")
        .destination("10.101.100.20:3037")
        .message("/eos/cue/1/fire")
        .pre_wait(pre_wait)
        .build()
}

#[test]
fn network_cue_sends_its_message_once_as_it_starts() {
    let expected = Outgoing {
        destination: Destination::Tcp("10.101.100.20:3037".to_owned()),
        packet: osc_packet("/eos/cue/1/fire").unwrap(),
    };
    let (waiting, none) = workspace(vec![fire(1, 0.5)]).apply(GO).drained();
    assert!(none.is_empty());
    let (started, messages) = waiting.apply(Command::Tick(0.5)).drained();
    assert_eq!(messages, [expected]);
    let (_, again) = started.apply(Command::Tick(1.0)).drained();
    assert!(again.is_empty());
}

#[test]
fn network_cue_without_destination_or_message_cannot_play() {
    let cue = |destination: &str, message: &str| {
        Cue::new(CueId(1), "1", Kind::Network, "Send")
            .destination(destination)
            .message(message)
            .build()
    };
    let problem = |cue| {
        let workspace = workspace(vec![cue]);
        workspace
            .problem(&workspace.lists()[0].cues[0])
            .map(|p| p.to_string())
    };
    assert_eq!(
        problem(cue("ion", "/eos/ping")).as_deref(),
        Some("no destination as host:port")
    );
    assert_eq!(
        problem(cue("ion:3037", "eos/ping")).as_deref(),
        Some("no OSC message")
    );
    assert_eq!(problem(cue("udp://ion:8000", "/eos/ping")), None);
    let (_, messages) = workspace(vec![cue("ion", "/eos/ping")]).apply(GO).drained();
    assert!(messages.is_empty());
}

#[test]
fn show_file_holds_the_destination_and_message_of_network_cues_alone() {
    let show = workspace(vec![fire(1, 0.0)]).show(Some(Path::new("/show")));
    assert!(show.contains("destination = \"10.101.100.20:3037\"\nmessage = \"/eos/cue/1/fire\"\n"));
    let read = Workspace::from_show(&show, "/show").unwrap();
    assert_eq!(read.show(Some(Path::new("/show"))), show);
    let memo = Cue::new(CueId(1), "1", Kind::Memo, "Note")
        .destination("ion:1")
        .build();
    assert!(!workspace(vec![memo]).show(None).contains("destination"));
}

#[test]
fn destinations_are_those_that_network_cues_name_as_they_read() {
    let udp = Cue::new(CueId(2), "2", Kind::Network, "Send")
        .destination("udp://10.101.100.21:8000 ")
        .message("/x")
        .build();
    let memo = Cue::new(CueId(3), "3", Kind::Memo, "Note")
        .destination("10.0.0.1:1")
        .build();
    let unreadable = Cue::new(CueId(4), "4", Kind::Network, "Send")
        .destination("nowhere")
        .build();
    let named = workspace(vec![fire(1, 0.0), udp, memo, unreadable]).destinations();
    assert_eq!(
        named.iter().map(String::as_str).collect::<Vec<_>>(),
        ["10.101.100.20:3037", "udp://10.101.100.21:8000"]
    );
}
