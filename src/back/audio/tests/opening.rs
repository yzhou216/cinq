// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of which devices open again as the outputs change, on mixers
//! without devices: a device that cinq opens by an ID that no host knows
//! fails at once, which shows where a device was opened again

use std::time::Instant;

use super::voice::opened;
use crate::back::{
    Audio, Channels, Command, CueId, Device, OutputEdit, OutputId, Workspace,
    audio::{
        mixer::{Mix, Mixer},
        opening::RETRY,
        patch::Patch,
    },
};

const MAIN: OutputId = OutputId(1);
const SUB: OutputId = OutputId(2);

fn named(id: &str) -> Device {
    Device::Named {
        id: id.to_owned(),
        name: id.to_owned(),
    }
}

/// Returns a show of Main on the device `a` and Sub on the device `b`, each
/// on the channels that `channels` gives it
fn show(main: Channels, sub: Channels) -> Workspace {
    Workspace::default()
        .apply(Command::EditOutput(MAIN, OutputEdit::Device(named("a"))))
        .apply(Command::EditOutput(MAIN, OutputEdit::Channels(main)))
        .apply(Command::AddOutput { at: 1 })
        .apply(Command::EditOutput(SUB, OutputEdit::Device(named("b"))))
        .apply(Command::EditOutput(SUB, OutputEdit::Channels(sub)))
}

fn patch(workspace: &Workspace) -> Patch {
    Patch::of(workspace.outputs(), &None)
}

/// Returns audio that has opened the devices `a` and `b` of two channels
/// as the show asks, each with its mix, and the sound of a cue on each
/// device and one on both
fn open_on_both() -> (Audio, Mix, Mix) {
    let mut audio = Audio::open(None);
    audio.patch = patch(&show(Channels::MAIN, Channels::Stereo(0, 1)));
    let (mut a, mix_a) = Mixer::bench(2, 2, 8000, 16);
    let (mut b, mix_b) = Mixer::bench(2, 2, 8000, 16);
    let on_a = opened(vec![("a", &mut a)]).expect("the sound starts");
    let on_b = opened(vec![("b", &mut b)]).expect("the sound starts");
    let on_both = opened(vec![("a", &mut a), ("b", &mut b)]).expect("the sound starts");
    audio.voices.insert(CueId(1), on_a);
    audio.voices.insert(CueId(2), on_b);
    audio.voices.insert(CueId(3), on_both);
    audio.mixers.insert(Some("a".to_owned()), Ok(a));
    audio.mixers.insert(Some("b".to_owned()), Ok(b));
    (audio, mix_a, mix_b)
}

/// Renders a mix until its sounds that went have faded out, and returns
/// how many sounds it holds
fn sounds(mix: &mut Mix) -> usize {
    mix.render(&mut [0.0_f32; 512]);
    mix.sounds()
}

fn is_open(audio: &Audio, device: &str) -> bool {
    matches!(audio.mixers.get(&Some(device.to_owned())), Some(Ok(_)))
}

fn cues(audio: &Audio) -> Vec<u32> {
    let mut cues: Vec<u32> = audio.voices.keys().map(|cue| cue.0).collect();
    cues.sort_unstable();
    cues
}

#[test]
fn outputs_that_still_fit_their_devices_keep_them_open_and_their_sounds() {
    let (mut audio, mut mix_a, mut mix_b) = open_on_both();
    audio.patched(patch(&show(Channels::Stereo(0, 1), Channels::Mono(1))));
    assert!(is_open(&audio, "a") && is_open(&audio, "b"));
    assert_eq!(cues(&audio), [1, 2, 3]);
    assert_eq!((sounds(&mut mix_a), sounds(&mut mix_b)), (2, 2));
}

#[test]
fn output_that_needs_more_channels_opens_its_device_again_and_no_other() {
    let (mut audio, mut mix_a, mut mix_b) = open_on_both();
    // Channels 12 and 13 ask a device of two for more, as typing 12-13
    // would, which opens only the device of Sub
    audio.patched(patch(&show(Channels::MAIN, Channels::Stereo(11, 12))));
    assert!(is_open(&audio, "a"));
    assert!(matches!(
        audio.mixers.get(&Some("b".to_owned())),
        Some(Err(_))
    ));
    assert_eq!(cues(&audio), [1], "only the sound on device a plays on");
    assert_eq!(sounds(&mut mix_a), 1, "the sound of the other cues goes");
    assert_eq!(sounds(&mut mix_b), 0);
}

#[test]
fn output_that_moves_to_another_device_closes_the_one_it_left() {
    let (mut audio, mut mix_a, _) = open_on_both();
    let workspace = show(Channels::MAIN, Channels::Stereo(0, 1))
        .apply(Command::EditOutput(SUB, OutputEdit::Device(named("a"))));
    audio.patched(patch(&workspace));
    assert!(is_open(&audio, "a"));
    assert!(!audio.mixers.contains_key(&Some("b".to_owned())));
    assert_eq!(cues(&audio), [1]);
    assert_eq!(sounds(&mut mix_a), 1);
}

#[test]
fn opening_devices_sets_the_time_of_their_last_try_for_the_retry_to_wait() {
    let (mut audio, ..) = open_on_both();
    audio.tried = Instant::now()
        .checked_sub(2 * RETRY)
        .expect("the clock is past two retries");
    audio.patched(patch(&show(Channels::MAIN, Channels::Stereo(11, 12))));
    assert!(audio.tried.elapsed() < RETRY);
}

#[test]
fn device_that_opened_with_fewer_channels_opens_again_once_it_offers_more() {
    let wide = show(Channels::Stereo(2, 3), Channels::Stereo(0, 1));
    let short = || {
        let (mut audio, ..) = open_on_both();
        let (a, _) = Mixer::bench(2, 4, 8000, 16);
        audio.mixers.insert(Some("a".to_owned()), Ok(a));
        audio.patch = patch(&wide);
        audio.tried = Instant::now()
            .checked_sub(2 * RETRY)
            .expect("the clock is past two retries");
        audio
    };
    let mut still = short();
    still.revived_by(|_| Some(2));
    assert!(
        is_open(&still, "a"),
        "it offers no more than it opened with"
    );
    let mut gone = short();
    gone.revived_by(|_| None);
    assert!(is_open(&gone, "a"), "it is not there to look at");
    let mut more = short();
    more.revived_by(|_| Some(4));
    assert!(!is_open(&more, "a"), "it offers all four now");
}
