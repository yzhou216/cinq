// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of which failed devices are tried again, and what a retry keeps:
//! the devices that the threads of the devices open are replaced with some
//! that fail

use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use tempfile::TempDir;

use super::{
    ask,
    fakes::{Fake, open_on_both, overdue, patch, show, sounds},
    settle, silent_file, waiting,
};
use crate::back::{
    Audio, Channels, CueId,
    audio::{Wanted, mixer::Mixer, opener::Opener, opening::RETRY},
};

/// Makes the sound of cue 1 ready in `audio`, waiting five seconds at most
fn make_ready(audio: &mut Audio, wanted: &Wanted) {
    audio.preloader.want(vec![ask(wanted)]);
    waiting(|| {
        audio.preloader.collect();
        audio.preloader.ready(CueId(1)).then_some(())
    });
}

/// Returns audio on the show of Main on the device `a` and Sub on the
/// device `b`, whose devices have failed, with the sound of a cue made
/// ready, and the file of the sound
fn failed(fake: Fake) -> (Audio, TempDir) {
    let (directory, wanted) = silent_file();
    let mut audio = Audio::open(None);
    audio.opener = Opener::with(fake);
    audio.patch = patch(&show(Channels::MAIN, Channels::Stereo(0, 1)));
    ["a", "b"].into_iter().for_each(|key| {
        let fault = "there is no audio device".to_owned();
        audio.mixers.insert(Some(key.to_owned()), Err(fault));
    });
    make_ready(&mut audio, &wanted);
    overdue(&mut audio);
    (audio, directory)
}

#[test]
fn retrying_devices_that_are_not_there_keeps_the_sounds_made_ready() {
    let fake = Fake::default();
    let opened = Arc::clone(&fake.opened);
    let (mut audio, _directory) = failed(fake);
    assert!(audio.preloader.ready(CueId(1)), "the sound is made ready");
    audio.revived();
    settle(&mut audio);
    assert_eq!(opened.load(Ordering::SeqCst), 2, "both devices were tried");
    assert!(audio.preloader.ready(CueId(1)), "no run stopped for it");
}

#[test]
fn retrying_a_device_that_a_run_plays_on_makes_its_sound_again() {
    let (mut audio, mut mix_a, _) = open_on_both(Fake::default());
    let (directory, wanted) = silent_file();
    make_ready(&mut audio, &wanted);
    drop(directory);
    audio.patched(patch(&show(Channels::MAIN, Channels::Stereo(11, 12))));
    settle(&mut audio);
    assert!(
        !audio.preloader.ready(CueId(1)),
        "the run on device b ended"
    );
    assert_eq!(sounds(&mut mix_a), 1);
}

#[test]
fn device_that_offers_no_more_is_looked_at_ever_more_seldom() {
    let fake = Fake {
        offers: Some(2),
        ..Fake::default()
    };
    let looked = Arc::clone(&fake.looked);
    let (mut audio, ..) = open_on_both(fake);
    let key = Some("a".to_owned());
    let (a, _) = Mixer::bench(2, 4, 8000, 16);
    audio.mixers.insert(key.clone(), Ok(a));
    audio.patch = patch(&show(Channels::Stereo(2, 3), Channels::Stereo(0, 1)));
    audio.probes.opened(key.clone(), 2);
    let looks = |audio: &mut Audio, age: Duration| {
        audio.probes.aged(&key, age);
        audio.revived();
        settle(audio);
        looked.load(Ordering::SeqCst)
    };
    assert_eq!(looks(&mut audio, RETRY / 2), 0, "too soon after opening");
    assert_eq!(looks(&mut audio, RETRY), 1);
    assert_eq!(looks(&mut audio, RETRY), 1, "its wait is four seconds now");
    assert_eq!(looks(&mut audio, 2 * RETRY), 2);
}
