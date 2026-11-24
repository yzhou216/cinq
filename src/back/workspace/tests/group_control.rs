// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the cues that act on a group, of stopping what waits to start
//! one, and of readying the cues of a group ahead

use super::{
    groups_playing::{phase_of, run_of},
    *,
};

/// Returns a show whose group 1 holds a cue that plays, one that waits two
/// seconds, and a group, with a cue after it that waits for its post-wait
fn grouped() -> Vec<Cue> {
    vec![
        Cue {
            advance: Advance::Continue,
            post_wait: 4.0.into(),
            ..group(1)
        },
        within(1, audio(2)),
        Cue {
            pre_wait: 2.0.into(),
            ..within(1, group(3))
        },
        within(2, audio(4)),
        audio(5),
    ]
}

fn all_paused(workspace: &Workspace) -> Vec<bool> {
    workspace
        .active()
        .iter()
        .map(|active| active.paused)
        .collect()
}

#[test]
fn stop_cue_on_a_group_stops_its_run_its_cues_and_the_cue_after_it() {
    let stopped = workspace([grouped(), vec![stop_cue(6, 1)]].concat())
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(GO);
    assert!(running(&stopped).is_empty());
}

#[test]
fn stop_button_stops_a_group_and_the_cues_waiting_inside_it() {
    let started = workspace(grouped()).apply(GO);
    assert_eq!(running(&started), [1, 2, 3, 4, 5]);
    let stopped = started.apply(Command::Stop(CueId(1)));
    assert!(running(&stopped).is_empty());
}

#[test]
fn stopping_a_nested_group_leaves_the_group_around_it_to_its_other_cues() {
    let stopped = workspace(grouped())
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(Command::Stop(CueId(3)));
    assert_eq!(running(&stopped), [1, 2, 5]);
}

#[test]
fn stopping_the_cue_that_waits_to_start_a_group_cancels_the_cues_of_the_group() {
    let waiting = workspace(vec![
        continuing_after(1, 3.0),
        group(2),
        within(1, audio(3)),
        within(1, group(4)),
        within(2, audio(5)),
    ])
    .apply(GO)
    .apply(Command::Tick(1.0));
    assert_eq!(running(&waiting), [1, 2, 3, 4, 5]);
    assert!(running(&waiting.apply(Command::Stop(CueId(1)))).is_empty());
}

#[test]
fn stop_cue_before_a_group_cancels_the_group_that_waits_for_it() {
    let waiting = workspace(vec![
        continuing_after(1, 3.0),
        group(2),
        within(1, audio(3)),
        stop_cue(4, 1),
    ])
    .apply(GO)
    .apply(Command::Tick(1.0))
    .apply(move_standby(4))
    .apply(GO);
    assert!(running(&waiting).is_empty());
}

#[test]
fn stopping_a_group_that_began_cancels_its_waiting_cues_and_the_cue_after_it() {
    let late = Cue {
        pre_wait: 3.0.into(),
        ..within(1, audio(3))
    };
    let begun = workspace(vec![
        Cue {
            advance: Advance::Continue,
            post_wait: 4.0.into(),
            ..group(1)
        },
        within(1, audio(2)),
        late,
        audio(4),
    ])
    .apply(GO)
    .apply(Command::Tick(1.0));
    assert!(running(&begun.apply(Command::Stop(CueId(1)))).is_empty());
}

#[test]
fn end_of_a_group_stops_nothing() {
    let short = Cue {
        end: 1.0.into(),
        ..within(1, audio(2))
    };
    let ended = workspace(vec![
        Cue {
            advance: Advance::Continue,
            post_wait: 5.0.into(),
            ..group(1)
        },
        short,
        audio(3),
    ])
    .apply(GO)
    .apply(Command::Tick(2.0));
    assert_eq!(running(&ended), [3]);
    assert_eq!(
        phase_of(&ended, 3),
        Some(Phase::Scheduled { starts_in: 3.0 })
    );
}

#[test]
fn pause_cue_on_a_group_holds_every_run_it_would_start() {
    let held = workspace([grouped(), vec![pause_cue(6, 1)]].concat())
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(GO)
        .apply(Command::Tick(3.0));
    assert_eq!(running(&held), [1, 2, 3, 4, 5]);
    assert!(all_paused(&held).iter().all(|&paused| paused));
    assert_eq!(
        phase_of(&held, 5),
        Some(Phase::Scheduled { starts_in: 3.0 })
    );
    assert_eq!(
        phase_of(&held, 4),
        Some(Phase::Scheduled { starts_in: 1.0 })
    );
}

#[test]
fn start_cue_on_a_paused_group_resumes_its_runs_and_fires_nothing() {
    let resumed = workspace([grouped(), vec![pause_cue(6, 1), start_cue(7, 1)]].concat())
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(GO)
        .apply(Command::Tick(3.0))
        .apply(GO);
    assert_eq!(running(&resumed), [1, 2, 3, 4, 5]);
    assert!(all_paused(&resumed).iter().all(|&paused| !paused));
    let later = resumed.apply(Command::Tick(1.0));
    assert_eq!(
        phase_of(&later, 5),
        Some(Phase::Scheduled { starts_in: 2.0 })
    );
}

#[test]
fn start_cue_on_a_group_that_is_not_paused_fires_it_with_the_cues_it_holds() {
    let fired = workspace(vec![
        start_cue(1, 2),
        group(2),
        within(1, audio(3)),
        within(1, audio(4)),
    ])
    .apply(GO)
    .apply(Command::Tick(1.0));
    assert_eq!(running(&fired), [2, 3, 4]);
    assert_eq!(standby(&fired), Some(2));
}

#[test]
fn go_on_a_group_again_restarts_it_and_cancels_what_the_old_run_would_start() {
    let first = workspace(grouped()).apply(GO).apply(Command::Tick(1.0));
    let old = run_of(&first, 1);
    let again = first.apply(move_standby(1)).apply(GO);
    assert_eq!(running(&again).len(), 5);
    assert!(again.active().iter().all(|active| active.run > old));
    assert_eq!(
        phase_of(&again, 5),
        Some(Phase::Scheduled { starts_in: 4.0 })
    );
}

#[test]
fn go_to_cue_on_a_group_puts_it_on_standby() {
    let moved = workspace(vec![goto_cue(1, 2), group(2), within(1, audio(3))]).apply(GO);
    assert_eq!(standby(&moved), Some(2));
}

#[test]
fn fade_and_devamp_cues_cannot_target_a_group() {
    let workspace = workspace(vec![
        group(1),
        within(1, vamp(2)),
        control(3, Kind::Fade, 1),
        devamp_cue(4, 1),
    ]);
    let problems: Vec<(u32, Problem)> = workspace
        .problems()
        .map(|(cue, problem)| (cue.id.0, problem))
        .collect();
    assert_eq!(
        problems,
        [(3, Problem::WrongTarget), (4, Problem::WrongTarget)]
    );
}

#[test]
fn start_cue_on_a_group_readies_the_cues_it_holds_ahead() {
    let workspace = workspace(vec![
        Cue {
            pre_wait: 1.0.into(),
            ..start_cue(1, 2)
        },
        Cue {
            pre_wait: 2.0.into(),
            ..group(2)
        },
        within(1, audio(3)),
        within(1, group(4)),
        within(2, audio(5)),
    ]);
    let ahead: Vec<(u32, f32)> = workspace
        .ahead()
        .into_iter()
        .map(|(cue, delay)| (cue.id.0, delay))
        .collect();
    assert_eq!(ahead, [(1, 0.0), (2, 1.0), (3, 3.0), (4, 3.0), (5, 3.0)]);
}

#[test]
fn cues_of_the_next_group_are_readied_ahead_with_their_delays() {
    let workspace = workspace(vec![
        Cue {
            pre_wait: 2.0.into(),
            ..group(1)
        },
        within(1, audio(2)),
        within(1, audio(3)),
    ]);
    let ahead: Vec<(u32, f32)> = workspace
        .ahead()
        .into_iter()
        .map(|(cue, delay)| (cue.id.0, delay))
        .collect();
    assert_eq!(ahead, [(1, 0.0), (2, 2.0), (3, 2.0)]);
}

#[test]
fn start_cue_in_the_group_that_it_starts_cannot_play() {
    let list = |depth| {
        workspace(vec![
            group(1),
            within(1, group(2)),
            within(depth, start_cue(3, 1)),
            within(2, audio(4)),
        ])
    };
    // At the depth of the group's cues, and deeper
    for depth in [1, 2] {
        let list = list(depth);
        assert_eq!(problems(&list), [(3, Problem::InTarget)]);
        assert_eq!(
            Problem::InTarget.to_string(),
            "lies in the group that it starts"
        );
        let fired = list.apply(GO);
        assert!(!running(&fired).contains(&3), "GO skips the start cue");
    }
    let outside = workspace(vec![group(1), within(1, audio(2)), start_cue(3, 1)]);
    assert_eq!(problems(&outside), []);
}

#[test]
fn stop_pause_and_go_to_cues_may_lie_in_their_own_group() {
    let workspace = workspace(vec![
        group(1),
        within(1, stop_cue(2, 1)),
        within(1, pause_cue(3, 1)),
        within(1, goto_cue(4, 1)),
    ]);
    assert_eq!(workspace.problems().count(), 0);
}

fn problems(workspace: &Workspace) -> Vec<(u32, Problem<'_>)> {
    workspace
        .problems()
        .map(|(cue, problem)| (cue.id.0, problem))
        .collect()
}
