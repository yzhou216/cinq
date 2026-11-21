// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hint that follows a cue just added

use unicode_width::UnicodeWidthStr;

use super::{YIELDING, fitted, hinted, opened, owned};
use crate::{
    back::{Command, Kind, Place, Workspace},
    front::{
        App,
        keymap::{Action, Preset},
    },
};

/// Returns the first two hints, which tell the latest change and then
/// the key that renames the cue, after `command`
fn told(preset: Preset, command: Command, text_held: bool) -> Vec<String> {
    let workspace = Workspace::default();
    let app = App {
        workspace: workspace.apply(command),
        text_held,
        ..opened(preset, false, false)
    };
    hinted(&app)
        .into_iter()
        .map(|(_, hint)| hint)
        .take(2)
        .collect()
}

fn add() -> Command {
    Command::Add {
        list: 0,
        kind: Kind::Audio,
        at: Place::End,
        count: 1,
    }
}

#[test]
fn a_cue_just_added_hints_the_key_that_renames_it_after_the_key_of_undo() {
    assert_eq!(
        told(Preset::Vi, add(), false),
        ["Added cue 1 | u: undo", "cc: rename"]
    );
    assert_eq!(
        told(Preset::Cua, add(), false),
        ["Added cue 1 | Control+z: undo", "F2: rename"]
    );
}

#[test]
fn a_text_field_that_keeps_every_key_hints_neither_undo_nor_rename() {
    assert_eq!(told(Preset::Vi, add(), true)[0], "Added cue 1");
    assert_eq!(told(Preset::Cua, add(), true)[0], "Added cue 1");
    assert!(
        told(Preset::Vi, add(), true)
            .iter()
            .all(|hint| !hint.contains("rename"))
    );
}

#[test]
fn the_show_mode_that_locks_the_name_hints_no_rename() {
    let locked = Workspace::default()
        .apply(add())
        .apply(Command::ShowMode(true));
    let app = App {
        workspace: locked,
        ..opened(Preset::Vi, false, false)
    };
    assert!(
        hinted(&app)
            .iter()
            .all(|(_, hint)| !hint.contains("rename"))
    );
}

#[test]
fn the_outputs_hint_the_rename_of_an_output_and_not_that_of_the_cue_just_added() {
    let app = App {
        workspace: Workspace::default().apply(add()),
        outputs_shown: true,
        ..opened(Preset::Vi, false, false)
    };
    let hints: Vec<_> = hinted(&app).into_iter().map(|(_, hint)| hint).collect();
    assert!(hints.contains(&"i: rename".to_owned()), "{hints:?}");
    assert!(!hints.contains(&"cc: rename".to_owned()), "{hints:?}");
}

#[test]
fn go_panic_and_save_give_way_before_the_rename_of_a_cue_just_added() {
    let hints = owned([
        (None, "Added cue 1 | u: undo"),
        (Some(Action::ChangeName), "cc: rename"),
        (Some(Action::Save), "Control+s: save"),
        (Some(Action::Go), "Space: GO"),
        (Some(Action::Panic), "Escape: Panic"),
        (Some(Action::Help), "?: help"),
    ]);
    let kept = "Added cue 1 | u: undo | cc: rename | ?: help";
    assert_eq!(fitted(&hints, kept.width(), &YIELDING), kept);
    assert_eq!(
        fitted(&hints, kept.width() - 1, &YIELDING),
        "Added cue 1 | u: undo | ?: help"
    );
    let with_save = "Added cue 1 | u: undo | cc: rename | Control+s: save | ?: help";
    assert_eq!(fitted(&hints, with_save.width(), &YIELDING), with_save);
}
