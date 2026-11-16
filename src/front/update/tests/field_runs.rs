// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the edits of one field that undo takes back one visit at a time:
//! keys typed in a row make one change, until the keyboard enters the field
//! again

use super::{after, history::cue, show};
use crate::{
    back::{CueId, Edit},
    front::{App, Caret, Focus, Message},
};

/// Messages that tell that the keyboard came into the field again
fn between() -> [Vec<Message>; 3] {
    [
        vec![Message::Focus(Focus::Name(Caret::End))],
        vec![Message::Focus(Focus::Next)],
        vec![Message::Clicked],
    ]
}

/// Types `text` into the name of the cue 3, one key at a time
fn type_into(app: App, text: &str) -> App {
    (1..=text.len()).fold(app, |app, end| {
        let edit = Edit::Name(text[..end].to_owned());
        after(app, [Message::Edit(CueId(3), edit)])
    })
}

fn name(app: &App) -> Option<String> {
    app.workspace.cue(CueId(3)).map(|cue| cue.name.clone())
}

#[test]
fn two_visits_to_the_name_of_a_cue_undo_as_two_steps() {
    between().into_iter().for_each(|entering| {
        let first = type_into(show(cue(3)), "Rain");
        let second = type_into(after(first, entering.clone()), "Rain on the roof");
        let once = after(second, [Message::Undo(1)]);
        assert_eq!(name(&once).as_deref(), Some("Rain"), "{entering:?}");
        let twice = after(once, [Message::Undo(1)]);
        assert_eq!(name(&twice).as_deref(), Some("Sound"), "{entering:?}");
    });
}

#[test]
fn keys_typed_in_one_visit_undo_as_one_step() {
    let typed = type_into(show(cue(3)), "Rain on the roof");
    assert_eq!(name(&typed).as_deref(), Some("Rain on the roof"));
    assert_eq!(
        name(&after(typed, [Message::Undo(1)])).as_deref(),
        Some("Sound")
    );
}
