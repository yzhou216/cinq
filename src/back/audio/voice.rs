// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sound of one run of an audio or video cue
//!
//! The sound plays in a kira manager of its own, whose renderer the mixer
//! of the device drives, as [`super::feed`] tells.

use std::{path::Path, time::Duration};

use kira::{
    AudioManager, AudioManagerSettings, Capacities, Tween,
    sound::{
        FromFileError, PlaybackState, Region, SoundData,
        streaming::{StreamingSoundData, StreamingSoundHandle},
    },
    track::MainTrackBuilder,
};

use super::{
    feed::Feed,
    mixer::{Gains, Mixer},
    preload::Preloaded,
};
use crate::back::{Active, Clip, Command, TICK};

/// Time a sound takes to reach a level the workspace asks for, which
/// outlasts a tick so that fades move smoothly from one tick to the next
const GLIDE: Duration = TICK.saturating_mul(2);

pub(super) struct Voice {
    pub(super) run: u32,
    sound: StreamingSoundHandle<FromFileError>,
    /// Place of the sound in the mixer, which a test that plays the sound
    /// in a manager of its own has none of
    place: Option<Place>,
    /// Level last sent to the sound
    level: f32,
    /// Pause state last sent to the sound
    paused: bool,
}

struct Place {
    /// Identity of the sound in the mixer
    id: u64,
    /// Manager of the sound alone, which hands the sound its commands
    _manager: AudioManager<Feed>,
}

impl Voice {
    /// Opens the file of a run and starts its sound at the level of the
    /// run, as far into its clip as the run has played, since the clock may
    /// have passed the start by part of a tick.  The sound streams from the
    /// file, so long files start as quickly as short ones, but opening the
    /// file can wait on the disk, which [`Voice::start`] avoids.  Returns
    /// why the sound cannot start, as when its file has gone since it
    /// loaded or the mixer has more to do than it can take.
    pub(super) fn open(
        mixer: &mut Mixer,
        gains: Gains,
        file: &Path,
        active: &Active,
        clip: Clip,
    ) -> Result<Self, String> {
        let data = StreamingSoundData::from_file(file).map_err(super::reason)?;
        let data = clipped(data, clip, clip.position(active.played()), active.level);
        let (sound, place) = placed(mixer, gains, data)?;
        Ok(Self::with(sound, Some(place), active))
    }

    /// Starts a sound made ready ahead, which waits at the start of its
    /// clip, at the level that the run has now.  A run that the clock
    /// started between two ticks, as a pre-wait or a chain does, has played
    /// part of a tick already, and its sound starts late by that part:
    /// kira keeps what it has decoded ahead when a playing sound seeks, so
    /// seeking would play the start twice.  Returns why the sound cannot
    /// start, as when the mixer has more to do than it can take.
    pub(super) fn start(
        mixer: &mut Mixer,
        gains: Gains,
        preloaded: Preloaded,
        active: &Active,
    ) -> Result<Self, String> {
        let (mut sound, place) = placed(mixer, gains, preloaded)?;
        let now = Tween {
            duration: Duration::ZERO,
            ..Tween::default()
        };
        sound.set_volume(active.level, now);
        Ok(Self::with(sound, Some(place), active))
    }

    #[cfg(test)]
    pub(super) fn of(sound: StreamingSoundHandle<FromFileError>, active: &Active) -> Self {
        Self::with(sound, None, active)
    }

    fn with(
        sound: StreamingSoundHandle<FromFileError>,
        place: Option<Place>,
        active: &Active,
    ) -> Self {
        Self {
            run: active.run,
            sound,
            place,
            level: active.level,
            paused: false,
        }
    }

    /// Sends the sound any change in the level or pause state of its run
    pub(super) fn followed(mut self, active: &Active) -> Self {
        match (self.paused, active.paused) {
            (false, true) => self.sound.pause(Tween::default()),
            (true, false) => self.sound.resume(Tween::default()),
            _ => {}
        }
        if self.level != active.level {
            let glide = Tween {
                duration: GLIDE,
                ..Tween::default()
            };
            self.sound.set_volume(active.level, glide);
        }
        Self {
            level: active.level,
            paused: active.paused,
            ..self
        }
    }

    /// Returns the command that tells the workspace how far the sound of the
    /// run has played, as long as the sound is still playing.  The
    /// position of the sound is where the audio device took it at the start
    /// of its last buffer, which kira shares without waiting on the device.
    pub(super) fn heard(&self, active: &Active) -> Option<Command> {
        let clip = active.sound()?;
        (self.sound.state() != PlaybackState::Stopped).then(|| Command::Heard {
            cue: active.id,
            run: active.run,
            played: clip.played_at(self.sound.position(), active.played()),
        })
    }

    /// Stops the sound, which the mixer fades out as quickly as kira's
    /// default fade, short enough to sound immediate and long enough to
    /// avoid a click
    pub(super) fn stop(self, mixer: &mut Mixer) {
        if let Some(place) = self.place {
            mixer.remove(place.id);
        }
    }
}

/// Plays `data` in a manager of its own, whose renderer `mixer` takes and
/// mixes at `gains`
fn placed<D>(
    mixer: &mut Mixer,
    gains: Gains,
    data: D,
) -> Result<(StreamingSoundHandle<FromFileError>, Place), String>
where
    D: SoundData<Handle = StreamingSoundHandle<FromFileError>>,
    D::Error: std::fmt::Display,
{
    let Ok(mut manager) = AudioManager::<Feed>::new(AudioManagerSettings {
        capacities: Capacities {
            sub_track_capacity: 1,
            send_track_capacity: 1,
            clock_capacity: 1,
            modulator_capacity: 1,
            listener_capacity: 1,
        },
        main_track_builder: MainTrackBuilder::new().sound_capacity(1),
        backend_settings: mixer.sample_rate(),
        ..AudioManagerSettings::default()
    });
    let sound = manager.play(data).map_err(|error| error.to_string())?;
    let renderer = manager
        .backend_mut()
        .take()
        .expect("kira hands its renderer to the backend as the manager opens");
    let id = mixer.add(renderer, gains)?;
    Ok((
        sound,
        Place {
            id,
            _manager: manager,
        },
    ))
}

/// Returns `data` set to play `clip`, from `position` in the file and at
/// `level`.
pub(super) fn clipped(
    data: StreamingSoundData<FromFileError>,
    clip: Clip,
    position: f64,
    level: f32,
) -> StreamingSoundData<FromFileError> {
    // Cut from the start of the file, so that times in the cut are times in
    // the file, which kira seeks to directly.  A cut is left out where it
    // would end past the last frame of the file.
    let cut = (clip.end < data.duration().as_secs_f64()).then(|| Region::from(0.0..clip.end));
    let looped = clip.looping.then(|| Region::from(clip.start..));
    data.slice(cut)
        .loop_region(looped)
        .start_position(position)
        .volume(level)
}
