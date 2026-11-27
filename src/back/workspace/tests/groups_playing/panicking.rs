// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of Panic on a group that is running or paused

use super::*;

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
    // Second panic stops everything at once, groups included
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
