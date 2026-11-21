// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of which fields the inspector shows for a cue, which it tells the
//! cue ignores, and which time it says reads as no time

use super::{Field, unread};
use crate::back::{Advance, Cue, CueId, Kind};

fn cue(kind: Kind) -> Cue {
    Cue::new(CueId(1), "1", kind, "Cue").build()
}

#[test]
fn a_kind_with_no_file_and_no_target_has_no_target_row() {
    [Kind::Light, Kind::Group, Kind::Wait, Kind::Memo]
        .into_iter()
        .for_each(|kind| assert!(!Field::Target.has_row(&cue(kind))));
    [Kind::Audio, Kind::Network, Kind::Stop]
        .into_iter()
        .for_each(|kind| assert!(Field::Target.has_row(&cue(kind))));
    assert!(Field::Notes.has_row(&cue(Kind::Memo)));
}

#[test]
fn a_network_cue_shows_its_message_where_other_cues_show_a_duration() {
    assert!(matches!(
        Field::Duration.of(&cue(Kind::Network)),
        Field::Osc
    ));
    assert!(matches!(
        Field::Duration.of(&cue(Kind::Audio)),
        Field::Duration
    ));
}

#[test]
fn a_cue_tells_why_it_ignores_a_field() {
    let following = Cue {
        advance: Advance::Follow,
        ..cue(Kind::Audio)
    };
    assert_eq!(
        Field::PostWait.ignored(&following, false),
        Some("with auto-follow")
    );
    assert_eq!(
        Field::PostWait.ignored(&cue(Kind::Audio), true),
        Some("in a group")
    );
    assert_eq!(
        Field::Then.ignored(&cue(Kind::Audio), true),
        Some("in a group")
    );
    assert_eq!(
        Field::LoopStart.ignored(&cue(Kind::Audio), false),
        Some("not looping")
    );
    assert_eq!(Field::PreWait.ignored(&following, true), None);
}

#[test]
fn the_first_time_that_reads_as_no_time_is_named() {
    let columns: [&[Field]; 2] = [&[Field::Name], &[Field::PreWait, Field::PostWait]];
    assert_eq!(unread(&cue(Kind::Audio), &columns), None);
    let typed = Cue {
        pre_wait: "1,5".to_owned().into(),
        post_wait: "abc".to_owned().into(),
        ..cue(Kind::Audio)
    };
    assert_eq!(
        unread(&typed, &columns).as_deref(),
        Some("Pre-wait counts as zero: use seconds or minutes:seconds")
    );
}

#[test]
fn an_end_or_a_loop_time_that_reads_as_no_time_is_told_as_empty() {
    let columns: [&[Field]; 1] = [&[Field::End, Field::LoopStart, Field::LoopEnd]];
    let words = |typed: Cue| unread(&typed, &columns);
    let empty = "reads as empty: use seconds or minutes:seconds";
    let end = Cue {
        end: "5s".to_owned().into(),
        ..cue(Kind::Audio)
    };
    assert_eq!(words(end), Some(format!("End {empty}")));
    let loop_start = Cue {
        loop_start: "5s".to_owned().into(),
        ..cue(Kind::Audio)
    };
    assert_eq!(words(loop_start), Some(format!("Loop start {empty}")));
    let loop_end = Cue {
        loop_end: "5s".to_owned().into(),
        ..cue(Kind::Audio)
    };
    assert_eq!(words(loop_end), Some(format!("Loop end {empty}")));
}

#[test]
fn minutes_and_seconds_and_an_empty_field_read_as_times() {
    let columns: [&[Field]; 2] = [&[Field::PreWait], &[Field::PostWait]];
    let typed = Cue {
        pre_wait: "1:30".to_owned().into(),
        post_wait: String::new().into(),
        ..cue(Kind::Audio)
    };
    assert_eq!(unread(&typed, &columns), None);
}

#[test]
fn a_duration_that_the_cue_does_not_hold_is_not_a_time_to_tell() {
    let columns: [&[Field]; 2] = [&[Field::Duration], &[]];
    let typed = Cue {
        duration: "abc".to_owned().into(),
        ..cue(Kind::Memo)
    };
    assert_eq!(unread(&typed, &columns), None);
}

#[test]
fn a_reason_fits_the_box_beside_the_longest_value_at_1024_pixels() {
    // The box of the Then setting at 1024x768 shows about 30 characters
    // of the longest value, "wait for GO", with its reason in parentheses
    let following = Cue {
        advance: Advance::Follow,
        ..cue(Kind::Audio)
    };
    [
        Field::PostWait.ignored(&following, false),
        Field::Then.ignored(&cue(Kind::Audio), true),
        Field::LoopEnd.ignored(&cue(Kind::Audio), false),
    ]
    .into_iter()
    .for_each(|reason| {
        let reason = reason.expect("a reason");
        assert!("wait for GO ".len() + reason.len() + 2 <= 30, "{reason}");
    });
}
