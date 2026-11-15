// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hints that fit in the status bar

mod cancel;
mod places;

use super::{clipped, fitted, hints as hinted};
use crate::{
    back::{Command, Cue, CueId, CueList, Kind, Workspace},
    front::{
        App,
        keymap::{Action, Keymap, Preset, config::Config},
        update::Clipped,
    },
};

fn owned<const N: usize>(hints: [(Option<Action>, &str); N]) -> Vec<(Option<Action>, String)> {
    hints
        .into_iter()
        .map(|(action, hint)| (action, hint.to_owned()))
        .collect()
}

/// Returns the hints after a change, with unsaved changes
fn hints() -> Vec<(Option<Action>, String)> {
    owned([
        (None, "Added cue 1 | u: undo"),
        (Some(Action::Save), "Control+s: save"),
        (Some(Action::Go), "Space: GO"),
        (Some(Action::Panic), "Escape: Panic"),
        (Some(Action::Help), "?: help"),
    ])
}

#[test]
fn hints_that_fit_show_in_full() {
    assert_eq!(
        fitted(&hints(), 77),
        "Added cue 1 | u: undo | Control+s: save | Space: GO | Escape: Panic | ?: help"
    );
}

#[test]
fn keys_give_way_in_turn_while_what_the_change_told_stays() {
    assert_eq!(
        fitted(&hints(), 76),
        "Added cue 1 | u: undo | Control+s: save | Escape: Panic | ?: help"
    );
    assert_eq!(fitted(&hints(), 31), "Added cue 1 | u: undo | ?: help");
    assert_eq!(fitted(&hints(), 5), "Added cue 1 | u: undo");
}

#[test]
fn keys_of_going_forward_and_back_give_way_before_the_rest() {
    let places = owned([
        (Some(Action::Back), "Control+o: back"),
        (Some(Action::Forward), "Control+i: forward"),
    ]);
    let hints: Vec<_> = hints()
        .into_iter()
        .take(1)
        .chain(places)
        .chain(hints().into_iter().skip(2))
        .collect();
    let all = "Added cue 1 | u: undo | Control+o: back | Control+i: forward | Space: GO | Escape: Panic | ?: help";
    assert_eq!(fitted(&hints, 100), all);
    assert_eq!(
        fitted(&hints, 80),
        "Added cue 1 | u: undo | Control+o: back | Space: GO | Escape: Panic | ?: help"
    );
    assert_eq!(
        fitted(&hints, 62),
        "Added cue 1 | u: undo | Space: GO | Escape: Panic | ?: help"
    );
}

fn opened(preset: Preset) -> App {
    let mut config = Config::default();
    config.keymap = Keymap::from(preset);
    App {
        config,
        ..App::default()
    }
}

#[test]
fn a_cut_names_the_keys_that_paste_and_undo_while_a_copy_names_only_the_paste_key() {
    let shown = |told: &str, cut: bool| App {
        clipped: Some(Clipped {
            told: told.to_owned(),
            cut,
        }),
        workspace: Workspace::new(vec![CueList::new(
            "Main",
            vec![Cue::new(CueId(1), "1", Kind::Memo, "Note").build()],
        )])
        .apply(Command::Delete(vec![CueId(1)])),
        ..App::default()
    };
    assert_eq!(
        clipped(&shown("Cut cue 4", true)).as_deref(),
        Some("Cut cue 4 | p: paste | u: undo")
    );
    assert_eq!(
        clipped(&shown("Copied 3 cues", false)).as_deref(),
        Some("Copied 3 cues | p: paste")
    );
    // Show mode locks pasting, so no key is offered
    let locked = App {
        workspace: Workspace::default().apply(Command::ShowMode(true)),
        ..shown("Copied 3 cues", false)
    };
    assert_eq!(clipped(&locked).as_deref(), Some("Copied 3 cues"));
}

#[test]
fn what_a_key_remarked_shows_in_the_bar_before_anything_else() {
    let app = App {
        remark: Some("The clipboard is empty"),
        ..App::default()
    };
    let told = hinted(&app)
        .into_iter()
        .find_map(|(action, hint)| action.is_none().then_some(hint));
    assert_eq!(told.as_deref(), Some("The clipboard is empty"));
}
