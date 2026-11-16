// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the edits of one field that undo takes back one visit at a time:
//! keys typed in a row make one change, until the keyboard leaves the field or
//! enters it again

use super::{after, history::cue, show, typing};
use crate::{
    back::{CueId, Edit},
    front::{App, Message},
};

#[test]
fn two_visits_to_the_name_of_a_cue_undo_as_two_steps() {
    let name = |app: &App| app.workspace.cue(CueId(3)).map(|cue| cue.name.clone());
    let type_into = |app: App, text: &str| {
        (1..=text.len()).fold(app, |app, end| {
            let edit = Edit::Name(text[..end].to_owned());
            after(app, [Message::Edit(CueId(3), edit), Message::Held(true)])
        })
    };
    let first = type_into(show(cue(3)), "Rain");
    let second = type_into(after(first, [Message::Held(false)]), "Rain on the roof");
    let once = after(second, [Message::Undo(1)]);
    assert_eq!(name(&once).as_deref(), Some("Rain"));
    let twice = after(once, [Message::Undo(1)]);
    assert_eq!(name(&twice).as_deref(), Some("Sound"));
}

#[test]
fn two_visits_to_the_name_of_a_cue_by_tab_undo_as_two_steps() {
    let name = |app: &App| app.workspace.cue(CueId(3)).map(|cue| cue.name.clone());
    let type_into = |app: App, text: &str| {
        (1..=text.len()).fold(app, |app, end| {
            let edit = Edit::Name(text[..end].to_owned());
            after(app, [Message::Edit(CueId(3), edit), Message::Held(true)])
        })
    };
    let first = type_into(show(cue(3)), "Rain");
    let away = typing(typing(first, "Tab"), "Shift+Tab");
    let second = type_into(away, "Rain on the roof");
    let once = after(second, [Message::Undo(1)]);
    assert_eq!(name(&once).as_deref(), Some("Rain"));
}
