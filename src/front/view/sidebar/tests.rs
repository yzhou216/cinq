// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the sidebar: the name of a list, the entry of a running cue, its
//! width, and the warnings that it lists

use unicode_width::UnicodeWidthStr;

use super::{
    LEAST, SIDEBAR,
    devices::{missing, named},
    label, limited,
    running::entry,
    width,
};
use crate::{
    back::{Command, Cue, CueId, CueList, Device, Fault, Kind, OutputId, Workspace, loaded, sound},
    front::App,
};

#[test]
fn a_long_list_name_gives_way_to_the_count_of_its_cues() {
    assert_eq!(label("Main", 12, 20), "Main (12)");
    assert_eq!(label("Act one, scene two", 7, 12), "Act one\u{2026} (7)");
    assert!(label(&"x".repeat(60), 120, 16).ends_with(" (120)"));
}

/// Returns a list of the looping sound Rain, which has run for a second
fn raining() -> Workspace {
    let cue = Cue::new(CueId(1), "1", Kind::Audio, "Rain")
        .file("rain.ogg")
        .looping(true)
        .build();
    Workspace::new(vec![CueList::new("Main", vec![cue])])
        .apply(loaded(1, "rain.ogg", sound(10.0)))
        .apply(Command::Go { list: 0 })
        .apply(Command::Tick(1.0))
}

#[test]
fn a_running_cue_keeps_three_lines_and_loses_its_phase_words_before_its_level() {
    let running = raining().apply(Command::Panic).apply(Command::Tick(0.1));
    let active = &running.active()[0];
    let cue = running.cue(CueId(1)).expect("the cue is there");
    assert_eq!(entry(active, cue, None, 40, false).0, "1 Rain");
    let (_, whole) = entry(active, cue, None, 60, false);
    assert!(whole.starts_with("looping, ") && whole.ends_with(" dB, fading"));
    let (_, how) = entry(active, cue, None, 20, false);
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
    assert_eq!(
        limited(vec![1, 2, 3, 4]),
        (vec![1, 2, 3, 4], 0),
        "a line for one warning takes the room of the warning"
    );
    assert_eq!(limited(vec![1, 2, 3, 4, 5]), (vec![1, 2, 3], 2));
    assert_eq!(limited(vec![1, 2, 3, 4, 5, 6, 7, 8]), (vec![1, 2, 3], 5));
    assert_eq!(limited(Vec::<u8>::new()), (Vec::new(), 0));
}

#[test]
fn a_running_cue_whose_outputs_all_failed_says_so_in_place_of_how_it_runs() {
    let running = raining();
    let active = &running.active()[0];
    let cue = running.cue(CueId(1)).expect("the cue is there");
    assert_eq!(
        entry(active, cue, None, 40, true).1,
        "silent: output failed"
    );
    assert_eq!(entry(active, cue, None, 12, true).1.width(), 12);
}

#[test]
fn a_warning_about_the_default_device_names_the_id_that_the_list_chooses() {
    let mut app = App::default();
    app.config.sound.device = Some("alsa:nonexistent-main".to_owned());
    assert_eq!(
        named(&app, &Device::Default),
        "Default device (alsa:nonexistent-main)"
    );
    let card = Device::Named {
        id: "alsa:front:CARD=PCH,DEV=0".to_owned(),
        name: String::new(),
    };
    assert_eq!(named(&app, &card), "alsa:front:CARD=PCH,DEV=0");
}

#[test]
fn the_default_device_list_is_lost_while_an_output_on_the_default_fails() {
    let app = App::default();
    assert!(!missing(&app));
    let failed = App {
        audio_problems: vec![(OutputId(1), Fault::Missing("gone".to_owned()))],
        ..App::default()
    };
    assert!(missing(&failed), "Main is on the default device");
}

#[test]
fn a_running_fade_names_its_target_where_the_name_fits_whole() {
    let rain = Cue::new(CueId(1), "1", Kind::Audio, "Rain")
        .file("rain.ogg")
        .build();
    let fade = Cue::new(CueId(2), "2", Kind::Fade, "Duck")
        .target(CueId(1))
        .duration(5.0)
        .build();
    let running = Workspace::new(vec![CueList::new("Main", vec![rain.clone(), fade])])
        .apply(Command::Go { list: 0 })
        .apply(Command::Go { list: 0 })
        .apply(Command::Tick(1.0));
    let active = running.active_of(CueId(2)).expect("the fade runs");
    let cue = running.cue(CueId(2)).expect("the cue is there");
    let how = |target, columns| entry(active, cue, target, columns, false).1;
    assert_eq!(how(Some(&rain), 40), "0:04.0 left, fades 1");
    assert_eq!(how(Some(&rain), 12), "0:04.0 left");
    assert_eq!(how(None, 40), "0:04.0 left");
}
