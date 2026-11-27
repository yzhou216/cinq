// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of how the standby that is about to move is told

use super::*;

#[test]
fn standby_about_to_move_is_told_by_the_number_of_its_target_and_the_seconds() {
    assert_eq!(moving(&memo(5), 2.94), "Standby moves to 5 in 2.9s");
}

#[test]
fn a_time_ahead_turns_to_a_clock_from_a_minute_on() {
    assert_eq!(moving(&memo(5), 75.0), "Standby moves to 5 in 1:15.0");
    assert_eq!(moving(&memo(5), 59.97), "Standby moves to 5 in 1:00.0");
}
