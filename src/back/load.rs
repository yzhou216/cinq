// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Loading the files that audio, video, and picture cues name
//!
//! [`load_files`] finds how long each file plays, or why it cannot be
//! played, without decoding it.  Reading can block on a slow drive, so the
//! interface runs it off its own thread.

#[cfg(test)]
mod tests;

use std::{collections::HashMap, io, path::PathBuf};

use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::{Command, Cue, CueId, Kind, Media, Stamp, Workspace, audio, video};
use crate::warn;

pub struct Load {
    cue: CueId,
    file: String,
    path: PathBuf,
    kind: Kind,
    /// Stamp that the file had when the cue last read it
    seen: Option<Stamp>,
}

impl Load {
    pub fn all<'a>(workspace: &Workspace, cues: impl IntoIterator<Item = &'a Cue>) -> Vec<Self> {
        cues.into_iter()
            .filter_map(|cue| {
                Some(Self {
                    cue: cue.id,
                    file: cue.file.clone(),
                    path: workspace.path(cue)?,
                    kind: cue.kind,
                    seen: cue.seen,
                })
            })
            .collect()
    }

    /// Returns whether the file has to be read, given the stamp that it has
    /// now.  It does unless it is there and stamped as the cue saw it, so
    /// that a file that has changed or gone is read again, while one that
    /// stayed as it was keeps what reading it found, a failure that
    /// playing found included.
    fn stale(&self, now: Option<Stamp>) -> bool {
        now.is_none() || self.seen != now
    }
}

/// Reads the files of `loads` that no cue has read, or that have changed or
/// gone since, and returns the commands that record what reading found.
/// A show whose files stay as they are costs one look at the stamp of each.
/// Each distinct file is read once for each kind of cue that plays it, and
/// the files are read in parallel, which shortens opening a large show
/// whose files are not cached yet.
pub fn load_files(loads: Vec<Load>) -> Vec<Command> {
    let paths: Vec<&PathBuf> = loads.iter().map(|load| &load.path).unique().collect();
    let stamps: HashMap<PathBuf, Option<Stamp>> = paths
        .par_iter()
        .map(|path| ((*path).clone(), Stamp::of(path)))
        .collect();
    let stale: Vec<Load> = loads
        .into_iter()
        .filter(|load| load.stale(stamps[&load.path]))
        .collect();
    // Collected first, so that rayon reads no file or a single one on this
    // thread instead of waking its pool
    let files: Vec<(PathBuf, Kind)> = stale
        .iter()
        .map(|load| (load.path.clone(), load.kind))
        .unique()
        .collect();
    let found: HashMap<&(PathBuf, Kind), Media> =
        files.par_iter().map(|file| (file, probe(file))).collect();
    stale
        .into_iter()
        .map(|load| {
            let media = found[&(load.path.clone(), load.kind)].clone();
            let seen = stamps[&load.path].and_then(|stamp| match media {
                Media::Failed(_) => stamp.when_readable(&load.path),
                _ => Some(stamp),
            });
            Command::Loaded {
                media,
                seen,
                cue: load.cue,
                file: load.file,
            }
        })
        .collect()
}

/// Reads how long a file plays for the kind of cue that plays it, and how
/// long its sound plays, or why it cannot be played.  The sound track of a
/// video file has to play as well, since the cue plays it along with the
/// picture, while a video file without one plays its picture alone.
fn probe((path, kind): &(PathBuf, Kind)) -> Media {
    let loaded = match kind {
        Kind::Video => video::probe(path).and_then(|(duration, sound)| {
            let sound = sound
                .then(|| audio::probe(path))
                .transpose()
                .map_err(|reason| format!("its sound cannot play: {reason}"))?;
            Ok((duration, sound))
        }),
        Kind::Picture => video::probe_still(path).map(|()| (0.0, None)),
        _ => audio::probe(path).map(|duration| (duration, Some(duration))),
    };
    match loaded {
        Ok((duration, sound)) => Media::Loaded { duration, sound },
        Err(reason) => Media::Failed(reason),
    }
}

/// Returns the command that records why the file of `cue` cannot be
/// played, as the sound and video outputs find when they play it, with the
/// stamp that the file has now, so that the failure stays until the file
/// changes, which a read of its header alone could not say it had cured
pub(super) fn unplayable(workspace: &Workspace, cue: &Cue, reason: String) -> Command {
    let seen = workspace
        .path(cue)
        .and_then(|path| Stamp::of(&path)?.when_readable(&path));
    Command::Loaded {
        cue: cue.id,
        file: cue.file.clone(),
        media: Media::Failed(reason),
        seen,
    }
}

/// Tells why a file cannot be read, in the same words for sound, video,
/// and picture files, and without the error number of the operating system
pub(super) fn unreadable(error: io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound => "file not found".to_owned(),
        _ => warn::words(&error),
    }
}
