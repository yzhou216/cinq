// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Builders of the cues that play, group, and chain

use crate::back::{Advance, Cue, CueId, Kind};

pub(crate) fn audio(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Audio, "Sound")
        .file("sound.ogg")
        .build()
}

pub(super) fn picture(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Picture, "Slide")
        .file("slide.png")
        .build()
}

pub(in crate::back) fn video(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Video, "Movie")
        .file("movie.mp4")
        .build()
}

pub(in crate::back) fn continued(cue: Cue) -> Cue {
    Cue {
        advance: Advance::Continue,
        ..cue
    }
}

pub(super) fn continuing(id: u32) -> Cue {
    continued(audio(id))
}

pub(in crate::back) fn continuing_after(id: u32, post_wait: f32) -> Cue {
    Cue {
        post_wait: post_wait.into(),
        ..continuing(id)
    }
}

pub(super) fn group(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Group, "Group").build()
}

/// Returns the cue at `depth` in groups
pub(super) fn within(depth: u8, cue: Cue) -> Cue {
    Cue { depth, ..cue }
}

/// Returns an audio cue that loops from 2 to 6 seconds into its file of ten
/// seconds, so that it reaches the end of its loop first after 6 seconds
/// and goes round the loop every 4
pub(super) fn vamp(id: u32) -> Cue {
    Cue {
        looping: true,
        loop_start: 2.0.into(),
        loop_end: 6.0.into(),
        ..audio(id)
    }
}
