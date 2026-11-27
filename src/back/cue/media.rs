// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What loading the file of a cue found, and why a cue cannot play

use std::{fs, path::Path, time::SystemTime};

use strum::Display;

/// Size and modification time of a file, by which a later look tells that it
/// has changed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
}

impl Stamp {
    /// Returns the stamp of the file at `path`, or none where it is not
    /// there.  This reads no more than the directory entry, so that
    /// looking at every file of a show costs little.
    pub fn of(path: &Path) -> Option<Self> {
        let found = fs::metadata(path).ok()?;
        Some(Self {
            len: found.len(),
            modified: found.modified().ok(),
        })
    }

    /// Returns the stamp as a failure of the file at `path` keeps it, which
    /// is none where the file cannot be opened.  Such a file may become
    /// readable with no change to its size or time, as a change of
    /// permission makes it, so that the failure must be looked at again,
    /// while one that opens and fails by its contents stays until the file
    /// changes.
    pub fn when_readable(self, path: &Path) -> Option<Self> {
        fs::File::open(path).ok().map(|_| self)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Media {
    #[default]
    Unloaded,
    /// File plays for `duration` seconds, and its sound for `sound`
    /// seconds, apart from a video file with a picture alone, which holds
    /// no sound.  The two are the same for a sound file, while the sound
    /// of a video file can end before its picture or after it.  A picture
    /// file, which shows until something stops its cue, plays for none and
    /// holds no sound.
    Loaded { duration: f32, sound: Option<f32> },
    /// File cannot be played, for the given reason
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum Problem<'a> {
    #[strum(to_string = "no file set")]
    NoFile,
    #[strum(to_string = "file not loaded")]
    NotLoaded,
    #[strum(transparent)]
    LoadFailed(&'a str),
    #[strum(to_string = "starts at or after its end")]
    EmptyClip,
    #[strum(to_string = "starts past the end of its file")]
    PastFile,
    /// Loop that starts at or after its end, once both lie within the clip
    #[strum(to_string = "loop starts at or after its end")]
    EmptyLoop,
    #[strum(to_string = "no target cue")]
    NoTarget,
    /// Control cue names a cue that it cannot act on, such as itself, or a
    /// group for a fade or a devamp
    #[strum(to_string = "cannot act on its target cue")]
    WrongTarget,
    /// Start cue lies in the group that it starts, which would fire it
    /// again and so never end
    #[strum(to_string = "lies in the group that it starts")]
    InTarget,
    #[strum(to_string = "target cue does not loop")]
    NotLooping,
    #[strum(to_string = "no destination as host:port")]
    NoDestination,
    #[strum(to_string = "port is not a number from 1 to 65535")]
    BadPort,
    #[strum(to_string = "no OSC message as /address arguments")]
    NoMessage,
    #[strum(to_string = "OSC message leaves a double quote open")]
    OpenQuote,
}
