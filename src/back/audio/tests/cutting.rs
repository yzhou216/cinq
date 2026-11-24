// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of telling a sound that stops before the end of its file, as one
//! does whose file is cut short, on the mock backend of kira, which plays
//! ten milliseconds of sound each time a test lets it.  The decoder of a
//! file whose data lies short of its header fails at once, so that its sound
//! stops as it starts.

use std::thread;

use super::voice::{DECODING, audio, fired_promising, sounding, video};
use crate::back::{
    Cue, CueId,
    audio::{
        mixer::Mixer,
        patch::Key,
        voice::{Target, Voice},
    },
};

/// Plays `buffers` buffers of the sound of `cue` from a file that holds
/// `seconds` of sound where its header promises `promised`, and returns
/// whether the voice then tells that the sound stopped early
fn cut(cue: Cue, seconds: f32, promised: f32, buffers: usize) -> bool {
    let (directory, workspace) = fired_promising(cue, seconds, promised);
    let (mut manager, voice) = sounding(&directory, &workspace);
    (0..buffers).for_each(|_| {
        let backend = manager.backend_mut();
        backend.on_start_processing();
        backend.process();
    });
    let cue = workspace.cue(CueId(1)).expect("the cue is there");
    let clip = workspace
        .active_of(CueId(1))
        .and_then(|active| active.sound())
        .expect("the cue has sound");
    let ends = cue.sound_ends(clip).expect("the cue has sound");
    voice.cut_short(clip, ends, |_| true)
}

#[test]
fn sound_that_stops_with_more_than_a_second_of_its_file_ahead_is_cut_short() {
    assert!(cut(audio(), 0.5, 2.0, 80));
}

#[test]
fn sound_that_stops_within_a_second_of_its_end_is_not() {
    assert!(!cut(audio(), 0.2, 0.8, 80));
}

#[test]
fn sound_that_plays_on_is_not() {
    assert!(!cut(audio(), 5.0, 5.0, 20));
}

#[test]
fn sound_that_plays_to_the_end_of_its_file_is_not() {
    assert!(!cut(audio(), 0.5, 0.5, 80));
}

#[test]
fn a_loop_that_stops_is_cut_short_however_little_lies_ahead() {
    let looping = Cue {
        looping: true,
        ..audio()
    };
    assert!(cut(looping, 0.2, 0.5, 80));
}

#[test]
fn sound_track_that_ends_before_the_picture_is_not() {
    // The picture plays for ten seconds, which the sound track need not
    assert!(!cut(video(), 0.5, 0.5, 80));
}

#[test]
fn sound_on_a_device_that_does_not_play_is_not() {
    let (directory, workspace) = fired_promising(audio(), 0.5, 2.0);
    let active = workspace.active_of(CueId(1)).expect("the cue runs");
    let clip = active.sound().expect("the cue has sound");
    let ends = workspace
        .cue(CueId(1))
        .and_then(|cue| cue.sound_ends(clip))
        .expect("the cue has sound");
    let (mut mixer, mut mix) = Mixer::bench(2, 2, 8000, 16);
    let target = Target {
        key: Some("a".to_owned()),
        mixer: &mut mixer,
        gains: Box::new([[1.0, 0.0]; 2]),
    };
    let voice = Voice::open(
        vec![target],
        &directory.path().join("silence.wav"),
        active,
        clip,
    )
    .expect("the device takes the sound");
    thread::sleep(DECODING);
    (0..3).for_each(|_| mix.render(&mut [0.0_f32; 512]));
    let on = |key: &Key| key.as_deref() == Some("a");
    assert!(voice.cut_short(clip, ends, on));
    assert!(!voice.cut_short(clip, ends, |_| false));
}
