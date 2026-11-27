// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sound of one run of an audio or video cue
//!
//! The run plays once on each device that its outputs are on, each sound in
//! a kira manager of its own, whose renderer the mixer of the device
//! drives, as [`super::feed`] tells.  The clock of the run follows the
//! sound on the device of the first output that the cue sends to.  Sounds
//! on two devices keep the clocks of their devices, which drift apart
//! unless the devices share a clock.

mod follow;
mod hearing;
mod place;

use std::{path::Path, time::Duration};

use kira::Tween;

use super::{mixer::Mixers, preload::Preloaded};
use crate::back::{Active, Clip, audio::patch::Key};
use place::{Play, circles, each, opened, placed};
pub(in crate::back::audio) use place::{Target, clipped};

pub(in crate::back::audio) struct Voice {
    pub(super) run: u32,
    /// Sound of the run on each device that it plays on
    plays: Vec<Play>,
    /// Level last sent to the sounds
    level: f32,
    /// Pause state last sent to the sounds
    paused: bool,
    /// Whether the sounds may still have the loop of their clip, which
    /// [`Voice::followed`] takes away once
    looped: bool,
}

impl Voice {
    /// Opens the file of a run on each target and starts its sound at the
    /// level of the run, as far into its clip as the run has played, since
    /// the clock may have passed the start by part of a tick.  The sound
    /// streams from the file, so long files start as quickly as short ones,
    /// but opening the file can wait on the disk, which [`Voice::start`]
    /// avoids.  Returns why the sound cannot start, as when its file has
    /// gone since it loaded or a mixer has more to do than it can take.
    pub(in crate::back::audio) fn open(
        mut targets: Vec<Target<'_>>,
        file: &Path,
        active: &Active,
        clip: Clip,
    ) -> Result<Self, String> {
        let plays = each(&mut targets, |target| opened(target, file, active, clip))?;
        Ok(Self::with(plays, active, circles(clip, active.played())))
    }

    /// Starts a sound made ready ahead on the first target, which waits at
    /// the start of its clip, at the level that the run has now, and opens
    /// the file on the others.  A run that the clock started between two
    /// ticks, as a pre-wait or a chain does, has played part of a tick
    /// already, and its sound starts late by that part: kira keeps what it
    /// has decoded ahead when a playing sound seeks, so seeking would play
    /// the start twice.  Returns why the sound cannot start, as when a
    /// mixer has more to do than it can take.
    pub(super) fn start(
        mut targets: Vec<Target<'_>>,
        preloaded: Preloaded,
        file: &Path,
        active: &Active,
        clip: Clip,
    ) -> Result<Self, String> {
        let now = Tween {
            duration: Duration::ZERO,
            ..Tween::default()
        };
        let mut ready = Some(preloaded);
        let plays = each(&mut targets, |target| match ready.take() {
            Some(data) => placed(target, data).map(|mut play| {
                play.sound.set_volume(active.level, now);
                play
            }),
            None => opened(target, file, active, clip),
        })?;
        // The sound made ready ahead has the loop of its clip, whether or not
        // the clip has been let go on since
        Ok(Self::with(plays, active, clip.looping.is_some()))
    }

    #[cfg(test)]
    pub(in crate::back::audio) fn of(
        sound: kira::sound::streaming::StreamingSoundHandle<kira::sound::FromFileError>,
        active: &Active,
    ) -> Self {
        let looped = active
            .sound()
            .is_some_and(|clip| circles(clip, active.played()));
        Self::with(vec![Play { sound, place: None }], active, looped)
    }

    fn with(plays: Vec<Play>, active: &Active, looped: bool) -> Self {
        Self {
            run: active.run,
            plays,
            level: active.level,
            paused: false,
            looped,
        }
    }

    #[cfg(test)]
    pub(in crate::back::audio) fn looped(&self) -> bool {
        self.looped
    }

    /// Returns where in its file the first sound is
    #[cfg(test)]
    pub(in crate::back::audio) fn position(&self) -> Option<f64> {
        self.plays.first().map(|play| play.sound.position())
    }

    /// Returns the devices that the run plays on, in order
    pub(in crate::back::audio) fn keys(&self) -> Vec<&Key> {
        self.plays
            .iter()
            .filter_map(|play| play.place.as_ref().map(|place| &place.key))
            .collect()
    }

    /// Stops the sounds, which their mixers fade out as quickly as kira's
    /// default fade, short enough to sound immediate and long enough to
    /// avoid a click
    pub(in crate::back::audio) fn stop(self, mixers: &mut Mixers) {
        self.plays
            .into_iter()
            .filter_map(|play| play.place)
            .for_each(|place| {
                if let Some(Ok(mixer)) = mixers.get_mut(&place.key) {
                    mixer.remove(place.id);
                }
            });
    }
}
