// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of copying, cutting, and pasting cues

mod targets;

pub(in crate::front::update::tests) use targets::{fading, targets};

use super::{operators::rows, *};
use crate::{
    back::{Command, Cue, InstrumentId, Kind, OutputEdit, OutputId, Sends},
    front::update::clipboard::Clipboard,
};

#[test]
fn copied_cue_pastes_below_the_selection_and_becomes_the_selection() {
    let app = after(
        show(Some(CueId(4))),
        [Message::Copy(1), Message::Paste(Place::After, 1)],
    );
    assert_eq!(rows(&app)[..6], [1, 2, 3, 4, 22, 5]);
    assert_eq!(app.selected, Some(CueId(22)));
    let (copy, original) = (app.workspace.cue(CueId(22)), app.workspace.cue(CueId(4)));
    let settings = |cue: Option<&Cue>| cue.map(|cue| (cue.name.clone(), cue.file.clone()));
    assert_eq!(settings(copy), settings(original));
}

#[test]
fn copy_with_a_count_takes_the_cues_below_and_paste_above_keeps_their_order() {
    let app = after(
        show(Some(CueId(3))),
        [
            Message::Copy(2),
            Message::Select(CueId(8)),
            Message::Paste(Place::Before, 1),
        ],
    );
    assert_eq!(rows(&app)[5..11], [6, 7, 22, 23, 8, 9]);
    assert_eq!(app.selected, Some(CueId(22)));
}

#[test]
fn paste_with_a_count_pastes_that_many_times_in_one_change() {
    let app = after(
        show(Some(CueId(4))),
        [Message::Copy(2), Message::Paste(Place::After, 3)],
    );
    assert_eq!(rows(&app).len(), 12 + 6);
    let undone = after(app, [Message::Undo(1)]);
    assert_eq!(rows(&undone).len(), 12);
}

#[test]
fn cut_then_paste_moves_a_cue_and_keeps_its_number() {
    let app = after(show(Some(CueId(2))), [Message::Cut(1)]);
    assert_eq!(rows(&app)[..3], [1, 3, 4]);
    assert_eq!(app.selected, Some(CueId(3)));
    let moved = after(
        app,
        [Message::Select(CueId(5)), Message::Paste(Place::After, 1)],
    );
    assert_eq!(rows(&moved)[..6], [1, 3, 4, 5, 22, 6]);
    assert_eq!(moved.workspace.cue(CueId(22)).unwrap().number, "2");
}

#[test]
fn delete_leaves_the_clipboard_alone_and_undo_does_not_touch_it() {
    let app = after(
        show(Some(CueId(4))),
        [
            Message::Copy(1),
            Message::Select(CueId(6)),
            Message::Delete(1),
            Message::Undo(1),
            Message::Paste(Place::After, 1),
            Message::Undo(1),
            Message::Paste(Place::After, 1),
        ],
    );
    // Undoing the first paste returns the selection to cue 6, which it was
    // made below, so that the second paste goes there as well
    assert_eq!(rows(&app)[4..8], [5, 6, 23, 7]);
}

#[test]
fn pasting_with_an_empty_clipboard_changes_nothing_and_says_that_it_is_empty() {
    let app = after(show(Some(CueId(4))), [Message::Paste(Place::After, 1)]);
    assert_eq!(rows(&app).len(), 12);
    assert_eq!(app.selected, Some(CueId(4)));
    assert!(app.workspace.latest_change().is_none());
    assert_eq!(app.told, None);
    assert_eq!(app.remark, Some("The clipboard is empty"));
    // Next key of any action takes the remark away
    let (moved, _) = app.next(Message::Step(1));
    assert_eq!(moved.remark, None);
    // Paste that finds cues says nothing
    let pasted = after(
        show(Some(CueId(4))),
        [Message::Copy(1), Message::Paste(Place::After, 1)],
    );
    assert_eq!(pasted.remark, None);
}

#[test]
fn copying_nothing_leaves_the_clipboard_as_it_was() {
    let app = after(
        show(Some(CueId(4))),
        [Message::Copy(1), Message::SwitchList(1)],
    );
    let app = App {
        selected: None,
        ..app
    };
    let app = after(app, [Message::Copy(1), Message::Paste(Place::After, 1)]);
    assert_eq!(app.workspace.lists()[1].cues.len(), 3);
}

#[test]
fn show_mode_refuses_cutting_and_pasting_but_lets_a_cue_be_copied() {
    let copied = after(show(Some(CueId(4))), [Message::Copy(1)]);
    let locked = after(
        copied,
        [
            Message::ShowMode(true),
            Message::Select(CueId(6)),
            Message::Cut(1),
            Message::Paste(Place::After, 1),
        ],
    );
    assert_eq!(rows(&locked).len(), 12);
    // Cut copied nothing, so that the clipboard still holds cue 4 for the
    // paste that follows, while copy replaces what it held
    let open = after(
        locked,
        [
            Message::Copy(1),
            Message::ShowMode(false),
            Message::Paste(Place::After, 1),
        ],
    );
    assert_eq!(
        open.workspace.cue(CueId(22)).map(|cue| cue.name.clone()),
        open.workspace.cue(CueId(6)).map(|cue| cue.name.clone())
    );
    assert_eq!(rows(&open).len(), 13);
}

#[test]
fn paste_adds_at_most_a_thousand_cues() {
    let app = after(
        show(Some(CueId(4))),
        [Message::Copy(1), Message::Paste(Place::After, usize::MAX)],
    );
    assert_eq!(rows(&app).len(), 12 + 1000);
}

#[test]
fn a_clipboard_of_more_than_a_thousand_cues_pastes_a_thousand() {
    let cues = (1..=1500).map(audio).collect();
    let app = App {
        workspace: Workspace::new(vec![CueList::new("Main", cues)]),
        selected: Some(CueId(1)),
        ..App::default()
    };
    let app = after(app, [Message::Copy(1500), Message::Paste(Place::After, 1)]);
    assert_eq!(rows(&app).len(), 1500 + 1000);
    let app = after(app, [Message::Paste(Place::After, 2)]);
    assert_eq!(rows(&app).len(), 1500 + 2000);
}

/// Returns a show of one empty list with outputs of the `names`, which take
/// identities from one in that order
fn with_outputs(cues: Vec<Cue>, names: &[&str]) -> Workspace {
    names.iter().enumerate().fold(
        Workspace::new(vec![CueList::new("Main", cues)])
            .apply(Command::add_output(1, names.len() - 1)),
        |workspace, (row, name)| {
            let id = workspace.outputs()[row].id;
            workspace.apply(Command::EditOutput(
                id,
                OutputEdit::Name((*name).to_owned()),
            ))
        },
    )
}

/// Returns the cue that a cue sending to House and Sub becomes once it is
/// copied and pasted into another show, one with outputs of the `names`
fn pasted_elsewhere(names: &[&str]) -> Cue {
    pasted_app(names).workspace.lists()[0].cues[0].clone()
}

/// Returns the state after the paste that [`pasted_elsewhere`] makes
fn pasted_app(names: &[&str]) -> App {
    let sends: Sends = [(OutputId(1), -3.0), (OutputId(2), -6.0)].into();
    let cue = Cue::new(CueId(1), "1", Kind::Audio, "Rain")
        .sends(sends)
        .build();
    let copied = after(
        App {
            workspace: with_outputs(vec![cue], &["House", "Sub"]),
            selected: Some(CueId(1)),
            ..App::default()
        },
        [Message::Copy(1)],
    );
    let elsewhere = App {
        workspace: with_outputs(Vec::new(), names),
        clipboard: Clipboard {
            show: copied.show.id().wrapping_add(1),
            ..copied.clipboard
        },
        selected: None,
        ..copied
    };
    after(elsewhere, [Message::Paste(Place::After, 1)])
}

#[test]
fn a_paste_tells_the_sends_that_the_other_show_could_not_take() {
    let told = |names| {
        pasted_app(names)
            .clipped
            .map(|clipped| (clipped.told, clipped.cut))
    };
    assert_eq!(
        told(&["Sub", "Foyer"]),
        Some(("Pasted 1 cue, dropped 1 send".to_owned(), true))
    );
    assert_eq!(
        told(&["Foyer"]),
        Some(("Pasted 1 cue, dropped 2 sends".to_owned(), true))
    );
    assert_eq!(told(&["House", "Sub"]), None);
}

#[test]
fn cue_pasted_in_another_show_sends_to_the_outputs_of_the_same_names() {
    // Sub is there in the other show, first, while House is not
    let sends = pasted_elsewhere(&["Sub", "Foyer"]).sends;
    assert_eq!(sends, Some([(OutputId(1), -6.0)].into()));
}

#[test]
fn cue_pasted_where_no_output_has_its_names_plays_on_the_first_output() {
    assert_eq!(pasted_elsewhere(&["Foyer"]).sends, None);
}

#[test]
fn a_paste_tells_the_light_cues_that_the_other_patch_lacks() {
    let cue = Cue {
        lights: [(InstrumentId(7), 50.0)].into(),
        ..Cue::new(CueId(1), "1", Kind::Light, "").build()
    };
    let copied = after(
        App {
            workspace: Workspace::new(vec![CueList::new("Main", vec![cue])]),
            selected: Some(CueId(1)),
            ..App::default()
        },
        [Message::Copy(1)],
    );
    let elsewhere = App {
        workspace: Workspace::default(),
        clipboard: Clipboard {
            show: copied.show.id().wrapping_add(1),
            ..copied.clipboard
        },
        selected: None,
        ..copied
    };
    let told = after(elsewhere, [Message::Paste(Place::After, 1)])
        .clipped
        .map(|clipped| clipped.told);
    assert_eq!(told, Some("Pasted 1 cue, 1 unpatched".to_owned()));
}
