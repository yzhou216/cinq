// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Part of a file that an audio or video cue plays, and what the stage
//! window shows of the file of a video or picture cue

use super::{Cue, Kind, Media};

/// Part of its file that an audio or video cue plays, from `start` to `end`
/// in seconds into the file, where `start` comes before `end`.  A clip
/// that loops plays from its start to the end of its loop, then from the
/// start of its loop again each time it reaches that end, until something
/// stops its cue or a devamp cue lets it go on to its end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::back) struct Clip {
    pub(in crate::back) start: f64,
    pub(in crate::back) end: f64,
    pub(in crate::back) looping: Option<Loop>,
}

/// Part of a clip that the clip plays again and again, from `start` to `end`
/// in seconds into the file, where `start` comes before `end` and both lie
/// within the clip
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::back) struct Loop {
    pub(in crate::back) start: f64,
    pub(in crate::back) end: f64,
    /// Seconds of the clip that will have played as its sound leaves the
    /// loop for the last time, at the end of a pass, once a devamp cue has
    /// let it go on
    pub(in crate::back) released: Option<f64>,
}

/// Stretch of a file that one pass through a clip plays, from `from` to
/// `to` in seconds into the file, which starts once `at` seconds of the
/// clip have played
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::back) struct Pass {
    pub(in crate::back) from: f64,
    pub(in crate::back) to: f64,
    pub(in crate::back) at: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::back) enum Shown {
    Clip(Clip),
    Still,
}

impl Loop {
    fn length(&self) -> f64 {
        self.end - self.start
    }
}

impl Clip {
    /// Returns how long the clip plays from its start to its end, without
    /// going round its loop
    pub(in crate::back) fn length(&self) -> f64 {
        self.end - self.start
    }

    /// Returns how many seconds of the clip play before it reaches the start
    /// of `section` for the first time
    fn lead(&self, section: &Loop) -> f64 {
        section.start - self.start
    }

    /// Returns where in the file the clip is once `played` seconds of it
    /// have played, counting every pass of a clip that loops
    pub(in crate::back) fn position(&self, played: f64) -> f64 {
        match self.looping {
            Some(section) => match section.released {
                Some(released) if played >= released => section.end + (played - released),
                _ if played < self.lead(&section) => self.start + played,
                _ => section.start + (played - self.lead(&section)) % section.length(),
            },
            None => self.start + played,
        }
    }

    /// Returns how many seconds of the clip have played once its sound
    /// reaches `position` in the file, which is [`Clip::position`] the other
    /// way round.  A position within the loop of a clip says nothing about
    /// which pass it is on, so the result takes the pass that lies nearest
    /// `near`, the seconds that the clock says have played, and no pass
    /// after the one that a devamp cue has made the last.
    pub(in crate::back) fn played_at(&self, position: f64, near: f64) -> f64 {
        let into = position - self.start;
        match self.looping {
            Some(section) => match section.released {
                Some(released) if position >= section.end => released + (position - section.end),
                _ if position < section.start => into,
                released => {
                    let length = section.length();
                    let last = released.map_or(f64::INFINITY, |released| {
                        ((released - self.lead(&section)) / length).round() - 1.0
                    });
                    ((near - into) / length).round().max(0.0).min(last) * length + into
                }
            },
            None => into,
        }
    }

    /// Returns how many seconds the clip plays in all, which a clip that
    /// loops has no end to until a devamp cue lets it go on
    pub(in crate::back) fn lasts(&self) -> Option<f64> {
        match self.looping {
            Some(section) => section
                .released
                .map(|released| released + (self.end - section.end)),
            None => Some(self.length()),
        }
    }

    /// Returns the clip let go on from its loop at the first end of a pass
    /// that lies more than `notice` seconds after `played` seconds of it,
    /// so that its sound leaves the loop there and plays on to the end of
    /// the clip.  A clip that does not loop, or that has been let go on
    /// already, stays as it is.
    pub(in crate::back) fn released(self, played: f64, notice: f64) -> Self {
        let Some(section) = self.looping.filter(|section| section.released.is_none()) else {
            return self;
        };
        let lead = self.lead(&section);
        let length = section.length();
        let passes = ((played + notice - lead) / length).floor().max(0.0) + 1.0;
        Self {
            looping: Some(Loop {
                released: Some(lead + passes * length),
                ..section
            }),
            ..self
        }
    }

    /// Returns the clip as it was before a devamp cue let it go on, which
    /// plays the same up to its last pass, so that a sound or a picture made
    /// ready ahead for a run serves it once a devamp cue has let it go on
    pub(in crate::back) fn unreleased(self) -> Self {
        Self {
            looping: self.looping.map(|section| Loop {
                released: None,
                ..section
            }),
            ..self
        }
    }

    /// Returns how many seconds of the clip will have played as its last
    /// pass through its loop begins, once a devamp cue has let it go on
    pub(in crate::back) fn last_pass(&self) -> Option<f64> {
        let section = self.looping?;
        Some(section.released? - section.length())
    }

    /// Returns pass `index` through the clip, counted from zero, or none
    /// once the clip has no more to play.  A clip that does not loop makes
    /// one pass from its start to its end.  One that loops makes its first
    /// pass from its start to the end of its loop, and each pass after that
    /// through its loop, until the last pass that a devamp cue lets it go
    /// on from, which is followed by a pass from the end of its loop to the
    /// end of the clip, where the loop ends before the clip does.
    pub(in crate::back) fn pass(&self, index: u32) -> Option<Pass> {
        let Some(section) = self.looping else {
            return (index == 0).then_some(Pass {
                from: self.start,
                to: self.end,
                at: 0.0,
            });
        };
        let lead = self.lead(&section);
        let length = section.length();
        let start = lead + f64::from(index) * length;
        match section.released {
            Some(released) if start >= released - length / 2.0 => {
                let ending = ((released - lead) / length).round() as u32;
                (index == ending && section.end < self.end).then_some(Pass {
                    from: section.end,
                    to: self.end,
                    at: released,
                })
            }
            _ if index == 0 => Some(Pass {
                from: self.start,
                to: section.end,
                at: 0.0,
            }),
            _ => Some(Pass {
                from: section.start,
                to: section.end,
                at: start,
            }),
        }
    }
}

impl Cue {
    /// A video cue that loops while its sound plays ends with its sound,
    /// where that comes first, so that the sound loops with the picture.
    /// The loop of a cue lies within its clip, so a loop that starts before
    /// the clip starts with the clip, and one that ends after the clip ends
    /// with it.
    pub(in crate::back) fn clip(&self) -> Option<Clip> {
        let (start, end) = self.span()?;
        let looping = self.looping.then(|| self.section(start, end));
        looping
            .is_none_or(|section| section.start < section.end)
            .then_some(Clip {
                start,
                end,
                looping,
            })
    }

    /// Returns whether the cue loops nothing, as one whose loop starts at or
    /// after the end of its loop does
    pub(in crate::back) fn loops_nothing(&self) -> bool {
        self.loops()
            && self
                .span()
                .is_some_and(|(start, end)| self.section(start, end).length() <= 0.0)
    }

    /// Returns where the clip of the cue starts and ends in its file, once
    /// the file has loaded, when the clip starts before it ends
    fn span(&self) -> Option<(f64, f64)> {
        let Media::Loaded { duration, .. } = self.media else {
            return None;
        };
        let length = match self.sound() {
            Some(sound) if self.looping => duration.min(sound),
            _ => duration,
        };
        let start = self.start.value();
        let end = self.end.time().map_or(length, |end| end.min(length));
        (self.kind.plays_file() && start < end).then_some((start.into(), end.into()))
    }

    fn section(&self, start: f64, end: f64) -> Loop {
        Loop {
            start: self
                .loop_start
                .time()
                .map_or(start, |time| f64::from(time).max(start)),
            end: self
                .loop_end
                .time()
                .map_or(end, |time| f64::from(time).min(end)),
            released: None,
        }
    }

    pub(in crate::back) fn shown(&self) -> Option<Shown> {
        match self.kind {
            Kind::Video => self.clip().map(Shown::Clip),
            Kind::Picture => matches!(self.media, Media::Loaded { .. }).then_some(Shown::Still),
            _ => None,
        }
    }

    /// Returns how long the sound of the file of the cue plays, once the
    /// file has loaded, when the clip of the cue starts before the sound
    /// ends
    fn sound(&self) -> Option<f32> {
        match self.media {
            Media::Loaded { sound, .. } => sound.filter(|&sound| self.start.value() < sound),
            _ => None,
        }
    }

    /// Returns whether the clip of the cue has sound to play: that of an
    /// audio cue always has, once its file has loaded, and that of a video
    /// cue has unless its file holds a picture alone or its sound ends
    /// before the clip starts
    pub fn sounds(&self) -> bool {
        self.sound().is_some()
    }

    pub fn loops(&self) -> bool {
        self.kind.plays_file() && self.looping
    }
}
