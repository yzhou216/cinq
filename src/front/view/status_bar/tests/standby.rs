// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hint of the key that moves standby to the selected cue

use super::{YIELDING, fitted, hinted, opened};
use crate::{
    back::{Command, Cue, CueId, CueList, Kind, Workspace},
    front::{App, keymap::Preset},
};

/// Returns an app of two cues with standby on the first and `selected` selected
fn with(selected: Option<u32>) -> App {
    let memo = |id: u32| Cue::new(CueId(id), id.to_string(), Kind::Memo, "Note").build();
    App {
        workspace: Workspace::new(vec![CueList::new("Main", vec![memo(1), memo(2)])]).apply(
            Command::Standby {
                list: 0,
                cue: Some(CueId(1)),
            },
        ),
        selected: selected.map(CueId),
        ..opened(Preset::Vi, false, false)
    }
}

fn told(app: &App) -> bool {
    hinted(app).iter().any(|(_, hint)| hint == "Enter: standby")
}

#[test]
fn enter_is_hinted_to_move_standby_while_the_selection_is_elsewhere() {
    assert!(told(&with(Some(2))));
    assert!(!told(&with(Some(1))), "the cue is the standby cue already");
    assert!(!told(&with(None)), "nothing is selected");
}

#[test]
fn no_field_list_view_or_page_that_has_the_keys_hints_standby() {
    let app = |change: fn(App) -> App| change(with(Some(2)));
    assert!(!told(&app(|app| App { held: true, ..app })));
    assert!(!told(&app(|app| App { help: true, ..app })));
    assert!(!told(&app(|app| App { menu: true, ..app })));
    assert!(!told(&app(|app| App {
        outputs_shown: true,
        ..app
    })));
}

#[test]
fn the_hint_of_standby_outlasts_the_hints_of_the_next_list_the_next_tab_and_hint_mode() {
    let app = with(Some(2));
    let all = fitted(&hinted(&app), 100, &YIELDING);
    assert_eq!(
        all,
        "Space: GO | Escape: Panic | ?: help | Enter: standby | gt: next tab | l: next list | f: hint"
    );
    let without = |keys: &[&str]| {
        let gone: usize = keys.iter().map(|key| key.len() + 3).sum();
        fitted(&hinted(&app), all.len() - gone, &YIELDING)
    };
    assert_eq!(
        without(&["l: next list"]),
        "Space: GO | Escape: Panic | ?: help | Enter: standby | gt: next tab | f: hint"
    );
    assert_eq!(
        without(&["l: next list", "gt: next tab"]),
        "Space: GO | Escape: Panic | ?: help | Enter: standby | f: hint"
    );
    assert_eq!(
        without(&["l: next list", "gt: next tab", "f: hint"]),
        "Space: GO | Escape: Panic | ?: help | Enter: standby"
    );
    assert_eq!(
        fitted(&hinted(&app), 40, &YIELDING),
        "Space: GO | Escape: Panic | ?: help"
    );
}
