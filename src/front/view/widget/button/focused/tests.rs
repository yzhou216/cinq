// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the keys that press a button that Tab reached

use iced::keyboard::{Key, key::Named};

use super::*;
use crate::back::PatchEdit;

fn button(leaves: bool, keymap: &Keymap) -> Focused<'_> {
    Focused::new(
        iced::widget::text("Add instrument").into(),
        Message::Patch(PatchEdit::Add),
        leaves,
        keymap,
    )
}

fn enter() -> Key {
    Key::Named(Named::Enter)
}

/// Types `key` in a button that has the keyboard or not, and returns whether
/// it took the key, the messages it sent, and whether it kept the event from
/// the widgets after it
fn typing(
    leaves: bool,
    focused: bool,
    key: &Key,
    at_once: bool,
    repeat: bool,
) -> (bool, Vec<Message>, bool) {
    let keymap = Keymap::default();
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let taken = button(leaves, &keymap).took(focused, key, at_once, repeat, &mut shell);
    let captured = shell.is_event_captured();
    (taken, messages, captured)
}

/// Like [`typing`], for a button that keeps the keyboard, counting messages
fn pressing(focused: bool, key: &Key, at_once: bool, repeat: bool) -> (bool, usize, bool) {
    let (taken, messages, captured) = typing(false, focused, key, at_once, repeat);
    (taken, messages.len(), captured)
}

#[test]
fn enter_presses_the_button_that_has_the_keyboard_and_keeps_the_key_from_the_bindings() {
    assert_eq!(pressing(true, &enter(), false, false), (true, 1, true));
}

#[test]
fn a_held_enter_presses_the_button_once() {
    assert_eq!(pressing(true, &enter(), false, true), (true, 0, true));
}

#[test]
fn a_button_without_the_keyboard_takes_no_key() {
    assert_eq!(pressing(false, &enter(), false, false), (false, 0, false));
}

#[test]
fn space_and_escape_are_left_to_go_and_panic() {
    [Named::Space, Named::Escape, Named::Tab, Named::ArrowDown]
        .into_iter()
        .for_each(|named| {
            let key = Key::Named(named);
            assert_eq!(pressing(true, &key, false, false), (false, 0, false));
        });
}

#[test]
fn a_key_that_acts_at_once_is_not_taken_even_when_it_is_enter() {
    assert_eq!(pressing(true, &enter(), true, false), (false, 0, false));
}

#[test]
fn a_button_that_leaves_gives_the_keyboard_back_to_the_cues_once_it_acts() {
    let (taken, messages, _) = typing(true, true, &enter(), false, false);
    assert_eq!(
        (
            taken,
            messages.len(),
            matches!(messages.last(), Some(Message::Focus(Focus::Cues)))
        ),
        (true, 2, true)
    );
    // A held key repeats without acting, so that it gives nothing back
    assert!(typing(true, true, &enter(), false, true).1.is_empty());
    // A button that does not leave keeps the keyboard
    assert_eq!(typing(false, true, &enter(), false, false).1.len(), 1);
}
