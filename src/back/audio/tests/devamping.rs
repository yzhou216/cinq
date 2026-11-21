// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the sound of a cue that loops part of its clip, and of a devamp
//! cue letting it go on, through kira itself: its streaming decoder runs
//! ahead of the sound, which is what the voice of a run has to allow for.
//! The sounds play in the mock backend of kira at 48000 frames a second,
//! which plays one tick of sound each time a test lets it.

use std::{fs, thread, time::Duration};

use kira::{
    AudioManager, AudioManagerSettings,
    backend::mock::{MockBackend, MockBackendSettings},
    sound::streaming::StreamingSoundData,
};
use tempfile::TempDir;

use super::silence_at;
use crate::back::{
    Command, Cue, CueId, CueList, Kind, Media, Workspace,
    audio::voice::{Voice, clipped},
    workspace::tests::{GO, devamp_cue, loaded},
};

pub(super) const RATE: u32 = 48_000;

/// Seconds that the workspace moves on in each step of a test
const TICK: f64 = 0.025;

/// Frames that the backend plays in each step, which is one tick
const FRAMES: usize = 1200;

/// Time that the thread of a sound gets to decode in each step, far more
/// than it needs
const DECODING: Duration = Duration::from_millis(3);

/// Returns a directory with a file of `seconds` of silence at `rate`, and
/// the workspace in which GO has fired an audio cue that loops from `from`
/// to `to` in that file, with a devamp cue after it
pub(super) fn vamp(
    rate: u32,
    seconds: f32,
    (from, to): (f32, f32),
    pre_wait: f32,
) -> (TempDir, Workspace) {
    let directory = TempDir::new().expect("a temporary directory can be made");
    fs::write(
        directory.path().join("silence.wav"),
        silence_at(seconds, rate),
    )
    .expect("the file can be written");
    let cue = Cue {
        pre_wait: pre_wait.into(),
        looping: true,
        loop_start: from.into(),
        loop_end: to.into(),
        ..Cue::new(CueId(1), "1", Kind::Audio, "Vamp")
            .file("silence.wav")
            .build()
    };
    let media = Media::Loaded {
        duration: seconds,
        sound: Some(seconds),
    };
    let workspace = Workspace::new(vec![CueList::new("Main", vec![cue, devamp_cue(2, 1)])])
        .directory(directory.path())
        .apply(loaded(1, "silence.wav", media))
        .apply(GO);
    (directory, workspace)
}

/// What a test sees after a step
#[derive(Clone, Copy)]
struct Seen {
    /// Where in its file the sound is
    position: f64,
    /// Seconds that the run has played, while it runs
    played: Option<f64>,
    /// Whether the sound still plays
    heard: bool,
}

/// Run and its sound in a mock backend
struct Rig {
    manager: AudioManager<MockBackend>,
    workspace: Workspace,
    voice: Voice,
}

impl Rig {
    /// Starts the sound of cue 1, which runs in `workspace`, as one opened
    /// where the run is
    fn start(directory: &TempDir, workspace: Workspace) -> Self {
        let mut manager = AudioManager::<MockBackend>::new(AudioManagerSettings {
            backend_settings: MockBackendSettings { sample_rate: RATE },
            internal_buffer_size: FRAMES,
            ..AudioManagerSettings::default()
        })
        .expect("the mock backend starts");
        let active = workspace.active_of(CueId(1)).expect("the cue runs");
        let clip = active.sound().expect("the cue has sound");
        let data = StreamingSoundData::from_file(directory.path().join("silence.wav"))
            .expect("the sound opens");
        let sound = manager
            .play(clipped(data, clip, active.played(), 0.0))
            .expect("the sound plays");
        let voice = Voice::of(sound, active);
        thread::sleep(Duration::from_millis(300));
        Self {
            manager,
            workspace,
            voice,
        }
    }

    /// Moves the clock and the sound on one tick and syncs the voice as the
    /// audio does
    fn stepped(self) -> (Self, Seen) {
        let Self {
            mut manager,
            workspace,
            voice,
        } = self;
        let ticked = workspace.apply(Command::Tick(TICK));
        let backend = manager.backend_mut();
        backend.on_start_processing();
        backend.process();
        thread::sleep(DECODING);
        let active = ticked.active_of(CueId(1)).copied();
        let voice = active.iter().fold(voice, Voice::followed);
        let heard = active
            .as_ref()
            .and_then(|active| voice.heard(active, |_| true));
        let workspace = heard.clone().into_iter().fold(ticked, Workspace::apply);
        let seen = Seen {
            position: voice.position().expect("the voice has a sound"),
            played: workspace.active_of(CueId(1)).map(|active| active.played()),
            heard: heard.is_some(),
        };
        (
            Self {
                manager,
                workspace,
                voice,
            },
            seen,
        )
    }

    /// Steps `steps` times, and before step `index` lets `act` change the
    /// workspace
    fn run(self, steps: usize, act: impl Fn(usize, Workspace) -> Workspace) -> (Self, Vec<Seen>) {
        (0..steps).fold((self, Vec::new()), |(rig, seen), index| {
            let (rig, now) = Self {
                workspace: act(index, rig.workspace),
                ..rig
            }
            .stepped();
            (rig, seen.into_iter().chain([now]).collect())
        })
    }
}

/// Returns how many times the sound went back, as it does at the end of a
/// pass of a loop
fn wraps(seen: &[Seen]) -> usize {
    seen.windows(2)
        .filter(|pair| pair[1].position < pair[0].position - 0.5)
        .count()
}

/// Returns by how much the clock of the run is off the time that the sound
/// has played, at most, over the steps
fn drift(seen: &[Seen]) -> f64 {
    seen.iter()
        .enumerate()
        .filter_map(|(index, seen)| Some((seen.played? - (index + 1) as f64 * TICK).abs()))
        .fold(0.0, f64::max)
}

#[test]
fn a_loop_section_goes_round_its_loop_and_never_back_before_its_start() {
    let (directory, workspace) = vamp(RATE, 2.5, (0.5, 1.5), 0.0);
    let (_, seen) = Rig::start(&directory, workspace).run(240, |_, workspace| workspace);
    assert_eq!(wraps(&seen), 5, "one at the end of each pass but the lead");
    let inside = seen.iter().skip_while(|seen| seen.position < 0.5);
    assert!(inside.clone().all(|seen| seen.position >= 0.5 - 1e-6));
    assert!(seen.iter().all(|seen| seen.position < 1.5 + 1e-6));
    assert!(seen.iter().all(|seen| seen.heard), "the sound never ends");
    let off = drift(&seen);
    assert!(
        off < 0.06,
        "the clock follows the sound across passes: {off}"
    );
}

#[test]
fn a_devamped_run_leaves_its_loop_at_the_planned_pass_end_and_plays_to_its_end() {
    let (directory, workspace) = vamp(RATE, 2.5, (0.5, 1.5), 0.0);
    // The pass that ends at 1.5 seconds is less than the notice away at 1.05
    // seconds, so the run leaves at the end of the pass after it, 2.5
    // seconds in, and its last pass begins at 1.5 seconds
    let (rig, seen) = Rig::start(&directory, workspace).run(160, |index, workspace| {
        if index == 42 {
            workspace.apply(GO)
        } else {
            workspace
        }
    });
    assert_eq!(wraps(&seen), 1, "it goes round once, at 1.5 seconds");
    let left = seen
        .iter()
        .position(|seen| seen.position > 1.5 + 1e-3)
        .map(|index| (index + 1) as f64 * TICK)
        .expect("the sound leaves its loop");
    assert!((2.5..=2.6).contains(&left), "{left}");
    let ends = seen
        .iter()
        .position(|seen| !seen.heard)
        .map(|index| (index + 1) as f64 * TICK)
        .expect("the sound ends");
    assert!((3.5..=3.65).contains(&ends), "{ends}");
    assert!(seen.iter().all(|seen| seen.position <= 2.5 + 1e-6));
    assert!(rig.workspace.active_of(CueId(1)).is_none(), "the run ends");
    let off = drift(&seen);
    assert!(
        off < 0.06,
        "the clock follows the sound out of its loop: {off}"
    );
    assert!(!rig.voice.looped());
}

#[test]
fn a_loop_that_ends_with_its_clip_ends_the_run_at_the_pass_end() {
    let (directory, workspace) = vamp(RATE, 2.5, (0.5, 2.5), 0.0);
    let (rig, seen) = Rig::start(&directory, workspace).run(140, |index, workspace| {
        if index == 20 {
            workspace.apply(GO)
        } else {
            workspace
        }
    });
    assert_eq!(wraps(&seen), 0, "the first pass is the last");
    assert!(seen.iter().all(|seen| seen.position <= 2.5 + 1e-6));
    assert!(rig.workspace.active_of(CueId(1)).is_none(), "the run ends");
    assert!(!seen.last().expect("the test steps").heard);
}
