// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Mixer of one audio device, which plays the sounds that play on it into
//! the channels of the device that their outputs name
//!
//! The device opens with the fewest channels that it offers at its default
//! sample rate, in 32-bit floating-point samples, that hold the channels
//! asked for.  Opening with the most it offers would ask 64 channels of the
//! plug devices of ALSA, which take any number.  A device with fewer
//! channels than asked for opens with all that it has, so that a typo in
//! the channels of one output silences that output alone.  The mix itself
//! runs on the thread of the stream, as [`mix`] tells.

#[cfg(test)]
mod bench;
mod mix;
mod stall;
#[cfg(test)]
mod tests;

use std::{
    iter,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use kira::backend::{
    Renderer,
    cpal::cpal::{
        self, BufferSize, Device, ErrorKind, SampleFormat, Stream, StreamConfig,
        traits::{DeviceTrait, HostTrait, StreamTrait},
    },
};
use rtrb::{Consumer, Producer, RingBuffer};

use super::{device, opener::Hold};
use mix::{Command, Gone};
pub(super) use mix::{Gains, Mix};
use stall::stalled;

/// Commands that can wait for the stream at once, far more than one
/// update of the workspace sends
const COMMANDS: usize = 4096;

/// Sounds that one device plays at once before its list of them grows on
/// the thread of the stream
const SOUNDS: usize = 1024;

pub(super) struct Mixer {
    /// Hold on the stream of the device, which plays as long as the mixer
    /// is kept, and which is absent only in a test that plays through a mix
    /// that it renders
    _hold: Option<Hold>,
    commands: Producer<Command>,
    gone: Consumer<Gone>,
    sample_rate: u32,
    channels: u16,
    /// Channels that the outputs needed as the device opened, which can be
    /// more than it has
    asked: u16,
    /// Latest error of the stream, which it tells from its own thread
    error: Arc<Mutex<Option<String>>>,
    /// Whether the stream has told an error after which it must be built
    /// anew
    lost: Arc<AtomicBool>,
    /// Moment the stream started, from which `called` counts
    started: Instant,
    /// Nanoseconds after `started` of the latest call of the stream for
    /// sound
    called: Arc<AtomicU64>,
    /// Identity that the next sound takes
    next: u64,
}

impl Mixer {
    /// Opens the device whose ID is `device`, or the only one named `name`
    /// on a machine that gives it another ID, or the default device without
    /// an ID, with `channels` channels or more
    pub(super) fn open(
        device: Option<&str>,
        name: Option<&str>,
        channels: u16,
    ) -> Result<(Self, Stream), String> {
        let found = match device {
            Some(id) => device::find(id, name)?,
            None => cpal::default_host()
                .default_output_device()
                .ok_or("there is no default audio device")?,
        };
        Self::start(&found, channels).map_err(|error| match device {
            Some(id) => format!("cannot play on {id}: {error}"),
            None => error,
        })
    }

    /// Returns how many channels the device whose ID is `device`, or the
    /// default device without one, would open with for `channels`, without
    /// opening it, or none where it is not there or cannot play
    pub(super) fn offers(device: Option<&str>, name: Option<&str>, channels: u16) -> Option<u16> {
        let found = match device {
            Some(id) => device::find(id, name).ok()?,
            None => cpal::default_host().default_output_device()?,
        };
        config(&found, channels).ok().map(|config| config.channels)
    }

    fn start(found: &Device, channels: u16) -> Result<(Self, Stream), String> {
        let config = config(found, channels)?;
        let (commands, taken) = RingBuffer::new(COMMANDS);
        let (handed, gone) = RingBuffer::new(COMMANDS);
        let mut mix = Mix::new(
            usize::from(config.channels),
            config.sample_rate,
            SOUNDS,
            taken,
            handed,
        );
        let error = Arc::new(Mutex::new(None));
        let told = Arc::clone(&error);
        let lost = Arc::new(AtomicBool::new(false));
        let ended = Arc::clone(&lost);
        let started = Instant::now();
        let called = Arc::new(AtomicU64::new(0));
        let calling = Arc::clone(&called);
        let stream = found
            .build_output_stream(
                config,
                move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    mix.render(out);
                    let since = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
                    calling.store(since, Ordering::Relaxed);
                },
                move |error: cpal::Error| {
                    if let Ok(mut latest) = told.lock() {
                        *latest = Some(error.to_string());
                    }
                    if ends(error.kind()) {
                        ended.store(true, Ordering::Relaxed);
                    }
                },
                None,
            )
            .map_err(|error| error.to_string())?;
        stream.play().map_err(|error| error.to_string())?;
        let mixer = Self {
            _hold: None,
            commands,
            gone,
            sample_rate: config.sample_rate,
            channels: config.channels,
            asked: channels,
            error,
            lost,
            started,
            called,
            next: 0,
        };
        Ok((mixer, stream))
    }

    /// Returns the mixer with `hold` on the stream that plays it, which
    /// closes the stream once the mixer goes
    pub(super) fn held(self, hold: Hold) -> Self {
        Self {
            _hold: Some(hold),
            ..self
        }
    }

    pub(super) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub(super) fn channels(&self) -> u16 {
        self.channels
    }

    pub(super) fn asked(&self) -> u16 {
        self.asked
    }

    /// Mixes the sound that `renderer` makes at `gains`, and returns the
    /// identity of the sound in the mixer
    pub(super) fn add(&mut self, renderer: Renderer, gains: Gains) -> Result<u64, String> {
        let id = self.next;
        self.send(Command::add(id, renderer, gains))?;
        self.next += 1;
        Ok(id)
    }

    /// Ramps the sound of `id` to `gains`
    pub(super) fn route(&mut self, id: u64, gains: Gains) {
        let _ = self.send(Command::Route(id, gains));
    }

    /// Fades out the sound of `id`, which then goes
    pub(super) fn remove(&mut self, id: u64) {
        let _ = self.send(Command::Remove(id));
    }

    /// Frees what the stream no longer needs
    pub(super) fn collect(&mut self) {
        iter::from_fn(|| self.gone.pop().ok()).for_each(drop);
    }

    /// Returns why the stream stopped, which is the latest error that it
    /// told, if any.  The stream has stopped if it told an error after which
    /// it must be built anew, or if it has asked for no sound for
    /// [`stall::STALL`].  Errors of the first kind tell at once, since a
    /// device that goes or takes another sample rate need not hold the cues.
    /// Other errors alone do not tell, since ALSA plays on after an underrun
    /// or a failed look at its buffer, where WASAPI ends the stream after any
    /// failed call.
    pub(super) fn failure(&self) -> Option<String> {
        // Stream that has called has done so after its start, so zero
        // nanoseconds mean that it has not
        let called = Some(self.called.load(Ordering::Relaxed))
            .filter(|nanos| *nanos > 0)
            .map(Duration::from_nanos);
        let stalled = stalled(self.started.elapsed(), called);
        (self.lost.load(Ordering::Relaxed) || stalled).then(|| {
            self.error
                .lock()
                .ok()
                .and_then(|error| error.clone())
                .unwrap_or_else(|| "the device stopped asking for sound".to_owned())
        })
    }

    fn send(&mut self, command: Command) -> Result<(), String> {
        self.commands
            .push(command)
            .map_err(|_| "the mixer of the device has more to do than it can take".to_owned())
    }
}

/// Returns the configuration of the stream of `device`: its default sample
/// rate, and the fewest channels that hold `channels`, or else all that it
/// has, in 32-bit floating-point samples, which kira renders.  A device of
/// fewer channels than an output needs still opens, so that the outputs
/// within its channels play, and only that output tells.
fn config(device: &Device, channels: u16) -> Result<StreamConfig, String> {
    let rate = device
        .default_output_config()
        .map_err(|error| error.to_string())?
        .sample_rate();
    let offered: Vec<u16> = device
        .supported_output_configs()
        .map_err(|error| error.to_string())?
        .filter(|range| {
            range.sample_format() == SampleFormat::F32
                && (range.min_sample_rate()..=range.max_sample_rate()).contains(&rate)
        })
        .map(|range| range.channels())
        .collect();
    let count = chosen(&offered, channels)
        .ok_or_else(|| format!("it takes no 32-bit floating-point samples at {rate} Hz"))?;
    Ok(StreamConfig {
        channels: count,
        sample_rate: rate,
        buffer_size: BufferSize::Default,
    })
}

/// Returns the count of channels to open from the counts that a device
/// `offered`: the fewest that hold `channels`, or else the most that it
/// offers, or none where it offers none
fn chosen(offered: &[u16], channels: u16) -> Option<u16> {
    offered
        .iter()
        .copied()
        .filter(|count| *count >= channels)
        .min()
        .or_else(|| offered.iter().copied().max())
}

/// Returns whether an error of `kind` means that the stream must be built
/// anew: that its device or host has gone, or that it no longer plays as
/// it was built to, as after a change of the sample rate of the device
fn ends(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::DeviceNotAvailable | ErrorKind::HostUnavailable | ErrorKind::StreamInvalidated
    )
}
