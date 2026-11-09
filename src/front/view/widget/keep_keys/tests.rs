// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of which keys that a field captured still act

use std::fs;

use iced::{
    Event,
    keyboard::{
        self, Key, Location, Modifiers,
        key::{Named, NativeCode, Physical},
    },
};
use tempfile::TempDir;

use super::left_to_act;
use crate::front::{
    keymap::{Keymap, config},
    view::widget::key_pressed,
};

/// Keymap that binds GO and Panic to keys that iced's text input takes for
/// the clipboard and the selection, and GO to `w`, which it types, besides
/// the defaults
fn clipboard_keys() -> Keymap {
    let directory = TempDir::new().expect("a directory can be made");
    let file = directory.path().join("config.toml");
    fs::write(
        &file,
        "[keys]\ngo = [\"Space\", \"Control+c\", \"Control+v\", \"Control+a\", \"w\"]\n\
         panic = [\"Escape\", \"Control+x\"]\n",
    )
    .expect("a file can be written");
    let config = config::load(Some(&file));
    assert!(config.problems.is_empty(), "{:?}", config.problems);
    config.keymap
}

/// Returns whether the key still acts after a field captured it
fn acts(keymap: &Keymap, key: Key, text: Option<&str>, modifiers: Modifiers) -> bool {
    let event = Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: Physical::Unidentified(NativeCode::Unidentified),
        location: Location::Standard,
        modifiers,
        text: text.map(Into::into),
        repeat: false,
    });
    let pressed = key_pressed(&event).expect("the event is a key press");
    left_to_act(keymap, &pressed).is_some()
}

fn letter(text: &str) -> Key {
    Key::Character(text.into())
}

#[test]
fn clipboard_and_selection_keys_act_though_the_field_takes_them() {
    let keymap = clipboard_keys();
    let cases = [
        ("c", "\u{3}"),
        ("x", "\u{18}"),
        ("v", "\u{16}"),
        ("a", "\u{1}"),
    ];
    let silent: Vec<_> = cases
        .into_iter()
        .filter(|&(key, text)| !acts(&keymap, letter(key), Some(text), Modifiers::CTRL))
        .collect();
    assert!(silent.is_empty(), "these keys did not act: {silent:?}");
}

/// A system that reports the letter itself as the text of a Control chord
/// still leaves the field nothing typed; on Darwin the command modifier is
/// the logo key, which key bindings never take
#[cfg(not(target_os = "macos"))]
#[test]
fn control_chord_acts_whatever_text_the_system_reports() {
    assert!(acts(
        &clipboard_keys(),
        letter("c"),
        Some("c"),
        Modifiers::CTRL
    ));
}

#[test]
fn space_acts_only_when_it_is_not_typed() {
    let keymap = Keymap::default();
    let space = || Key::Named(Named::Space);
    assert!(!acts(&keymap, space(), Some(" "), Modifiers::empty()));
    assert!(!acts(&keymap, space(), Some(" "), Modifiers::SHIFT));
    assert!(acts(&keymap, space(), Some("\u{0}"), Modifiers::CTRL));
}

#[test]
fn control_chord_that_the_field_types_does_not_act() {
    let keymap = clipboard_keys();
    assert!(!acts(&keymap, letter("w"), Some("w"), Modifiers::CTRL));
    assert!(!acts(&keymap, letter("w"), Some("w"), Modifiers::empty()));
}

#[test]
fn escape_acts_though_the_field_takes_it_to_give_up_the_keyboard() {
    let escape = Key::Named(Named::Escape);
    assert!(acts(
        &Keymap::default(),
        escape,
        Some("\u{1b}"),
        Modifiers::empty()
    ));
}
