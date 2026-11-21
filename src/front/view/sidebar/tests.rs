// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the sidebar: the name of a list, the entry of a running cue, its
//! width, and the warnings that it lists

use unicode_width::UnicodeWidthStr;

use super::{LEAST, SIDEBAR, label, limited, running::entry, width};
use crate::back::{Command, Cue, CueId, CueList, Kind, Workspace, loaded, sound};

#[test]
fn a_long_list_name_gives_way_to_the_count_of_its_cues() {
    assert_eq!(label("Main", 12, 20), "Main (12)");
    assert_eq!(label("Act one, scene two", 7, 12), "Act one\u{2026} (7)");
    assert!(label(&"x".repeat(60), 120, 16).ends_with(" (120)"));
}

#[test]
fn a_running_cue_keeps_three_lines_and_loses_its_phase_words_before_its_level() {
    let cue = Cue::new(CueId(1), "1", Kind::Audio, "Rain")
        .file("rain.ogg")
        .looping(true)
        .build();
    let running = Workspace::new(vec![CueList::new("Main", vec![cue])])
        .apply(loaded(1, "rain.ogg", sound(10.0)))
        .apply(Command::Go { list: 0 })
        .apply(Command::Tick(1.0))
        .apply(Command::Panic)
        .apply(Command::Tick(0.1));
    let active = &running.active()[0];
    let cue = running.cue(CueId(1)).expect("the cue is there");
    assert_eq!(entry(active, cue, 40).0, "1 Rain");
    let (_, whole) = entry(active, cue, 60);
    assert!(whole.starts_with("looping, ") && whole.ends_with(" dB, fading"));
    let (_, how) = entry(active, cue, 20);
    assert!(how.width() <= 20 && how.ends_with('\u{2026}'));
    assert!(how.starts_with("looping, ") && how.contains(" dB"));
}

#[test]
fn the_sidebar_takes_a_quarter_of_the_window_between_its_least_and_its_most() {
    assert_eq!(width(512.0), LEAST);
    assert_eq!(width(1024.0), 256.0);
    assert_eq!(width(4000.0), SIDEBAR);
}

#[test]
fn the_sidebar_lists_three_warnings_and_counts_the_rest() {
    assert_eq!(limited(vec![1, 2, 3]), (vec![1, 2, 3], 0));
    assert_eq!(limited(vec![1, 2, 3, 4, 5, 6, 7, 8]), (vec![1, 2, 3], 5));
    assert_eq!(limited(Vec::<u8>::new()), (Vec::new(), 0));
}
