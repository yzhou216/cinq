// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of a level edit of a cue that plays, which moves the run by the
//! change and leaves the fades of other cues going

use super::*;
use crate::back::LOUDEST;

/// Returns a fade cue that fades cue 1 to `to` dB over `seconds` and leaves
/// it playing
fn ducking(id: u32, to: f32, seconds: f32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Fade, "Duck")
        .target(CueId(1))
        .level(to)
        .duration(seconds)
        .stop_target(false)
        .build()
}

/// Returns a workspace in which cue 1 plays and cue 2 has ducked it to -20 dB
fn ducked() -> Workspace {
    let ducked = workspace(vec![continuing(1), ducking(2, -20.0, 2.0)])
        .apply(GO)
        .apply(Command::Tick(3.0));
    assert_eq!(level(&ducked, 1), Some(-20.0));
    ducked
}

fn set(workspace: Workspace, level: f32) -> Workspace {
    workspace.apply(edit(1, Edit::Level(level)))
}

#[test]
fn level_edit_moves_a_run_by_the_change_and_not_to_the_level() {
    let nudged = set(ducked(), -0.5);
    assert_eq!(level(&nudged, 1), Some(-20.5));
}

#[test]
fn level_dragged_to_the_bottom_and_back_restores_a_duck() {
    let bottom = set(set(ducked(), -0.5), -60.0);
    assert_eq!(level(&bottom, 1), Some(-80.0), "no floor below silence");
    assert_eq!(level(&set(bottom, 0.0), 1), Some(-20.0));
}

#[test]
fn level_edit_stops_at_the_loudest_level() {
    let raised = workspace(vec![continuing(1), ducking(2, 6.0, 1.0)])
        .apply(GO)
        .apply(Command::Tick(2.0));
    assert_eq!(level(&raised, 1), Some(6.0));
    assert_eq!(level(&set(raised, 12.0), 1), Some(LOUDEST));
}

#[test]
fn level_edit_of_a_run_that_waits_moves_it_too() {
    let waiting = workspace(vec![Cue {
        pre_wait: 5.0.into(),
        ..audio(1)
    }])
    .apply(GO)
    .apply(Command::Tick(1.0));
    assert_eq!(level(&set(waiting, -6.0), 1), Some(-6.0));
}

#[test]
fn level_edit_during_a_fade_that_leaves_the_cue_playing_moves_the_level_on() {
    let fading = workspace(vec![continuing(1), ducking(2, -20.0, 4.0)])
        .apply(GO)
        .apply(Command::Tick(2.0));
    let halfway = level(&fading, 1).expect("the cue plays");
    assert!(halfway < 0.0 && halfway > -20.0, "{halfway}");
    let moved = set(fading, -6.0);
    let active = moved.active_of(CueId(1)).expect("the cue plays");
    assert_eq!(active.level, halfway - 6.0);
    assert!(active.fade.is_some(), "the fade goes on");
    let ended = moved.apply(Command::Tick(4.0));
    let active = ended.active_of(CueId(1)).expect("the cue plays on");
    assert_eq!((active.level, active.fade.is_none()), (-20.0, true));
}

#[test]
fn level_edit_leaves_a_cue_that_a_fade_is_stopping() {
    let fading = workspace(vec![continuing(1), fade_out(2)])
        .apply(GO)
        .apply(Command::Tick(1.0));
    let before = level(&fading, 1);
    let edited = set(fading, -3.0);
    assert_eq!(level(&edited, 1), before);
    assert!(edited.apply(Command::Tick(4.0)).active().is_empty());
}

#[test]
fn undo_and_redo_of_a_level_edit_move_a_run_back_and_forth() {
    let nudged = set(ducked(), -6.0);
    assert_eq!(level(&nudged, 1), Some(-26.0));
    let undone = nudged.apply(Command::Undo);
    assert_eq!(level(&undone, 1), Some(-20.0));
    assert_eq!(level(&undone.apply(Command::Redo), 1), Some(-26.0));
}
