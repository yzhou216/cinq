// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of which cues of a chain a stopped cue takes along

use super::*;

#[test]
fn stopping_a_cue_cancels_the_chain_past_a_cue_that_could_not_play() {
    let failed = loaded(2, "sound.ogg", Media::Failed("file not found".to_owned()));
    let workspace = workspace(vec![
        continuing_after(1, 2.0),
        continuing_after(2, 2.0),
        audio(3),
    ])
    .apply(GO)
    .apply(Command::Tick(1.0))
    .apply(failed)
    .apply(Command::Stop(CueId(1)));
    assert!(workspace.active().is_empty());
}

#[test]
fn stopping_a_cue_cancels_the_chain_past_a_cue_that_was_fired_again() {
    let workspace = workspace(vec![
        continuing_after(1, 2.0),
        continuing_after(2, 2.0),
        audio(3),
    ])
    .apply(GO)
    .apply(Command::Tick(1.0))
    .apply(edit(2, Edit::Advance(Advance::Manual)))
    .apply(move_standby(2))
    .apply(GO);
    assert_eq!(running(&workspace), [1, 3, 2]);
    // The cue fired again has a run of its own, which stopping cue 1 leaves
    let stopped = workspace.apply(Command::Stop(CueId(1)));
    assert_eq!(running(&stopped), [2]);
}

#[test]
fn stopping_a_cue_cancels_every_cue_down_a_chain_of_waiting_cues() {
    let stopped = workspace(vec![
        continuing_after(1, 2.0),
        continuing_after(2, 2.0),
        continuing_after(3, 2.0),
        audio(4),
    ])
    .apply(GO)
    .apply(Command::Tick(1.0))
    .apply(Command::Stop(CueId(1)));
    assert!(stopped.active().is_empty());
}

#[test]
fn stopping_a_cue_leaves_the_cues_that_a_cue_it_started_has_yet_to_start() {
    let short = Cue {
        end: 1.0.into(),
        ..continuing_after(2, 5.0)
    };
    let workspace = workspace(vec![continuing(1), short, audio(3)])
        .apply(GO)
        .apply(Command::Tick(2.0));
    // Cue 2 has ended, and its post-wait still runs
    assert_eq!(running(&workspace), [1, 3]);
    let stopped = workspace.apply(Command::Stop(CueId(1)));
    assert_eq!(running(&stopped), [3]);
}

#[test]
fn stopping_a_cue_leaves_the_cues_past_one_that_failed_after_it_started() {
    let failed = loaded(2, "sound.ogg", Media::Failed("file not found".to_owned()));
    let workspace = workspace(vec![
        continuing_after(1, 2.0),
        continuing_after(2, 2.0),
        audio(3),
    ])
    .apply(GO)
    .apply(Command::Tick(2.0))
    .apply(failed);
    // Cue 2 had started, so it was the one to start cue 3, which still waits
    assert_eq!(running(&workspace), [1, 3]);
    let stopped = workspace.apply(Command::Stop(CueId(1)));
    assert_eq!(running(&stopped), [3]);
}
