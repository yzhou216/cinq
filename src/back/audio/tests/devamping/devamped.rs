// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of a devamped run leaving its loop at the planned pass end

use super::*;

#[test]
fn a_devamped_run_leaves_its_loop_at_the_planned_pass_end_and_plays_to_its_end() {
    let (directory, workspace) = vamp(RATE, 2.5, (0.5, 1.5), 0.0);
    // The pass that ends at 1.5 seconds is less than the notice away at 1.05
    // seconds, so the run leaves at the end of the pass after it, 2.5
    // seconds in, and its last pass begins at 1.5 seconds
    let (rig, seen) = Rig::start(&directory, workspace).run(160, |index, workspace| {
        if index == 42 {
            workspace.apply(GO)
        } else {
            workspace
        }
    });
    assert_eq!(wraps(&seen), 1, "it goes round once, at 1.5 seconds");
    let left = seen
        .iter()
        .position(|seen| seen.position > 1.5 + 1e-3)
        .map(|index| (index + 1) as f64 * TICK)
        .expect("the sound leaves its loop");
    assert!((2.5..=2.6).contains(&left), "{left}");
    let ends = seen
        .iter()
        .position(|seen| !seen.heard)
        .map(|index| (index + 1) as f64 * TICK)
        .expect("the sound ends");
    assert!((3.5..=3.65).contains(&ends), "{ends}");
    assert!(seen.iter().all(|seen| seen.position <= 2.5 + 1e-6));
    assert!(
        seen.iter().all(|seen| !seen.cut),
        "a devamp is no early stop"
    );
    assert!(rig.workspace.active_of(CueId(1)).is_none(), "the run ends");
    let off = drift(&seen);
    assert!(
        off < 0.06,
        "the clock follows the sound out of its loop: {off}"
    );
    assert!(!rig.voice.looped());
}
