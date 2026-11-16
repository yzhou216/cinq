// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of reaching and leaving the outputs and of the keys and messages there

use super::{
    super::{after, show},
    names,
};
use crate::{
    back::{CueId, OutputId, Place},
    front::{
        App, Message,
        keymap::Action,
        message::{Height, LineKey, Page},
    },
};

#[test]
fn outputs_lie_past_the_last_cue_list_and_back() {
    let shown = after(
        show(Some(CueId(1))),
        [Message::StepList(1), Message::StepList(1)],
    );
    assert!(shown.outputs_shown);
    assert_eq!(shown.output, Some(OutputId(1)));
    let past = after(shown, [Message::StepList(1)]);
    assert!(past.outputs_shown, "nothing lies past the outputs");
    let back = after(past, [Message::StepList(-1)]);
    assert!(!back.outputs_shown);
    assert_eq!(back.list, 1);
}

#[test]
fn keys_of_cues_add_move_and_delete_outputs_while_they_show() {
    let added = after(
        show(None),
        [
            Message::ShowOutputs,
            Message::Add(Place::After, 1),
            Message::Add(Place::Before, 1),
        ],
    );
    assert_eq!(names(&added), ["Main", "Output 2", "Output 1"]);
    assert_eq!(added.output, Some(OutputId(3)), "the new output shows");
    assert_eq!(added.workspace.cues().count(), 14, "no cue is added");
    let moved = after(added, [Message::Move(-1)]);
    assert_eq!(names(&moved), ["Output 2", "Main", "Output 1"]);
    let deleted = after(moved, [Message::Delete(1)]);
    assert_eq!(names(&deleted), ["Main", "Output 1"]);
    assert_eq!(deleted.output, Some(OutputId(1)), "the next output shows");
}

/// Returns what a message for the rows of a cue list could change in it
fn cue_list(app: &App) -> String {
    let lists: Vec<_> = app
        .workspace
        .lists()
        .iter()
        .map(|list| {
            (
                list.standby,
                list.cues.iter().map(|cue| cue.id).collect::<Vec<_>>(),
            )
        })
        .collect();
    format!(
        "{lists:?} {} {:?} {:?} {} {} {:?}",
        app.list,
        app.selected,
        app.tab,
        app.search.is_some(),
        app.clipped.is_some(),
        app.remark,
    )
}

#[test]
fn no_message_for_rows_reaches_the_cue_list_while_the_outputs_show() {
    let rows = [
        Message::Step(1),
        Message::Row(2),
        Message::Page(Page::HalfDown, None),
        Message::Landed {
            list: 0,
            from: Some(CueId(3)),
            row: 5,
        },
        Message::Scroll(2),
        Message::InView(Height::Top, None),
        Message::Reached {
            list: 0,
            from: Some(CueId(3)),
            row: 5,
        },
        Message::Place(Height::Middle, None),
        Message::StandbySelected,
        Message::StepTab(1),
        Message::ShowTab(2),
        Message::Add(Place::After, 2),
        Message::Move(1),
        Message::Copy(2),
        Message::Cut(2),
        Message::Paste(Place::Before, 2),
        Message::Delete(2),
        Message::Across(Action::Delete, Action::Next, None),
        Message::Search { backward: false },
        Message::Searching("a".to_owned()),
        Message::SearchKey(LineKey::DeleteAll),
        Message::SearchAccepted,
        Message::SearchStep(1),
    ];
    let shown = || after(show(Some(CueId(3))), [Message::ShowOutputs]);
    let before = cue_list(&shown());
    rows.into_iter().for_each(|message| {
        assert!(message.acts_on_rows(), "{message:?}");
        let app = shown().handled(message.clone()).0;
        assert!(app.outputs_shown, "{message:?}");
        assert_eq!(cue_list(&app), before, "{message:?}");
    });
}

#[test]
fn showing_the_cue_list_again_remembers_the_place_that_going_back_returns_to() {
    let outputs = after(
        show(Some(CueId(3))),
        [Message::Select(CueId(21)), Message::ShowOutputs],
    );
    let back = after(outputs, [Message::Back(1)]);
    assert!(!back.outputs_shown);
    assert_eq!((back.list, back.selected), (0, Some(CueId(3))));
    let switched = after(
        show(Some(CueId(3))),
        [Message::ShowOutputs, Message::SwitchList(1)],
    );
    assert!(!switched.outputs_shown);
    let back = after(switched, [Message::Back(1)]);
    assert_eq!((back.list, back.selected), (0, Some(CueId(3))));
}

#[test]
fn warning_about_an_output_shows_the_outputs_with_it_selected() {
    let added = after(
        show(None),
        [Message::ShowOutputs, Message::Add(Place::After, 1)],
    );
    let back = after(added, [Message::StepList(-1)]);
    assert!(!back.outputs_shown);
    let inspected = after(back, [Message::InspectOutput(OutputId(2))]);
    assert!(inspected.outputs_shown);
    assert_eq!(inspected.output, Some(OutputId(2)));
}
