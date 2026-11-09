// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of decoding right from the first frame after a seek, and across the
//! wrap of a loop, on a tone in each of AAC, MP3, and Ogg Vorbis
//!
//! Each tone is 0.4 seconds of two sines at 16000 frames a second, which
//! ffmpeg encoded with the command `ffmpeg -f lavfi -i
//! 'aevalsrc=0.1*sin(2*PI*440*t)+0.05*sin(2*PI*1330*t):s=16000:d=0.4' -ac 1
//! -c:a CODEC -b:a RATE -fflags +bitexact -flags:a +bitexact -map_metadata -1
//! FILE`, with `aac`, `12k`, and `tone.m4a` (and `-movflags +faststart`),
//! `libmp3lame`, `16k`, and `tone.mp3`, and `libvorbis`, `16k`, and
//! `tone.ogg`.  The AAC file holds a frame of priming.  Each sound is nowhere
//! silent, so that a frame decoded without the frames before it differs from
//! the right one.

use std::{io::Cursor, iter};

use kira::{Frame, sound::streaming::Decoder};

use super::Prerolled;

const TONES: [(&str, &[u8]); 3] = [
    ("AAC", include_bytes!("../tests/tone.m4a")),
    ("MP3", include_bytes!("../tests/tone.mp3")),
    ("Ogg Vorbis", include_bytes!("../tests/tone.ogg")),
];

/// Frames that a test checks after each seek
const SPAN: usize = 1024;

/// Greatest difference of a sample that two decodes of the same frames may
/// show, where a decode with a wrong state shows 0.07 or more
const TOLERANCE: f32 = 1e-5;

fn tone(bytes: &'static [u8]) -> Prerolled {
    Prerolled::new(Box::new(Cursor::new(bytes))).expect("the tone opens")
}

/// Returns every frame of the tone, decoded from the start without a seek,
/// as a decoder decodes a file that plays from its start
fn straight(bytes: &'static [u8]) -> Vec<Frame> {
    let mut decoder = tone(bytes);
    iter::from_fn(|| decoder.packet().expect("the tone decodes"))
        .flat_map(|(_, frames)| frames)
        .collect()
}

/// Returns by how much the frames of `found` differ the most from those of
/// `expected`, as the greatest difference of a sample
fn worst(found: &[Frame], expected: &[Frame]) -> f32 {
    assert_eq!(found.len(), expected.len());
    found
        .iter()
        .zip(expected)
        .flat_map(|(found, expected)| {
            [
                (found.left - expected.left).abs(),
                (found.right - expected.right).abs(),
            ]
        })
        .fold(0.0, f32::max)
}

#[test]
fn frames_after_a_seek_are_those_of_a_straight_decode() {
    TONES.into_iter().for_each(|(name, bytes)| {
        let straight = straight(bytes);
        let mut decoder = tone(bytes);
        // From the end backwards, so that the decoder holds the state of
        // loud frames far later when a seek lands in the first packets
        (0..straight.len() - SPAN)
            .step_by(61)
            .rev()
            .for_each(|index| {
                let seeked = decoder.seek(index).expect("the seek works");
                assert_eq!(seeked, index, "{name}: seek to {index} lands elsewhere");
                let found: Vec<Frame> = iter::repeat_with(|| decoder.decode().expect("decodes"))
                    .take(SPAN)
                    .flatten()
                    .take(SPAN)
                    .collect();
                let worst = worst(&found, &straight[index..index + SPAN]);
                assert!(
                    worst < TOLERANCE,
                    "{name}: seek to {index} is off by {worst}"
                );
            });
    });
}

#[test]
fn a_seek_past_the_last_frame_leaves_nothing_to_decode() {
    TONES.into_iter().for_each(|(name, bytes)| {
        let mut decoder = tone(bytes);
        decoder.seek(1000).expect("the seek works");
        decoder.seek(straight(bytes).len()).expect("the seek works");
        let left = decoder.decode().expect("the tone decodes").len();
        assert_eq!(left, 0, "{name}: {left} frames are left");
    });
}

/// A sound that kira plays from a decoder, as its decoding thread asks the
/// decoder for the frame at each index in turn, and seeks only to go back
///
/// The thread runs on its own, so that a test can hear its result only by
/// waiting for it, which this avoids by doing what it does.
struct Playing {
    decoder: Prerolled,
    decoded: usize,
    chunk: (usize, Vec<Frame>),
}

impl Playing {
    fn from(index: usize, bytes: &'static [u8]) -> Self {
        let mut decoder = tone(bytes);
        Self {
            decoded: decoder.seek(index).expect("the seek works"),
            decoder,
            chunk: (0, Vec::new()),
        }
    }

    fn frame(&mut self, index: usize) -> Frame {
        let within = |(first, frames): &(usize, Vec<Frame>)| {
            index
                .checked_sub(*first)
                .and_then(|at| frames.get(at))
                .copied()
        };
        if let Some(frame) = within(&self.chunk) {
            return frame;
        }
        if index < self.decoded {
            self.decoded = self.decoder.seek(index).expect("the seek works");
        }
        // A decoder that labels its frames wrong would never reach the frame
        iter::repeat_with(|| {
            let frames = self.decoder.decode().expect("the tone decodes");
            self.chunk = (self.decoded, frames);
            self.decoded += self.chunk.1.len();
            within(&self.chunk)
        })
        .take(SPAN)
        .flatten()
        .next()
        .unwrap_or_else(|| panic!("no frame at {index}"))
    }
}

#[test]
fn a_loop_wraps_as_a_straight_decode_does() {
    TONES.into_iter().for_each(|(name, bytes)| {
        let straight = straight(bytes);
        let end = straight.len() * 3 / 4;
        // One loop starts in the first packets, and one past the roll
        [300, 3000].into_iter().for_each(|start| {
            let mut playing = Playing::from(start, bytes);
            (0..3).for_each(|pass| {
                let found: Vec<Frame> = (start..end).map(|index| playing.frame(index)).collect();
                let worst = worst(&found, &straight[start..end]);
                assert!(
                    worst < TOLERANCE,
                    "{name}: pass {pass} from {start} is off by {worst}"
                );
            });
        });
    });
}
