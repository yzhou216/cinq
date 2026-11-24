// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of what the notice of a change tells while the outputs show, and how
//! it shortens

use unicode_width::UnicodeWidthStr;

use super::{
    super::{hints::notices::notice, hints_in, name},
    cancel::COLUMNS,
    opened,
    outputs::shown,
};
use crate::{
    back::{Command, Cue, CueId, CueList, Edit, Kind, OutputEdit, OutputId, Sends, Workspace},
    front::{App, keymap::Preset, update::Clipped, view::format},
};

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
        "Changed the channels of Another l… | u: undo | ?: help"
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
        "o: add output | i: rename | Control+o: back to Main | Space: GO | Escape: Panic | ?: help | f: hint"
    );
    // A change to the outputs is told there
    let renamed = rain().apply(rename(1, "Pit"));
    assert!(notice(&app(renamed, true)).is_some_and(|told| told.contains("Pit")));
}

#[test]
fn a_notice_that_fits_in_no_room_ends_in_an_ellipsis_at_the_edge_of_the_room() {
    let base = rain().apply(rename(1, "House speaker pair 02"));
    let app = App {
        workspace: base.apply(Command::CutOutputs(vec![OutputId(1)])),
        outputs_shown: true,
        ..App::default()
    };
    // Rooms narrower than the notice with its keys cut to their least, which
    // the cut at a word does not reach, and than the one key that stays
    // where the notice is left out
    (4..7).for_each(|room| {
        let hints = hints_in(&app, room);
        assert_eq!(hints.width(), room, "{hints}");
        assert!(hints.ends_with('…'), "{hints}");
    });
}

#[test]
fn a_notice_in_the_least_rooms_never_overruns_them() {
    let long = "House-speaker-pair-02-with-a-very-long-single-word";
    let base = rain().apply(rename(1, long));
    let app = App {
        workspace: base.apply(Command::CutOutputs(vec![OutputId(1)])),
        outputs_shown: true,
        ..App::default()
    };
    (0..4).for_each(|room| {
        let hints = hints_in(&app, room);
        assert!(hints.width() <= room, "room {room}: {hints:?}");
    });
}

#[test]
fn the_notice_of_a_delete_or_a_new_file_tells_that_the_cue_it_changed_stopped() {
    let app = |change: Command, stopped: &[CueId]| App {
        workspace: rain().apply(change),
        stopped: stopped.to_vec(),
        ..App::default()
    };
    let delete = || Command::Delete(vec![CueId(1)]);
    let file = || Command::Edit(CueId(1), Edit::File("hail.ogg".into()));
    assert_eq!(
        notice(&app(delete(), &[CueId(1)])).as_deref(),
        Some("Deleted cue 1, which stopped | u: undo")
    );
    assert_eq!(
        notice(&app(file(), &[CueId(1)])).as_deref(),
        Some("Changed the file of cue 1, which stopped | u: undo")
    );
    assert_eq!(
        notice(&app(file(), &[])).as_deref(),
        Some("Changed the file of cue 1 | u: undo")
    );
    // The undo of the change does not claim the stop
    let undone = App {
        workspace: rain().apply(file()).apply(Command::Undo),
        stopped: vec![CueId(1)],
        ..App::default()
    };
    assert_eq!(
        notice(&undone).as_deref(),
        Some("Undid: changed the file of cue 1 | Control+r: redo")
    );
}

#[test]
fn an_undo_or_redo_of_several_changes_tells_how_many_and_the_last() {
    let added = (0..5).fold(rain(), |workspace, _| {
        workspace.apply(Command::Add {
            list: 0,
            kind: Kind::Memo,
            at: crate::back::Place::End,
            count: 1,
        })
    });
    let app = |workspace, turns| App {
        workspace,
        turns,
        ..App::default()
    };
    let undone = (0..3).fold(added, |workspace, _| workspace.apply(Command::Undo));
    assert_eq!(
        notice(&app(undone.clone(), 3)).as_deref(),
        Some("Undid 3 changes, the last: added cue 4 | Control+r: redo")
    );
    assert_eq!(
        notice(&app(undone.clone(), 1)).as_deref(),
        Some("Undid: added cue 4 | Control+r: redo")
    );
    let redone = undone.apply(Command::Redo).apply(Command::Redo);
    assert_eq!(
        notice(&app(redone, 2)).as_deref(),
        Some("Redid 2 changes, the last: added cue 5 | u: undo")
    );
}

#[test]
fn light_output_that_cannot_go_out_counts_as_a_warning_until_a_round_gets_out() {
    let count = |light_problem| {
        crate::front::view::warnings(&App {
            light_problem,
            ..App::default()
        })
    };
    assert_eq!(
        count(Some("no network interface".to_owned())),
        count(None) + 1
    );
}

#[test]
fn the_notice_of_a_time_that_reads_as_no_time_tells_that_it_counts_as_zero() {
    let told = |time: &str| {
        notice(&App {
            workspace: rain().apply(Command::Edit(
                CueId(1),
                Edit::PreWait(time.to_owned().into()),
            )),
            ..App::default()
        })
    };
    assert_eq!(
        told("1:0:0:5").as_deref(),
        Some("Changed the pre-wait of cue 1 to zero | u: undo")
    );
    assert_eq!(
        told("2").as_deref(),
        Some("Changed the pre-wait of cue 1 | u: undo")
    );
}

#[test]
fn the_notice_of_a_cue_gone_silent_outlasts_what_enter_does_but_a_plain_one_does_not() {
    let app = |sends: Option<Sends>| App {
        held: true,
        check_held: true,
        workspace: rain().apply(Command::Edit(CueId(1), Edit::Sends(sends))),
        ..opened(Preset::Vi, false, false)
    };
    let room = 52;
    let silent = hints_in(&app(Some(Sends::new())), room);
    assert!(silent.contains("play nowhere"), "{silent}");
    assert!(
        silent.contains("leave") && !silent.contains("Enter"),
        "{silent}"
    );
    let plain = hints_in(&app(None), room);
    assert!(plain.contains("Enter: flip"), "{plain}");
}

#[test]
fn only_a_notice_of_silence_that_an_undo_has_not_taken_back_silences() {
    assert!(format::silences("changed cue 2 to play nowhere"));
    assert!(format::silences("1 cue now sends to no output"));
    assert!(!format::silences("undid: changed cue 2 to play nowhere"));
    assert!(!format::silences("changed where cue 2 plays"));
}
