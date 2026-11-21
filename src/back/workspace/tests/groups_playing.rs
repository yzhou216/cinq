// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of GO on a group and of the run of a group: what it fires, how
//! long it lasts, what it shows, and which run starts which

use super::*;

/// Returns the group `id` with the setting that goes on to the next cue
fn then(id: u32, advance: Advance, pre_wait: f32, post_wait: f32) -> Cue {
    Cue {
        advance,
        pre_wait: pre_wait.into(),
        post_wait: post_wait.into(),
        ..group(id)
    }
}

pub(super) fn phase_of(workspace: &Workspace, id: u32) -> Option<Phase> {
    workspace.active_of(CueId(id)).map(|active| active.phase())
}

/// Returns the run that starts the run of cue `id`
pub(super) fn started_by(workspace: &Workspace, id: u32) -> Option<u32> {
    workspace
        .active_of(CueId(id))
        .and_then(|active| active.after)
}

pub(super) fn run_of(workspace: &Workspace, id: u32) -> u32 {
    workspace.active_of(CueId(id)).expect("the cue runs").run
}

#[test]
fn group_with_its_cues_runs_while_any_of_them_does() {
    let late = Cue {
        pre_wait: 2.0.into(),
        ..within(1, audio(3))
    };
    let fired = workspace(vec![
        then(1, Advance::Manual, 1.0, 0.0),
        within(1, audio(2)),
        late,
    ])
    .apply(GO);
    assert_eq!(phase_of(&fired, 1), Some(Phase::PreWait { remaining: 1.0 }));
    assert_eq!(
        phase_of(&fired, 2),
        Some(Phase::Scheduled { starts_in: 1.0 })
    );
    let begun = fired.apply(Command::Tick(1.0));
    assert_eq!(
        phase_of(&begun, 1),
        Some(Phase::Running { remaining: 12.0 })
    );
    let half = begun.apply(Command::Tick(5.0));
    assert_eq!(phase_of(&half, 1), Some(Phase::Running { remaining: 7.0 }));
    assert_eq!(half.active_of(CueId(1)).unwrap().progress(), 5.0 / 12.0);
    // The first cue has ended, and the group lasts for the second
    let first = half.apply(Command::Tick(5.0));
    assert_eq!(running(&first), [1, 3]);
    assert_eq!(phase_of(&first, 1), Some(Phase::Running { remaining: 2.0 }));
    assert!(running(&first.apply(Command::Tick(2.0))).is_empty());
}

#[test]
fn group_ends_once_the_cues_it_holds_have_been_stopped() {
    let stopped = workspace(vec![group(1), within(1, audio(2)), within(1, audio(3))])
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(Command::Stop(CueId(2)));
    assert_eq!(running(&stopped), [1, 3]);
    assert!(running(&stopped.apply(Command::Stop(CueId(3)))).is_empty());
}

#[test]
fn group_with_no_cue_that_plays_ends_as_its_action_begins() {
    let unplayable = Cue::new(CueId(2), "2", Kind::Audio, "Sound").build();
    let waiting = Cue {
        pre_wait: 3.0.into(),
        ..group(1)
    };
    let fired = workspace(vec![waiting, within(1, unplayable)]).apply(GO);
    assert_eq!(running(&fired), [1]);
    assert!(running(&fired.apply(Command::Tick(3.0))).is_empty());
}

#[test]
fn group_with_a_loop_runs_without_end_and_is_shown_so() {
    let following = then(1, Advance::Follow, 0.0, 0.0);
    let workspace = workspace(vec![
        following,
        within(1, vamp(2)),
        within(1, audio(3)),
        audio(4),
    ]);
    // No auto-follow where a cue in the group never ends
    let chain: Vec<u32> = workspace.lists()[0]
        .chain(CueId(1))
        .map(|(cue, _)| cue.id.0)
        .collect();
    assert_eq!(chain, [1, 2, 3]);
    let later = workspace.apply(GO).apply(Command::Tick(100.0));
    assert_eq!(running(&later), [1, 2]);
    assert_eq!(phase_of(&later, 1), Some(Phase::Endless));
    assert_eq!(later.active_of(CueId(1)).unwrap().progress(), 1.0);
    assert_eq!(standby(&later), Some(4));
}

#[test]
fn group_in_a_group_runs_until_every_cue_at_any_depth_has_ended() {
    let inner = Cue {
        pre_wait: 2.0.into(),
        ..within(1, group(2))
    };
    let fired = workspace(vec![
        then(1, Advance::Manual, 1.0, 0.0),
        inner,
        within(2, audio(3)),
        within(
            1,
            Cue {
                end: 2.0.into(),
                ..audio(4)
            },
        ),
    ])
    .apply(GO);
    assert_eq!(
        phase_of(&fired, 2),
        Some(Phase::Scheduled { starts_in: 1.0 })
    );
    assert_eq!(
        phase_of(&fired, 3),
        Some(Phase::Scheduled { starts_in: 3.0 })
    );
    assert_eq!(
        phase_of(&fired, 4),
        Some(Phase::Scheduled { starts_in: 1.0 })
    );
    let begun = fired.apply(Command::Tick(1.0));
    assert_eq!(
        phase_of(&begun, 1),
        Some(Phase::Running { remaining: 12.0 })
    );
    assert_eq!(phase_of(&begun, 2), Some(Phase::PreWait { remaining: 2.0 }));
    let inside = begun.apply(Command::Tick(2.0));
    assert_eq!(
        phase_of(&inside, 2),
        Some(Phase::Running { remaining: 10.0 })
    );
    assert_eq!(running(&inside), [1, 2, 3]);
    assert_eq!(
        running(&inside.clone().apply(Command::Tick(9.0))),
        [1, 2, 3]
    );
    assert!(running(&inside.apply(Command::Tick(10.0))).is_empty());
}

#[test]
fn group_that_continues_fires_the_next_cue_after_its_pre_wait_and_post_wait() {
    let fired = workspace(vec![
        then(1, Advance::Continue, 1.0, 2.0),
        within(1, audio(2)),
        audio(3),
    ])
    .apply(GO);
    assert_eq!(
        phase_of(&fired, 3),
        Some(Phase::Scheduled { starts_in: 3.0 })
    );
    assert_eq!(standby(&fired), None);
}

#[test]
fn group_that_follows_fires_the_next_cue_as_the_last_of_its_cues_ends() {
    let late = Cue {
        pre_wait: 2.0.into(),
        ..within(1, audio(3))
    };
    let fired = workspace(vec![
        then(1, Advance::Follow, 1.0, 0.0),
        within(1, audio(2)),
        late,
        audio(4),
    ])
    .apply(GO);
    assert_eq!(
        phase_of(&fired, 4),
        Some(Phase::Scheduled { starts_in: 13.0 })
    );
    let end = fired.apply(Command::Tick(13.0));
    assert_eq!(running(&end), [4]);
}

#[test]
fn run_of_a_group_starts_the_cues_it_holds_and_the_cue_that_follows_it() {
    let fired = workspace(vec![
        then(1, Advance::Continue, 0.0, 4.0),
        within(1, audio(2)),
        within(1, group(3)),
        within(2, audio(4)),
        continuing_after(5, 2.0),
        audio(6),
    ])
    .apply(GO);
    assert_eq!(running(&fired), [1, 2, 3, 4, 5, 6]);
    let after = |id| started_by(&fired, id);
    let run = |id| Some(run_of(&fired, id));
    assert_eq!(after(1), None);
    assert_eq!(after(2), run(1));
    assert_eq!(after(3), run(1));
    assert_eq!(after(4), run(3));
    assert_eq!(after(5), run(1));
    assert_eq!(after(6), run(5));
}

#[test]
fn chain_that_reaches_a_group_starts_it_by_the_cue_before_it() {
    let fired = workspace(vec![continuing(1), group(2), within(1, audio(3)), audio(4)]).apply(GO);
    assert_eq!(started_by(&fired, 2), Some(run_of(&fired, 1)));
    assert_eq!(started_by(&fired, 3), Some(run_of(&fired, 2)));
}

#[test]
fn go_with_standby_in_a_group_fires_that_cue_alone_and_moves_to_the_next_row() {
    let fired = workspace(vec![
        group(1),
        within(1, continuing(2)),
        within(1, group(3)),
        within(2, audio(4)),
        within(1, audio(5)),
        audio(6),
    ])
    .apply(move_standby(2))
    .apply(GO);
    assert_eq!(running(&fired), [2]);
    assert_eq!(standby(&fired), Some(3));
    // A group inside it fires with its own cues, and standby leaves it
    let inner = fired.apply(GO);
    assert_eq!(running(&inner), [2, 3, 4]);
    assert_eq!(started_by(&inner, 3), None);
    assert_eq!(started_by(&inner, 4), Some(run_of(&inner, 3)));
    assert_eq!(standby(&inner), Some(5));
}

#[test]
fn group_in_another_list_runs_when_a_start_cue_fires_it() {
    let fired = lists_of(vec![
        vec![start_cue(1, 2)],
        vec![group(2), within(1, audio(3))],
    ])
    .apply(GO)
    .apply(Command::Tick(1.0));
    assert_eq!(running(&fired), [2, 3]);
    assert_eq!(started_by(&fired, 3), Some(run_of(&fired, 2)));
    assert_eq!(standby_of(&fired, 1), Some(2));
}

#[test]
fn panic_keeps_the_run_of_a_group_while_its_cues_fade() {
    let panicked = workspace(vec![group(1), within(1, audio(2))])
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(Command::Panic);
    assert_eq!(running(&panicked), [1, 2]);
    let fading = panicked.clone().apply(Command::Tick(PANIC_FADE / 2.0));
    assert_eq!(running(&fading), [1, 2]);
    let faded = panicked.clone().apply(Command::Tick(PANIC_FADE));
    assert!(running(&faded).is_empty());
    // A second panic stops everything at once, groups included
    assert!(panicked.apply(Command::Panic).active().is_empty());
}

#[test]
fn panic_drops_a_paused_group_with_its_cues() {
    let panicked = workspace(vec![group(1), within(1, audio(2))])
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(Command::Pause(true))
        .apply(Command::Panic);
    assert!(panicked.active().is_empty());
}
