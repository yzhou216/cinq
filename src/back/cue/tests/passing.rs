// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of how a clip that a devamp cue has let go on tells where its sound
//! is, and of the passes it makes

use super::*;

#[test]
fn sound_in_the_last_pass_has_played_to_that_pass_whatever_the_clock_says() {
    let free = sectioned().released(4.2, 0.5);
    assert_eq!(free.played_at(4.9, 100.0), 5.9);
    assert_eq!(free.played_at(3.0, 100.0), 4.0);
    assert_eq!(free.played_at(3.0, 0.0), 2.0);
}
