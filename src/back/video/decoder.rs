// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Threads that decode the clip of one run of a video cue, a pass through
//! the clip each, a few pictures ahead of the clock of the run
//!
//! A pass starts a decoder at the last sync sample before the start of the
//! clip that shows no later than the first picture of the clip, and decodes
//! from there to the last sample that shows before its end.  The decoder
//! spreads the pictures over the cores, which 1080p at sixty pictures a
//! second and 4K need to keep up, and gives them back in the order they
//! show, each with the index of the sample that started it, so that it
//! shows at the time of its own sample.  Those before the start of the clip
//! are let go without turning them into RGBA, which [`color`] turns the
//! rest into.  The decoder holds back as many pictures as the stream says
//! it reorders.  A picture that fails to decode is dropped, and those that
//! refer to it, up to the next sync sample, draw on a grey picture in its
//! place.  A pass ends by telling its player that it has sent every
//! picture, or why its picture cannot play, as a pass that decodes no
//! picture cannot, and it stops decoding as soon as its player has gone.
//! Each pass opens its file and reads the index of its track anew, which
//! takes a long file a moment, hidden for a clip that loops by the pass
//! that decodes ahead.

mod color;

use std::{
    num::NonZero,
    path::{Path, PathBuf},
    sync::{
        Weak,
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
};

use rust_h264::{
    decoder::{ChromaFormat, Frame as Decoded},
    nal::parse_sps_info,
    threading::ThreadedDecoder,
};

use super::track::Track;
use crate::back::Clip;
pub use color::Picture;
use color::rgba;

/// Pictures that the thread of a pass decodes ahead of the clock of its
/// run, besides the one it holds as it waits to send it: enough to ride out
/// the pictures that take longest to decode, such as those that start a
/// group of pictures, and few enough to keep the memory of a pass small,
/// as each picture of 1080p takes 8 MB
const AHEAD: usize = 2;

/// Most threads that decode the pictures of a pass, one for each core:
/// enough to spread them over the cores of most machines
const THREADS: usize = 8;

/// Pictures that the decoder holds back to put in order when the stream
/// does not say how many it reorders, which is the decoder's own default,
/// and the most that it holds back for any stream, which is as many as
/// H.264 allows
const REORDER: usize = 16;

/// Why the picture of a file that holds H.264 cannot play, when the decoder
/// decodes none of it
pub(super) const UNDECODABLE: &str = "its picture cannot be decoded";

/// Picture along with the time of its run at which it is due: the seconds
/// of the clip played, counting every pass of a clip that loops
pub(super) struct Frame {
    pub(super) at: f64,
    pub(super) picture: Picture,
}

pub(super) enum Sent {
    Picture(Frame),
    /// Pass is over, and every picture of it has been sent
    Ended,
    Failed(String),
}

/// Why decoding stops before the end of the pass
enum Stop {
    /// Player of the run has gone, along with the way to it
    Gone,
    /// File cannot be read, for the given reason
    Failed(String),
}

/// Starts decoding pass `pass` through `clip` of the video file at `path`
/// on a thread of its own, for the player whose life `player` watches, and
/// returns the way to its pictures.  The thread ends once it has decoded
/// the pass, or once the player has gone.
pub(super) fn start(path: PathBuf, clip: Clip, pass: u32, player: Weak<()>) -> Receiver<Sent> {
    let (sender, pictures) = mpsc::sync_channel(AHEAD);
    thread::spawn(move || {
        let end = match decoded(&path, clip, pass, &sender, &player) {
            Ok(0) => Sent::Failed(UNDECODABLE.to_owned()),
            Ok(_) => Sent::Ended,
            Err(Stop::Failed(reason)) => Sent::Failed(reason),
            Err(Stop::Gone) => return,
        };
        // Player that would hear of it may have gone since
        let _ = sender.send(end);
    });
    pictures
}

/// Returns how many pictures of the pass went to `sender`
fn decoded(
    path: &Path,
    clip: Clip,
    pass: u32,
    sender: &SyncSender<Sent>,
    player: &Weak<()>,
) -> Result<usize, Stop> {
    let mut track = Track::open(path).map_err(Stop::Failed)?;
    let Some((from, to, first)) = track.span(clip.start, clip.end) else {
        return Ok(0);
    };
    let passed = f64::from(pass) * clip.length();
    // Copied out, as reading a sample borrows the track anew
    let times: Vec<f64> = track.samples.iter().map(|sample| sample.shows).collect();
    let send = |pictures: Vec<(Decoded, usize)>, sent: usize| {
        pictures
            .into_iter()
            .map(|(picture, index)| (times[index], picture))
            .filter(|(shows, picture)| {
                (first..clip.end).contains(shows)
                    && picture.bit_depth == 8
                    && picture.chroma_format == ChromaFormat::Yuv420
            })
            .try_fold(sent, |sent, (shows, picture)| {
                let frame = Frame {
                    at: passed + (shows - clip.start).max(0.0),
                    picture: rgba(&picture),
                };
                sender
                    .send(Sent::Picture(frame))
                    .map(|()| sent + 1)
                    .map_err(|_| Stop::Gone)
            })
    };
    let parameters = track.parameter_sets();
    let reorder = parameters
        .iter()
        .find_map(|unit| parse_sps_info(unit).ok())
        .and_then(|info| info.max_reorder_frames)
        .map_or(REORDER, |frames| (frames as usize).min(REORDER));
    let threads = thread::available_parallelism().map_or(1, NonZero::get);
    let mut decoder = ThreadedDecoder::with_max_depth(threads.min(THREADS), reorder);
    // Parameter sets that fail to decode leave every sample to fail as well
    parameters.iter().for_each(|unit| {
        let _ = decoder.decode_nal_with_meta(unit, 0);
    });
    let sent = (from..=to).try_fold(0, |sent, index| {
        if player.strong_count() == 0 {
            return Err(Stop::Gone);
        }
        let bytes = track.sample(index).map_err(Stop::Failed)?;
        // Unit that fails to decode adds nothing to the picture, which
        // decoding goes on without
        let pictures = track
            .units(&bytes)
            .iter()
            .filter_map(|unit| decoder.decode_nal_with_meta(unit, index).ok())
            .flatten()
            .collect();
        send(pictures, sent)
    })?;
    send(decoder.flush_with_meta(), sent)
}
