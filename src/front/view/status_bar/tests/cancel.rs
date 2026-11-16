// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hint of the key that cancels what is typed

use iced::keyboard::{Key, Modifiers};
use unicode_width::UnicodeWidthStr;

use super::{super::status, fitted, hinted, opened};
use crate::front::{
    App,
    keymap::{Preset, Press},
};

/// Columns of a bar 1024 pixels wide in DejaVu Sans Mono at 16 pixels.
/// The test counts them rather than measuring the font, since the
/// monospace font that a build finds differs from one system to another.
pub(super) const COLUMNS: usize = 103;

/// Returns the hint of the key of cancel, if the bar shows one
fn cancel_hint(app: &App) -> Option<String> {
    hinted(app)
        .into_iter()
        .map(|(_, hint)| hint)
        .find(|hint| hint.ends_with(": cancel"))
}

fn press(key: &str) -> Option<Press> {
    Press::from_key(&Key::Character(key.into()), Modifiers::empty())
}

#[test]
fn the_bar_hints_the_key_of_cancel_while_keys_wait() {
    let d = Press::from_key(&Key::Character("d".into()), Modifiers::empty()).expect("a key");
    let waiting = |preset| App {
        keys: vec![d],
        ..opened(preset)
    };
    assert_eq!(cancel_hint(&opened(Preset::Vi)), None);
    assert_eq!(
        cancel_hint(&waiting(Preset::Vi)).as_deref(),
        Some("Control+c: cancel")
    );
    // Cua binds no key to it
    assert_eq!(cancel_hint(&waiting(Preset::Cua)), None);
}

#[test]
fn the_key_of_cancel_stays_in_a_bar_of_1024_pixels_while_go_and_panic_give_way() {
    let d = Press::from_key(&Key::Character("d".into()), Modifiers::empty()).expect("a key");
    let shown = |told: &str| {
        let app = App {
            keys: vec![d],
            told: Some(told.to_owned()),
            ..opened(Preset::Vi)
        };
        let taken: usize = status(&app).iter().map(|part| part.text.width()).sum();
        fitted(&hinted(&app), COLUMNS - taken)
    };
    // GO gives way first, then Panic, while cancel stays
    let short = shown("Saved the show");
    assert!(short.contains("Control+c: cancel") && short.contains("Escape: Panic"));
    assert!(!short.contains("GO"));
    let long = shown("Opened a show whose name holds the words that were typed");
    assert!(long.contains("Control+c: cancel"));
    assert!(!long.contains("GO") && !long.contains("Panic"));
}

#[test]
fn the_bar_shows_keys_typed_so_far_joined_without_the_space_that_is_go() {
    let app = App {
        keys: ["2", "d"].iter().filter_map(|key| press(key)).collect(),
        ..opened(Preset::Vi)
    };
    assert_eq!(hinted(&app)[0].1, "2d");
}
