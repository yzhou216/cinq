// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of making ready the sounds that start cues fire

use super::upcoming_cues;
use crate::back::{
    Cue,
    workspace::tests::{GO, audio, start_cue, workspace},
};

#[test]
fn sounds_that_start_cues_fire_are_made_ready_until_the_start_cues_act() {
    let waiting = Cue {
        pre_wait: 1.0.into(),
        ..start_cue(1, 3)
    };
    let workspace = workspace(vec![waiting, audio(2), audio(3)]).directory("/show");
    assert_eq!(upcoming_cues(&workspace), [3]);
    // Standby has moved on, and the start cue has yet to act
    assert_eq!(upcoming_cues(&workspace.apply(GO)), [3, 2]);
}
