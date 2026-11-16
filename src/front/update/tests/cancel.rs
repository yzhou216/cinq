// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the key that cancels: the keys typed so far and the field with
//! the keyboard, never playback

use super::{chord, press, show};
use crate::{
    back::{CueId, loaded, sound},
    front::{App, Message, keymap::Press},
};
use iced::keyboard::{Key, Modifiers};

fn control_c() -> Press {
    chord(Key::Character("c".into()), Modifiers::CTRL)
}

#[test]
fn control_c_drops_the_keys_typed_so_far_and_does_nothing_else() {
    let (halfway, _) = show(Some(CueId(2))).typed(press(Key::Character("d".into())), false);
    assert_eq!(halfway.keys.len(), 1);
    let (cancelled, _) = halfway.typed(control_c(), false);
    assert!(cancelled.keys.is_empty());
    assert_eq!(cancelled.selected, Some(CueId(2)));
    assert!(cancelled.workspace.latest_change().is_none());
}

#[test]
fn cancel_leaves_the_help_page_and_playback_alone() {
    let playing = || {
        let app = show(Some(CueId(1)));
        let workspace = app.workspace.apply(loaded(1, "sound.ogg", sound(10.0)));
        App { workspace, ..app }.handled(Message::Go).0
    };
    assert!(!playing().workspace.active().is_empty());
    let open = App {
        help: true,
        ..playing()
    };
    let (typed, task) = open.typed(control_c(), false);
    assert!(typed.help);
    assert_eq!(task.units(), 0);
    assert!(!typed.workspace.active().is_empty());
    let (cancelled, _) = playing().handled(Message::Cancel);
    assert!(!cancelled.workspace.active().is_empty());
}

#[test]
fn cancel_takes_the_keyboard_out_of_a_field_with_a_task() {
    let (_, task) = show(Some(CueId(2))).handled(Message::Cancel);
    assert_eq!(task.units(), 1);
}
