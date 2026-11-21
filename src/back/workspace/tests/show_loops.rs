// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of how show files hold loop sections and devamp cues

use super::{show::read_list, *};

/// Returns a show in `/show` of a vamp that loops from 2 to 6 seconds, a
/// devamp cue for it, and a vamp that does not loop but keeps its section
fn vamping() -> Workspace {
    workspace(vec![
        vamp(1),
        devamp_cue(2, 1),
        Cue {
            looping: false,
            ..vamp(3)
        },
    ])
    .directory("/show")
}

#[test]
fn show_file_holds_where_the_loop_of_a_cue_starts_and_ends() {
    let text = shown(&vamping());
    assert!(
        text.contains("file = \"sound.ogg\"\nloop = true\nloop_start = 2.0\nloop_end = 6.0\n"),
        "{text}"
    );
    let read = read_back(&text);
    let cue = read.cue(CueId(1)).expect("the vamp is there");
    assert_eq!(
        (cue.loop_start.time(), cue.loop_end.time()),
        (Some(2.0), Some(6.0))
    );
    assert!(cue.looping);
}

#[test]
fn show_file_keeps_the_loop_of_a_cue_that_does_not_loop() {
    let read = read_back(&shown(&vamping()));
    let cue = read.cue(CueId(3)).expect("the vamp is there");
    assert!(!cue.looping);
    assert_eq!(
        (cue.loop_start.time(), cue.loop_end.time()),
        (Some(2.0), Some(6.0))
    );
}

#[test]
fn show_file_leaves_out_a_loop_that_a_cue_does_not_set() {
    let text = shown(&workspace(vec![Cue {
        looping: true,
        ..audio(1)
    }]));
    assert!(text.contains("loop = true\n"), "{text}");
    assert!(!text.contains("loop_"), "{text}");
}

#[test]
fn show_file_holds_a_devamp_cue_with_its_target() {
    let text = shown(&vamping());
    assert!(
        text.contains("kind = \"devamp\"\nname = \"Control\"\ntarget = 1\n"),
        "{text}"
    );
    let read = read_back(&text);
    let devamp = read.cue(CueId(2)).expect("the devamp cue is there");
    assert_eq!((devamp.kind, devamp.target), (Kind::Devamp, Some(CueId(1))));
}

#[test]
fn show_file_leaves_out_a_loop_for_a_kind_that_plays_no_file() {
    let read = read_list(
        "[[list.cue]]\nid = 1\nkind = \"memo\"\nloop_start = 2.0\nloop_end = 6.0\n\n\
         [[list.cue]]\nid = 2\nkind = \"picture\"\nfile = \"slide.png\"\nloop_start = 2.0\n\n\
         [[list.cue]]\nid = 3\nkind = \"devamp\"\ntarget = 4\nloop_end = 6.0\n\n\
         [[list.cue]]\nid = 4\nkind = \"audio\"\nfile = \"sound.ogg\"\nloop_start = 2.0\n",
    )
    .expect("the show reads");
    let text = shown(&read);
    assert_eq!(text.matches("loop_").count(), 1, "{text}");
    assert!(text.contains("loop_start = 2.0\n"), "{text}");
}
