// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hints of the keys that act where a field, a checkbox, or a
//! drop-down list has the keyboard

use unicode_width::UnicodeWidthStr;

use super::{
    super::{fit::shown, order::YIELDING_IN_FIELD},
    hinted, opened,
};
use crate::{
    back::{Cue, CueId, CueList, Kind, Workspace},
    front::{
        App,
        keymap::{Action, Preset, sequence},
    },
};

#[test]
fn the_help_key_is_hinted_only_where_it_acts() {
    let told = |app: App| hinted(&app).iter().any(|(_, hint)| hint == "?: help");
    let vi = |menu, help| opened(Preset::Vi, menu, help);
    assert!(told(vi(false, false)));
    assert!(!told(App {
        held: true,
        text_held: true,
        ..vi(false, false)
    }));
    assert!(!told(App {
        held: true,
        ..vi(false, false)
    }));
    assert!(told(App {
        held: true,
        ..vi(true, false)
    }));
    assert!(told(App {
        held: true,
        text_held: true,
        ..vi(false, true)
    }));
}

#[test]
fn the_bar_hints_enter_while_the_empty_file_field_has_the_keyboard() {
    let cue = |file: &str| {
        Cue::new(CueId(1), "1", Kind::Audio, "Rain")
            .file(file)
            .build()
    };
    let told = |file: &str, file_held: bool| {
        let app = App {
            held: true,
            file_held,
            selected: Some(CueId(1)),
            workspace: Workspace::new(vec![CueList::new("Main", vec![cue(file)])]),
            ..opened(Preset::Vi, false, false)
        };
        hinted(&app)
            .iter()
            .any(|(_, hint)| hint == "Enter: choose a file")
    };
    assert!(told("", true));
    assert!(!told("rain.flac", true), "a named file needs no chooser");
    assert!(!told("", false), "another field has the keyboard");
}

#[test]
fn a_keymap_with_no_key_to_cancel_hints_enter_to_leave_a_text_field_only() {
    let told = |text_held| {
        let app = App {
            held: true,
            text_held,
            ..opened(Preset::Cua, false, false)
        };
        hinted(&app).iter().any(|(_, hint)| hint == "Enter: leave")
    };
    assert!(told(true));
    assert!(!told(false), "Enter opens the menu of a drop-down list");
}

#[test]
fn the_key_of_the_next_tab_is_hinted_late_while_no_field_has_the_keyboard() {
    let hints = |held| {
        hinted(&App {
            held,
            ..opened(Preset::Vi, false, false)
        })
    };
    assert_eq!(
        hints(false)
            .iter()
            .rev()
            .nth(2)
            .map(|(_, hint)| hint.as_str()),
        Some("gt: next tab")
    );
    assert!(
        hints(true)
            .iter()
            .all(|(_, hint)| !hint.contains("next tab"))
    );
}

#[test]
fn enter_is_not_hinted_to_leave_where_a_key_binding_takes_it_over() {
    let told = |action| {
        let mut app = App {
            held: true,
            text_held: true,
            ..opened(Preset::Cua, false, false)
        };
        let enter = vec![sequence("Enter").expect("a key")];
        app.config.keymap = app.config.keymap.clone().with(action, enter);
        hinted(&app).iter().any(|(_, hint)| hint == "Enter: leave")
    };
    assert!(!told(Action::Go));
    assert!(!told(Action::Panic));
    assert!(told(Action::Pick), "pick acts in a menu and not in a field");
}

#[test]
fn the_key_that_leaves_a_field_outlasts_the_words_of_a_notice_that_is_cut() {
    let told = "Changed the file of cue 13";
    let hints = |told: Option<String>| {
        told.into_iter()
            .map(|told| (None, told))
            .chain([
                (Some(Action::Go), "Space: GO".to_owned()),
                (Some(Action::Cancel), "Control+c: leave".to_owned()),
            ])
            .collect::<Vec<_>>()
    };
    let shown = |outlasting: &[Action], room| {
        shown(
            hints,
            Some(told.to_owned()),
            room,
            &YIELDING_IN_FIELD,
            outlasting,
        )
    };
    // The notice cut at a word keeps the key, where the key would go first otherwise
    let room = "Changed the file… | Control+c: leave".width();
    assert_eq!(
        shown(&[Action::Cancel], room),
        "Changed the file… | Control+c: leave"
    );
    assert!(!shown(&[], room).contains("leave"));
}
