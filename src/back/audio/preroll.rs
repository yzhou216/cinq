// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sound files decoded right from the first frame after a seek
//!
//! AAC and MP3 decode each frame from the frames before it, but kira seeks
//! symphonia straight to the frame it wants, so the first frame or two that
//! come out are wrong.  kira seeks like this where a sound starts partway
//! into its file and at each wrap of a loop, so the wrong frames play as a
//! short burst of noise.  [`Prerolled`] takes the place of kira's decoder: on
//! a seek it goes [`ROLL`] frames earlier, starts the decoder afresh, and
//! drops what it decodes up to the frame that kira asked for.  kira keeps its
//! own decoder out of reach, so this one drives symphonia itself.

#[cfg(test)]
mod tests;

use std::{fs::File, path::Path};

use kira::{
    Frame,
    sound::{
        FromFileError,
        streaming::{Decoder, StreamingSoundData},
    },
};
use symphonia::core::{
    audio::GenericAudioBufferRef,
    codecs::{CodecParameters, audio::AudioDecoder},
    formats::{FormatReader, SeekMode, SeekTo, TrackType, probe::Hint},
    io::{MediaSource, MediaSourceStream},
    units::Timestamp,
};

/// Frames to decode before the one that a seek asks for, which is more than
/// a packet of AAC or MP3 holds, 1024 frames and 1152, so that a whole packet
/// comes first to bring the decoder to the state that the next one needs
const ROLL: usize = 2048;

/// Opens the sound file at `path` for kira to stream
pub(super) fn open(path: &Path) -> Result<StreamingSoundData<FromFileError>, FromFileError> {
    Prerolled::new(Box::new(File::open(path)?)).map(StreamingSoundData::from_decoder)
}

struct Prerolled {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track: u32,
    sample_rate: u32,
    num_frames: usize,
    /// Frames that a seek decoded from the frame it asked for to the end of
    /// their packet, which the next decoding hands out first
    ahead: Vec<Frame>,
}

impl Prerolled {
    fn new(source: Box<dyn MediaSource>) -> Result<Self, FromFileError> {
        let stream = MediaSourceStream::new(source, Default::default());
        let reader = symphonia::default::get_probe().probe(
            &Hint::default(),
            stream,
            Default::default(),
            Default::default(),
        )?;
        let track = reader
            .default_track(TrackType::Audio)
            .ok_or(FromFileError::NoDefaultTrack)?;
        let Some(CodecParameters::Audio(params)) = track.codec_params.as_ref() else {
            return Err(FromFileError::NoDefaultTrack);
        };
        let num_frames = track.num_frames.ok_or(FromFileError::UnknownDuration)?;
        Ok(Self {
            decoder: symphonia::default::get_codecs()
                .make_audio_decoder(params, &Default::default())?,
            track: track.id,
            sample_rate: params.sample_rate.ok_or(FromFileError::UnknownSampleRate)?,
            num_frames: usize::try_from(num_frames).map_err(|_| FromFileError::UnknownDuration)?,
            reader,
            ahead: Vec::new(),
        })
    }

    /// Decodes the next packet of the track, and returns the index of its
    /// first frame with the frames, or nothing at the end of the file
    ///
    /// The index comes from the packet, as adding up the frames decoded since
    /// a seek falls short: the first packet that an Ogg Vorbis decoder decodes
    /// after a reset has no frames, though the file counts frames for it.
    fn packet(&mut self) -> Result<Option<(usize, Vec<Frame>)>, FromFileError> {
        let packet = loop {
            match self.reader.next_packet()? {
                Some(packet) if packet.track_id == self.track => break packet,
                Some(_) => {}
                None => return Ok(None),
            }
        };
        let at = packet.pts.saturating_add(packet.trim_start).get();
        Ok(Some((
            usize::try_from(at).unwrap_or(0),
            frames(self.decoder.decode(&packet)?)?,
        )))
    }
}

fn frames(buffer: GenericAudioBufferRef<'_>) -> Result<Vec<Frame>, FromFileError> {
    let mut planes = Vec::new();
    buffer.copy_to_vecs_planar::<f32>(&mut planes);
    match planes.as_slice() {
        [mono] => Ok(mono
            .iter()
            .map(|&sample| Frame::from_mono(sample))
            .collect()),
        [left, right] => Ok(left
            .iter()
            .zip(right)
            .map(|(&left, &right)| Frame::new(left, right))
            .collect()),
        _ => Err(FromFileError::UnsupportedChannelConfiguration),
    }
}

impl Decoder for Prerolled {
    type Error = FromFileError;

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn num_frames(&self) -> usize {
        self.num_frames
    }

    fn decode(&mut self) -> Result<Vec<Frame>, Self::Error> {
        if self.ahead.is_empty() {
            Ok(self.packet()?.map(|(_, frames)| frames).unwrap_or_default())
        } else {
            Ok(std::mem::take(&mut self.ahead))
        }
    }

    /// Seeks so that the next decoding starts at the frame at `index` and
    /// decodes right
    fn seek(&mut self, index: usize) -> Result<usize, Self::Error> {
        self.ahead = Vec::new();
        self.reader.seek(
            SeekMode::Accurate,
            SeekTo::Timestamp {
                ts: Timestamp::new(index.saturating_sub(ROLL) as i64),
                track_id: self.track,
            },
        )?;
        // The decoder has to start from nothing, as it would at the start of
        // the file, or what it kept of the frames before the seek would mix
        // into the first frames after it
        self.decoder.reset();
        let (at, frames) = loop {
            match self.packet()? {
                None => return Ok(index.min(self.num_frames)),
                Some((at, frames)) if !frames.is_empty() && at + frames.len() > index => {
                    break (at, frames);
                }
                Some(_) => {}
            }
        };
        self.ahead = frames[index.saturating_sub(at)..].to_vec();
        Ok(at.max(index))
    }
}
