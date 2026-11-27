// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Where a fired cue is in time: its phase, its progress, and the moving
//! of its clock

use super::{Active, Effect, Phase};

/// Share of the way to the time that its sound has reached that a run
/// moves its clock, once for each sync of the sound output
const HEARING: f64 = 0.1;

impl Active {
    pub fn phase(&self) -> Phase {
        match self.elapsed {
            elapsed if elapsed < 0.0 => Phase::Scheduled {
                starts_in: -elapsed,
            },
            elapsed if elapsed < self.pre_wait => Phase::PreWait {
                remaining: self.pre_wait - elapsed,
            },
            _ if self.effect == Effect::Picture => Phase::Showing,
            _ if self.loops() => Phase::Looping,
            _ if self.endless() => Phase::Endless,
            elapsed => Phase::Running {
                remaining: (self.pre_wait + self.length - elapsed).max(0.0),
            },
        }
    }

    /// Returns how much of the action has played, from zero to one.  For an
    /// audio or video cue this is how far its sound has reached through its
    /// clip: for a clip that loops it returns to the start of the loop with
    /// each pass, and once a devamp cue has let the clip go on it runs to the
    /// end of the clip without going back.
    pub fn progress(&self) -> f32 {
        let progress = match (self.phase(), self.effect.clip()) {
            (Phase::Scheduled { .. } | Phase::PreWait { .. }, _) => 0.0,
            (Phase::Running { .. } | Phase::Looping, Some(clip)) => {
                ((clip.position(self.played()) - clip.start) / clip.length()).min(1.0)
            }
            (Phase::Running { remaining }, None) if self.length > 0.0 => {
                1.0 - remaining / self.length
            }
            _ => 1.0,
        };
        progress as f32
    }

    /// Returns how many seconds of the action have passed, which tells
    /// where in its clip the sound and the picture of the cue should be
    pub(in crate::back) fn played(&self) -> f64 {
        (self.elapsed - self.pre_wait).max(0.0)
    }

    pub(in crate::back::workspace::running) fn waiting(&self) -> bool {
        self.elapsed < 0.0
    }

    /// Returns the seconds until the action of a cue that has yet to begin,
    /// which a pause holds off
    pub(in crate::back::workspace::running) fn acts_in(&self) -> Option<f64> {
        (!self.started && !self.paused).then_some(self.pre_wait - self.elapsed)
    }

    /// Returns whether the action is to begin, which a pause holds off, as a
    /// pause cue holds a cue that a start cue has fired before it acts
    pub(in crate::back::workspace::running) fn due(&self) -> bool {
        !self.started && !self.paused && self.elapsed >= self.pre_wait
    }

    pub(in crate::back::workspace::running) fn begun(self) -> Self {
        Self {
            started: self.started || self.due(),
            ..self
        }
    }

    /// Returns whether the cue has run its course, which it cannot before
    /// its action has begun, so that a cue that a start cue has just fired
    /// and that takes no time still acts.  A group never does by time, since
    /// it ends with the cues that it holds.
    pub(in crate::back::workspace::running) fn finished(&self) -> bool {
        self.started
            && !self.endless()
            && !self.holds()
            && self.elapsed >= self.pre_wait + self.length
    }

    pub(in crate::back::workspace::running) fn advanced(self, seconds: f64) -> Self {
        if self.paused {
            self
        } else {
            Self {
                elapsed: self.elapsed + seconds,
                ..self
            }
            .faded(seconds)
        }
    }

    /// Returns the run with its clock moved a share of the way to the time
    /// its sound has reached.  The position of a sound is up to a buffer of
    /// the audio device old, by a different amount each time, so a share
    /// keeps the picture from jittering while the clock still follows the
    /// drift of the audio device.  A paused run keeps its clock.
    pub(in crate::back::workspace::running) fn heard(self, played: f64) -> Self {
        if self.paused {
            self
        } else {
            Self {
                elapsed: self.elapsed + (played - self.played()) * HEARING,
                ..self
            }
        }
    }
}
