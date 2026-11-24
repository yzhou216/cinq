// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hints while hint mode is on

use super::{hinted, opened_with};
use crate::front::{
    App, Hints,
    keymap::{Keymap, desk},
};

fn labelling(keymap: Keymap) -> Vec<String> {
    hinted(&App {
        hints: Some(Hints::default()),
        ..opened_with(keymap, false, false)
    })
    .into_iter()
    .map(|(_, hint)| hint)
    .collect()
}

#[test]
fn hint_mode_hints_only_what_acts_there() {
    assert_eq!(
        labelling(Keymap::default()),
        [
            "Type a label",
            "Control+c: cancel",
            "Space: GO",
            "Escape: Panic"
        ]
    );
}

#[test]
fn backspace_leaves_where_the_keymap_binds_no_cancel() {
    assert_eq!(
        labelling(desk(""))[..2],
        ["Type a label", "Backspace: leave"]
    );
}

#[test]
fn backspace_takes_a_letter_back_once_one_is_typed() {
    let typed = |keymap| {
        hinted(&App {
            hints: Some(Hints::after("s")),
            ..opened_with(keymap, false, false)
        })
        .into_iter()
        .map(|(_, hint)| hint)
        .take(2)
        .collect::<Vec<_>>()
    };
    assert_eq!(typed(desk("")), ["s", "Backspace: back"]);
    assert_eq!(typed(Keymap::default()), ["s", "Control+c: cancel"]);
}
