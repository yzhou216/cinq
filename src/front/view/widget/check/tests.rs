// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the keys that flip a checkbox and of the keys that none may take

use iced::keyboard::{Key, Modifiers, key::Named};
use strum::VariantArray;

use super::*;
use crate::{
    back::{CueId, Edit},
    front::keymap::{Action, Preset, Press},
};

fn checkbox(keymap: &Keymap, changes: bool) -> Check<'_> {
    Check {
        look: iced::widget::checkbox(false).label("Loop").into(),
        flip: changes.then_some(Message::Edit(CueId(1), Edit::Loop(true))),
        keymap,
    }
}

fn enter() -> Key {
    Key::Named(Named::Enter)
}

/// Types `key` in a checkbox of the default keymap that changes or not and has
/// the keyboard or not, and returns whether it took the key, how many messages
/// it sent, and whether it kept the event from the widgets after it
fn pressing(focused: bool, changes: bool, key: &Key, repeat: bool) -> (bool, usize, bool) {
    let keymap = Keymap::default();
    let at_once =
        Press::from_key(key, Modifiers::empty()).is_some_and(|press| keymap.acts_at_once(press));
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let taken = checkbox(&keymap, changes).took(focused, key, at_once, repeat, &mut shell);
    let captured = shell.is_event_captured();
    (taken, messages.len(), captured)
}

#[test]
fn enter_flips_the_checkbox_that_has_the_keyboard_and_keeps_the_key_from_the_bindings() {
    assert_eq!(pressing(true, true, &enter(), false), (true, 1, true));
}

#[test]
fn enter_is_the_key_of_standby_and_no_key_that_acts_at_once_in_both_keymaps() {
    Preset::VARIANTS.iter().for_each(|&preset| {
        let keymap = Keymap::from(preset);
        let press = Press::from_key(&enter(), Modifiers::empty()).expect("a key");
        assert_eq!(keymap.alone(press), Some(Action::Standby));
        assert!(!keymap.acts_at_once(press));
    });
}

#[test]
fn a_held_enter_flips_the_checkbox_once_and_stays_from_the_bindings() {
    assert_eq!(pressing(true, true, &enter(), true), (true, 0, true));
}

#[test]
fn space_and_escape_never_flip_the_checkbox_and_are_left_to_go_and_panic() {
    [Named::Space, Named::Escape, Named::Tab, Named::ArrowDown]
        .into_iter()
        .for_each(|named| {
            let key = Key::Named(named);
            assert_eq!(pressing(true, true, &key, false), (false, 0, false));
        });
    let letter = Key::Character("x".into());
    assert_eq!(pressing(true, true, &letter, false), (false, 0, false));
}

#[test]
fn a_checkbox_without_the_keyboard_takes_no_key() {
    assert_eq!(pressing(false, true, &enter(), false), (false, 0, false));
}

#[test]
fn enter_changes_nothing_in_a_checkbox_that_cannot_change() {
    assert_eq!(pressing(true, false, &enter(), false), (true, 0, true));
}

#[test]
fn a_key_that_acts_at_once_is_not_taken_even_when_it_is_enter() {
    let keymap = Keymap::default();
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let taken = checkbox(&keymap, true).took(true, &enter(), true, false, &mut shell);
    assert!(!taken && !shell.is_event_captured());
    assert!(messages.is_empty());
}

#[test]
fn the_state_takes_and_gives_up_the_keyboard() {
    let mut state = State { focused: false };
    assert!(!state.is_focused());
    state.focus();
    assert!(state.is_focused());
    state.unfocus();
    assert!(!state.is_focused());
}
