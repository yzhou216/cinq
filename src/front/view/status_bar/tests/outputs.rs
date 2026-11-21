// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hints that the outputs view shows

use super::{
    super::{hints::notice, name, yielding},
    cancel::COLUMNS,
    fitted, hinted, opened,
};
use crate::{
    back::{Command, Cue, CueId, CueList, Edit, Kind, OutputEdit, OutputId, Sends, Workspace},
    front::{
        App,
        keymap::{Action, Preset},
        update::Clipped,
        view::format,
    },
};

fn in_outputs(preset: Preset) -> App {
    App {
        outputs_shown: true,
        ..opened(preset, false, false)
    }
}

/// Returns the hints that fit in `room` columns
fn shown(app: &App, room: usize) -> String {
    fitted(&hinted(app), room, yielding(app))
}

#[test]
fn the_outputs_hint_the_way_back_and_the_keys_to_add_and_to_rename_an_output() {
    assert_eq!(
        shown(&in_outputs(Preset::Vi), COLUMNS),
        "o: add output | i: rename | Control+o: back to Main | Space: GO | Escape: Panic | ?: help"
    );
    // Cue list needs none of them, and has nowhere to go back to
    assert_eq!(
        shown(&opened(Preset::Vi, false, false), COLUMNS),
        "Space: GO | Escape: Panic | ?: help | gt: next tab"
    );
}

#[test]
fn show_mode_locks_the_outputs_so_that_no_key_to_edit_them_is_hinted() {
    let locked = App {
        workspace: Workspace::default().apply(Command::ShowMode(true)),
        ..in_outputs(Preset::Vi)
    };
    assert_eq!(
        shown(&locked, COLUMNS),
        "Control+o: back to Main | Space: GO | Escape: Panic | ?: help"
    );
}

#[test]
fn hints_of_the_outputs_give_way_with_the_way_back_lasting_longest_after_help() {
    let app = in_outputs(Preset::Vi);
    let in_turn: Vec<_> = [81, 65, 53, 41, 25, 6]
        .into_iter()
        .map(|room| shown(&app, room))
        .collect();
    assert_eq!(
        in_turn,
        [
            "i: rename | Control+o: back to Main | Space: GO | Escape: Panic | ?: help",
            "Control+o: back to Main | Space: GO | Escape: Panic | ?: help",
            "Control+o: back to Main | Escape: Panic | ?: help",
            "Control+o: back to Main | ?: help",
            "?: help",
            "",
        ]
    );
}

#[test]
fn the_cue_list_keeps_its_order_of_giving_way() {
    let cues = yielding(&App::default());
    assert_eq!(
        cues[..8],
        [
            Action::NextTab,
            Action::Standby,
            Action::Forward,
            Action::Back,
            Action::Go,
            Action::Panic,
            Action::Save,
            Action::ChangeName
        ]
    );
    assert_ne!(cues, yielding(&in_outputs(Preset::Vi)));
}

#[test]
fn a_cut_output_stays_a_cut_in_the_notice_and_in_undo_and_redo() {
    let sub = OutputId(2);
    let show = Workspace::default().apply(Command::add_output(1, 1));
    let told = |workspace: Workspace| {
        notice(&App {
            workspace,
            ..App::default()
        })
    };
    let cut = show.clone().apply(Command::CutOutputs(vec![sub]));
    assert_eq!(
        told(cut.clone()).as_deref(),
        Some("Cut the output Output 1 | u: undo")
    );
    let undone = cut.clone().apply(Command::Undo);
    assert_eq!(
        told(undone.clone()).as_deref(),
        Some("Undid: cut the output Output 1 | Control+r: redo")
    );
    let redone = undone.apply(Command::Redo);
    assert_eq!(
        told(redone).as_deref(),
        Some("Redid: cut the output Output 1 | u: undo")
    );
    // Delete says so, as it did
    let deleted = show.apply(Command::DeleteOutputs(vec![sub]));
    assert_eq!(
        told(deleted).as_deref(),
        Some("Deleted the output Output 1 | u: undo")
    );
}

/// Returns a show of one cue that plays a file, with a second output
fn rain() -> Workspace {
    let cue = Cue::new(CueId(1), "1", Kind::Audio, "Rain")
        .file("rain.ogg")
        .build();
    Workspace::new(vec![CueList::new("Main", vec![cue])]).apply(Command::add_output(1, 1))
}

fn rename(id: u32, name: &str) -> Command {
    Command::EditOutput(OutputId(id), OutputEdit::Name(name.to_owned()))
}

/// Returns the hints beside the status of the app that shows `workspace`,
/// which `room` columns hold
fn told(workspace: Workspace, room: usize) -> String {
    let app = App {
        workspace,
        ..App::default()
    };
    name::shown(&app, room).1
}

#[test]
fn a_cue_that_sends_to_no_output_left_is_a_warning() {
    let sends: Sends = [(OutputId(2), 0.0)].into();
    let show = rain().apply(Command::Edit(CueId(1), Edit::Sends(Some(sends))));
    let warnings = |workspace: Workspace| {
        crate::front::view::warnings(&App {
            workspace,
            ..App::default()
        })
    };
    let counted = warnings(show.clone());
    let stranded = show.apply(Command::DeleteOutputs(vec![OutputId(2)]));
    assert_eq!(warnings(stranded.clone()), counted + 1);
    assert_eq!(warnings(stranded.apply(Command::Undo)), counted);
}

#[test]
fn what_a_change_did_to_the_cues_comes_first_and_stays_whole_beside_long_names() {
    let base = rain()
        .apply(rename(2, "Dante Virtual Soundcard Left Main Speaker"))
        .apply(rename(1, "Another long name"));
    // The bar of 1024 pixels with the mark of unsaved changes, which takes ten
    // columns of the status
    let in_bar = |workspace| told(workspace, COLUMNS - 10);
    let deleted = base
        .clone()
        .apply(Command::DeleteOutputs(vec![OutputId(1)]));
    assert_eq!(
        in_bar(deleted.clone()),
        "Cues without sends play on Dante Vir…, deleted… | u: undo"
    );
    assert_eq!(
        in_bar(deleted.clone().apply(Command::Undo)),
        "Cues without sends play on Another l… | Control+r: redo"
    );
    assert_eq!(
        in_bar(deleted.apply(Command::Undo).apply(Command::Redo)),
        "Cues without sends play on Dante Vir…, redid… | u: undo"
    );
    let moved = Command::MoveOutput {
        output: OutputId(1),
        by: 1,
    };
    assert_eq!(
        in_bar(base.clone().apply(moved)),
        "Cues without sends play on Dante Vir…, moved… | u: undo"
    );
    let added = Command::add_output(0, 1);
    assert_eq!(
        in_bar(base.clone().apply(added)),
        "Cues without sends play on Output 1, added Output 1 | u: undo"
    );
    // A change that leaves the first output alone tells itself, with its names
    // short
    let calm = Command::EditOutput(
        OutputId(1),
        OutputEdit::Channels(crate::back::Channels::Mono(1)),
    );
    assert_eq!(
        in_bar(base.apply(calm)),
        "Changed the channels of the output Another l… | u: undo"
    );
}

#[test]
fn the_notice_of_a_cut_keeps_what_it_did_to_the_cues_whole_and_gives_up_the_key_to_paste() {
    let before = rain().apply(rename(2, "Sub"));
    let taken = before.outputs()[..1].to_vec();
    let workspace = before.clone().apply(Command::CutOutputs(vec![OutputId(1)]));
    let facts = format::consequences(&workspace, before.outputs(), workspace.outputs());
    let app = App {
        clipped: Some(Clipped {
            told: format::clipped_outputs("cut", &taken, facts),
            cut: true,
        }),
        workspace,
        ..App::default()
    };
    // The bar of 1024 pixels, with the mark of unsaved changes and a warning
    // in the status
    let shown = |room| name::shown(&app, room).1;
    assert_eq!(
        shown(COLUMNS - 10),
        "Cues without sends play on Sub, cut Main | p: paste | u: undo"
    );
    // Room for the notice and the key of undo alone, as a longer status leaves
    assert_eq!(
        shown(COLUMNS - 20),
        "Cues without sends play on Sub, cut Main | u: undo"
    );
}

#[test]
fn an_undo_that_moves_the_first_output_back_tells_where_cues_play_without_a_dangling_ellipsis() {
    let deleted = rain().apply(Command::DeleteOutputs(vec![OutputId(1)]));
    let undone = deleted.apply(Command::Undo);
    assert_eq!(
        told(undone.clone(), COLUMNS),
        "Cues without sends play on Main, undid: deleted Main | Control+r: redo"
    );
    // With the mark of unsaved changes in the status, what was undone gives way
    // to the words that tell where the cues play, and then goes altogether,
    // leaving no ellipsis after a name that is whole
    assert_eq!(
        told(undone.clone(), COLUMNS - 10),
        "Cues without sends play on Main, undid… | Control+r: redo"
    );
    assert_eq!(
        told(undone, COLUMNS - 20),
        "Cues without sends play on Main | Control+r: redo"
    );
}

#[test]
fn a_change_to_a_cue_is_not_told_while_the_outputs_show_so_that_their_keys_stay() {
    let edited = rain().apply(Command::Edit(CueId(1), Edit::File("storm.ogg".to_owned())));
    let app = |workspace: Workspace, outputs_shown| App {
        workspace,
        outputs_shown,
        ..opened(Preset::Vi, false, false)
    };
    assert!(notice(&app(edited.clone(), false)).is_some());
    assert_eq!(notice(&app(edited.clone(), true)), None);
    assert_eq!(
        shown(&app(edited, true), COLUMNS),
        "o: add output | i: rename | Control+o: back to Main | Space: GO | Escape: Panic | ?: help"
    );
    // A change to the outputs is told there
    let renamed = rain().apply(rename(1, "Pit"));
    assert!(notice(&app(renamed, true)).is_some_and(|told| told.contains("Pit")));
}
