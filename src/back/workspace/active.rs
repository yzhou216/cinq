// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cues that GO has fired, with what they do and the fades they make in
//! [`effect`], and what they play and show in [`playing`]
//!
//! Time here is counted in seconds as `f64`, since a clock that adds a
//! tick forty times a second for an hour would drift by seconds in `f32`.

mod effect;
mod group;
mod playing;

use crate::back::{Cue, CueId, Kind, LOUDEST};
pub(super) use effect::{Effect, Fade};

/// Share of the way to the time that its sound has reached that a run
/// moves its clock, once for each sync of the sound output
const HEARING: f64 = 0.1;

/// Cue that GO has fired and that has not finished yet
///
/// A fired cue keeps its timing and its effect, such as the clip of an
/// audio or video cue, as its cue had them when GO fired it, so that an
/// edit changes the next run of a cue and not the one under way.  Only a
/// running audio or video cue follows an edit at once: its level moves by
/// the change in the level of its cue, and it stops when it gets a new
/// file.  The fades that a fade cue makes, and its runs that have yet to
/// act, take the new level of the fade cue.  A light cue reads its levels,
/// and a network cue its message, as its action begins.
#[derive(Debug, Clone, Copy)]
pub struct Active {
    pub id: CueId,
    /// Serial number of this firing, which tells a cue that GO fired again
    /// apart from one that kept running
    pub(in crate::back) run: u32,
    /// Run of the cue whose start, or whose end for an auto-follow, starts
    /// this one, or `None` for the cue that GO itself starts.  A group
    /// starts the cues that it holds directly, and the cue that follows it
    /// in the chain.  Runs of a chain are not always consecutive, since a
    /// cue that GO fires again takes a run of its own, so a cue finds the
    /// cues it starts by this.
    pub(super) after: Option<u32>,
    /// Seconds since GO, negative while the cue waits its turn in a chain
    pub(super) elapsed: f64,
    pub paused: bool,
    /// Level in dB an audio or video cue plays at.  An edit can move it
    /// below -60 dB, which sounds as silence, so that moving the level
    /// of the cue back restores what a fade had set.
    pub level: f32,
    pub fade: Option<Fade>,
    pub(super) pre_wait: f64,
    /// Seconds that the action takes after the pre-wait.  For a clip that
    /// loops this is how long the clip takes from its start to its end,
    /// without going round its loop, until a devamp cue lets it go on, and
    /// then how long the whole action takes.
    pub(super) length: f64,
    pub(super) effect: Effect,
    /// Whether the action has begun, so that a fade or stop cue acts once
    pub(super) started: bool,
}

/// Where an active cue is in its lifetime.  An audio or video cue whose
/// clip loops stays [`Phase::Looping`] until something stops it, as a
/// picture cue stays [`Phase::Showing`], unless a devamp cue lets it go on
/// from its loop, which makes it [`Phase::Running`] until its clip ends.  A
/// group is [`Phase::Endless`] where a cue that it holds never ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Scheduled { starts_in: f64 },
    PreWait { remaining: f64 },
    Running { remaining: f64 },
    Looping,
    Showing,
    Endless,
}

impl Active {
    /// Fires a cue `delay` seconds after GO, as run `run`, started by run
    /// `after` of the same chain, or by GO itself when `after` is `None`
    pub(super) fn new(cue: &Cue, run: u32, after: Option<u32>, delay: f64) -> Self {
        let length = f64::from(cue.action());
        let effect = match (cue.kind, cue.target) {
            (Kind::Audio, _) => cue.clip().map_or(Effect::Nothing, Effect::Sound),
            (Kind::Video, _) => cue.clip().map_or(Effect::Nothing, |clip| Effect::Video {
                clip,
                sound: cue.sounds(),
            }),
            (Kind::Picture, _) => Effect::Picture,
            (Kind::Light, _) => Effect::Light,
            (Kind::Network, _) => Effect::Send,
            (Kind::Group, _) => Effect::Group { endless: false },
            (Kind::Fade, Some(target)) => Effect::Fade {
                target,
                fade: Fade {
                    by: Some(cue.id),
                    to: cue.level,
                    remaining: length,
                    length,
                    then_stop: cue.stop_target,
                    opacity: 1.0,
                },
            },
            (Kind::Stop, Some(target)) => Effect::Stop(target),
            (Kind::Start, Some(target)) => Effect::Start(target),
            (Kind::Pause, Some(target)) => Effect::Pause(target),
            (Kind::Devamp, Some(target)) => Effect::Devamp(target),
            (Kind::GoTo, Some(target)) => Effect::GoTo(target),
            _ => Effect::Nothing,
        };
        Self {
            id: cue.id,
            run,
            after,
            elapsed: -delay,
            paused: false,
            level: cue.level,
            fade: None,
            pre_wait: cue.pre_wait.value().into(),
            length,
            effect,
            started: false,
        }
    }

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
    /// clip, so that the progress of a clip that loops runs to the end of its
    /// loop and back to the start of its loop with each pass, and runs on to
    /// the end of the clip, without going back, once a devamp cue has let it
    /// go on.
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

    pub(super) fn waiting(&self) -> bool {
        self.elapsed < 0.0
    }

    /// Returns whether the action is to begin, which waits while the cue is
    /// paused, as one that a start cue has fired and a pause cue holds before
    /// it acts
    pub(super) fn due(&self) -> bool {
        !self.started && !self.paused && self.elapsed >= self.pre_wait
    }

    pub(super) fn begun(self) -> Self {
        Self {
            started: self.started || self.due(),
            ..self
        }
    }

    pub(super) fn faded_out(&self) -> bool {
        self.fade
            .is_some_and(|fade| fade.then_stop && fade.remaining <= 0.0)
    }

    /// Returns whether the cue has run its course, which it cannot before
    /// its action has begun, so that a cue that a start cue has just fired
    /// and that takes no time still acts.  A group never does by time, since
    /// it ends with the cues that it holds.
    pub(super) fn finished(&self) -> bool {
        self.started
            && !self.endless()
            && !self.holds()
            && self.elapsed >= self.pre_wait + self.length
    }

    pub(super) fn advanced(self, seconds: f64) -> Self {
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
    pub(super) fn heard(self, played: f64) -> Self {
        if self.paused {
            self
        } else {
            Self {
                elapsed: self.elapsed + (played - self.played()) * HEARING,
                ..self
            }
        }
    }

    /// Returns the run with its level moved by `by` dB, as far up as
    /// [`LOUDEST`] and without a limit down, and with any fade left to go on
    /// from there
    pub(super) fn moved(self, by: f32) -> Self {
        Self {
            level: (self.level + by).min(LOUDEST),
            ..self
        }
    }

    /// Returns the run with the fades that the fade cue `by` is making on
    /// it, and the fade that it has yet to make if it is a run of that cue,
    /// ending at `level`.  A fade under way keeps its remaining time, so
    /// that the level turns from where it is toward the new one.
    pub(super) fn retargeted(self, by: CueId, level: f32) -> Self {
        let to = |fade: Fade| Fade { to: level, ..fade };
        Self {
            fade: self
                .fade
                .map(|fade| if fade.by == Some(by) { to(fade) } else { fade }),
            effect: match self.effect {
                Effect::Fade { target, fade } if self.id == by && !self.started => Effect::Fade {
                    target,
                    fade: to(fade),
                },
                effect => effect,
            },
            ..self
        }
    }

    /// Returns the run with its clip let go on from its loop, as
    /// [`Clip::released`](crate::back::Clip::released) tells, `notice`
    /// seconds ahead at the least, and with its action lasting until the
    /// clip ends.  A run whose clip stays as it was keeps its length.
    pub(super) fn released(self, notice: f64) -> Self {
        let effect = match self.effect {
            Effect::Sound(clip) => Effect::Sound(clip.released(self.played(), notice)),
            Effect::Video { clip, sound } => Effect::Video {
                clip: clip.released(self.played(), notice),
                sound,
            },
            effect => effect,
        };
        match effect.clip().and_then(|clip| clip.lasts()) {
            Some(length) if effect != self.effect => Self {
                effect,
                length,
                ..self
            },
            _ => self,
        }
    }

    pub(super) fn under(self, fade: Fade) -> Self {
        Self {
            fade: Some(Fade {
                opacity: self.opacity(),
                ..fade
            }),
            ..self
        }
    }

    /// Moves the level `seconds` along its fade.  Each step covers the share
    /// of the remaining change that the step is of the remaining time, so
    /// the level moves in a straight line in dB.  A fade that leaves the cue
    /// running is dropped once it ends.
    pub(super) fn faded(self, seconds: f64) -> Self {
        match self.fade {
            Some(fade) if fade.remaining > seconds => Self {
                level: self.level + (fade.to - self.level) * (seconds / fade.remaining) as f32,
                fade: Some(Fade {
                    remaining: fade.remaining - seconds,
                    ..fade
                }),
                ..self
            },
            Some(fade) => Self {
                level: fade.to,
                fade: fade.then_stop.then_some(Fade {
                    remaining: 0.0,
                    ..fade
                }),
                ..self
            },
            None => self,
        }
    }
}
