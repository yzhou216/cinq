// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hints while a drop-down list or a checkbox has the keyboard

use super::{clipped, hinted, opened};
use crate::{
    back::{Command, Cue, CueId, CueList, Kind, Place, Workspace},
    front::{
        App,
        keymap::{Action, Preset},
        update::Clipped,
    },
};

/// Returns all the hints of `app`
fn all(app: &App) -> Vec<String> {
    hinted(app).into_iter().map(|(_, hint)| hint).collect()
}

/// Returns an app in which a list has the keyboard, or a text field does
fn held(preset: Preset, text: bool) -> App {
    App {
        held: true,
        text_held: text,
        ..opened(preset, false, false)
    }
}

fn added() -> Workspace {
    Workspace::default().apply(Command::Add {
        list: 0,
        kind: Kind::Audio,
        at: Place::End,
        count: 1,
    })
}

#[test]
fn a_list_hints_undo_but_not_the_rename_of_the_cue_just_added() {
    let list = App {
        workspace: added(),
        ..held(Preset::Vi, false)
    };
    assert_eq!(all(&list)[0], "Added cue 1 | u: undo");
    assert!(all(&list).iter().all(|hint| !hint.contains("rename")));
}

#[test]
fn a_list_hints_neither_the_keys_to_add_and_rename_an_output_nor_paste() {
    let outputs = App {
        outputs_shown: true,
        ..held(Preset::Vi, false)
    };
    assert!(all(&outputs).iter().all(|hint| !hint.contains("output")));
    assert!(all(&outputs).iter().all(|hint| !hint.contains("rename")));
    let cut = |held: App| App {
        clipped: Some(Clipped {
            told: "Cut cue 1".to_owned(),
            cut: true,
        }),
        workspace: Workspace::new(vec![CueList::new(
            "Main",
            vec![Cue::new(CueId(1), "1", Kind::Memo, "Note").build()],
        )])
        .apply(Command::Delete(vec![CueId(1)])),
        ..held
    };
    // Undo acts in a list, where paste does not
    assert_eq!(
        clipped(&cut(held(Preset::Vi, false))).as_deref(),
        Some("Cut cue 1 | u: undo")
    );
    assert_eq!(
        clipped(&cut(held(Preset::Vi, true))).as_deref(),
        Some("Cut cue 1")
    );
}

#[test]
fn the_show_mode_check_box_hints_the_next_field_go_and_panic_in_show_mode_too() {
    [Preset::Vi, Preset::Cua].into_iter().for_each(|preset| {
        let app = App {
            workspace: added().apply(Command::ShowMode(true)),
            ..held(preset, false)
        };
        let hints = all(&app);
        assert!(
            hints.iter().any(|hint| hint.contains("next field")),
            "{hints:?}"
        );
        assert!(hints.iter().any(|hint| hint.contains("GO")), "{hints:?}");
        assert!(hints.iter().any(|hint| hint.contains("Panic")), "{hints:?}");
        assert!(
            hints
                .iter()
                .all(|hint| !hint.contains("rename") && !hint.contains("standby")),
            "{hints:?}"
        );
    });
}

/// Returns the hint that tells what Enter does in a field of `app`, if any,
/// which is not the hint of a key that a user binds to Enter
fn enter(app: &App) -> Option<String> {
    all(app).into_iter().find(|hint| {
        matches!(
            hint.as_str(),
            "Enter: flip" | "Enter: press" | "Enter: open" | "Enter: leave"
        )
    })
}

#[test]
fn enter_is_hinted_to_flip_a_check_box_and_to_open_a_list() {
    for preset in [Preset::Vi, Preset::Cua] {
        let check = App {
            check_held: true,
            ..held(preset, false)
        };
        assert_eq!(enter(&check).as_deref(), Some("Enter: flip"));
        assert_eq!(enter(&held(preset, false)).as_deref(), Some("Enter: open"));
    }
}

#[test]
fn enter_is_hinted_in_no_text_field_and_where_no_field_has_the_keyboard() {
    // Enter leaves a text field in cua, where its own hint says so
    assert_eq!(
        enter(&held(Preset::Cua, true)).as_deref(),
        Some("Enter: leave")
    );
    assert_eq!(enter(&held(Preset::Vi, true)), None);
    assert_eq!(enter(&opened(Preset::Cua, false, false)), None);
    let check = |change: fn(App) -> App| {
        change(App {
            check_held: true,
            ..held(Preset::Vi, false)
        })
    };
    assert_eq!(enter(&check(|app| App { menu: true, ..app })), None);
    assert_eq!(enter(&check(|app| App { help: true, ..app })), None);
}

#[test]
fn enter_is_not_hinted_to_open_or_flip_where_a_key_binding_takes_it_over() {
    let mut app = App {
        check_held: true,
        ..held(Preset::Cua, false)
    };
    let key = vec![crate::front::keymap::sequence("Enter").expect("a key")];
    app.config.keymap = app.config.keymap.clone().with(Action::Go, key);
    assert_eq!(enter(&app), None);
}

#[test]
fn enter_is_hinted_to_press_a_button_and_not_to_open_it() {
    for preset in [Preset::Vi, Preset::Cua] {
        let button = App {
            button_held: true,
            ..held(preset, false)
        };
        assert_eq!(enter(&button).as_deref(), Some("Enter: press"));
        assert!(all(&button).iter().all(|hint| hint != "Enter: open"));
    }
}
