// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of a group that continues or follows to the next cue

use super::*;

#[test]
fn group_that_continues_fires_the_next_cue_after_its_pre_wait_and_post_wait() {
    let fired = workspace(vec![
        then(1, Advance::Continue, 1.0, 2.0),
        within(1, audio(2)),
        audio(3),
    ])
    .apply(GO);
    assert_phase(&fired, 3, Phase::Scheduled { starts_in: 3.0 });
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
    assert_phase(&fired, 4, Phase::Scheduled { starts_in: 13.0 });
    let end = fired.apply(Command::Tick(13.0));
    assert_eq!(running(&end), [4]);
}
