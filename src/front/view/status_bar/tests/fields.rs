// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hints of the keys that act where a field, a checkbox, or a
//! drop-down list has the keyboard

use unicode_width::UnicodeWidthStr;

use super::{
    super::{
        fit::shown,
        hints_in,
        order::{self, YIELDING_IN_FIELD},
    },
    hinted, opened,
};
use crate::{
    back::{Command, Cue, CueId, CueList, Kind, Place, Workspace},
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
fn enter_is_not_hinted_to_leave_the_empty_file_field_where_it_chooses_a_file() {
    let told = |preset, file: &str| {
        let app = App {
            held: true,
            text_held: true,
            file_held: true,
            selected: Some(CueId(1)),
            workspace: Workspace::new(vec![CueList::new(
                "Main",
                vec![
                    Cue::new(CueId(1), "1", Kind::Audio, "Rain")
                        .file(file)
                        .build(),
                ],
            )]),
            ..opened(preset, false, false)
        };
        hinted(&app)
            .into_iter()
            .map(|(_, hint)| hint)
            .collect::<Vec<_>>()
    };
    let empty = told(Preset::Cua, "");
    assert!(empty.contains(&"Enter: choose a file".to_owned()));
    assert!(!empty.contains(&"Enter: leave".to_owned()));
    assert!(told(Preset::Cua, "rain.flac").contains(&"Enter: leave".to_owned()));
}

#[test]
fn tab_outlasts_the_words_of_a_notice_where_no_key_cancels_in_a_checkbox_or_list() {
    let outlasting = |preset, text_held| {
        order::outlasting(&App {
            held: true,
            text_held,
            ..opened(preset, false, false)
        })
        .contains(&Action::NextField)
    };
    assert!(outlasting(Preset::Cua, false));
    assert!(!outlasting(Preset::Cua, true), "Enter leaves a text field");
    assert!(!outlasting(Preset::Vi, false), "the key of cancel leaves");
}

#[test]
fn tab_outlasts_the_words_of_a_notice_in_the_empty_file_field_where_enter_chooses() {
    let outlasting = |file: &str, text_held| {
        order::outlasting(&App {
            held: true,
            text_held,
            file_held: true,
            selected: Some(CueId(1)),
            workspace: Workspace::new(vec![CueList::new(
                "Main",
                vec![
                    Cue::new(CueId(1), "1", Kind::Audio, "Rain")
                        .file(file)
                        .build(),
                ],
            )]),
            ..opened(Preset::Cua, false, false)
        })
        .contains(&Action::NextField)
    };
    assert!(outlasting("", true));
    assert!(!outlasting("rain.flac", true), "Enter leaves a named file");
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
    // The notice cut to the cue keeps the key, where the key would go first otherwise
    let room = "Changed… cue 13 | Control+c: leave".width();
    assert_eq!(
        shown(&[Action::Cancel], room),
        "Changed… cue 13 | Control+c: leave"
    );
    assert!(!shown(&[], room).contains("leave"));
}

#[test]
fn a_notice_that_would_keep_under_three_words_gives_its_room_to_the_keys() {
    let told = "Added cue 12";
    let hints = |told: Option<String>| {
        told.into_iter()
            .map(|told| (None, told))
            .chain([(Some(Action::Cancel), "Control+c: leave".to_owned())])
            .collect::<Vec<_>>()
    };
    let shown = |room| {
        shown(
            hints,
            Some(told.to_owned()),
            room,
            &YIELDING_IN_FIELD,
            &[Action::Cancel],
        )
    };
    assert_eq!(shown(31), "Added cue 12 | Control+c: leave");
    assert_eq!(
        shown(20),
        "Control+c: leave",
        "no stub such as `Added cue…`"
    );
}

#[test]
fn a_notice_that_is_cut_keeps_the_words_that_tell_which_cue_before_those_that_do_not() {
    let hints = |told: Option<String>| {
        told.into_iter()
            .map(|told| (None, told))
            .chain([(Some(Action::Redo), "Control+r: redo".to_owned())])
            .collect::<Vec<_>>()
    };
    let shown =
        |told: &str, room| shown(hints, Some(told.to_owned()), room, &YIELDING_IN_FIELD, &[]);
    let told = "Undid: changed the number of cue 6.5";
    let key = " | Control+r: redo";
    let room = |text: &str| text.width() + key.width();
    assert_eq!(shown(told, room(told)), format!("{told}{key}"));
    // Dropping the words before the last two, as many as the room asks
    assert_eq!(
        shown(told, room("Undid: changed the number… cue 6.5")),
        "Undid: changed the number… cue 6.5 | Control+r: redo"
    );
    assert_eq!(
        shown(told, room("Undid: changed… cue 6.5") + 1),
        "Undid: changed… cue 6.5 | Control+r: redo"
    );
    // Room for a verb alone before the ellipsis still keeps the cue
    assert_eq!(
        shown(told, room("Undid: changed… cue 6.5") - 1),
        "Undid… cue 6.5 | Control+r: redo"
    );
    // Notices that name no cue are cut as before, and so are those of a cue
    // whose words fit whole
    assert_eq!(
        shown("Moved standby past the end", room("Moved standby past…")),
        "Moved standby past… | Control+r: redo"
    );
    assert_eq!(
        shown("Moved cue 13 into a group", room("Moved cue 13 into a…")),
        "Moved cue 13 into a… | Control+r: redo"
    );
}

#[test]
fn what_enter_does_outlasts_the_words_of_a_notice_that_is_cut() {
    for (preset, room) in [(Preset::Vi, 52), (Preset::Cua, 62)] {
        let app = App {
            held: true,
            workspace: Workspace::default().apply(Command::Add {
                list: 0,
                kind: Kind::Memo,
                at: Place::End,
                count: 1,
            }),
            ..opened(preset, false, false)
        };
        let shown = hints_in(&app, room);
        assert!(shown.contains("Enter: open"), "{shown}");
    }
}
