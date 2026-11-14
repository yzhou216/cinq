// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Mixer of one audio device, which plays the sounds that play on it into
//! the channels of the device that their outputs name
//!
//! The device opens with the fewest channels that it offers at its default
//! sample rate that hold the channels asked for, in the first samples of
//! [`config::FORMATS`] that it takes.  Opening with the most it offers would ask 64
//! channels of the plug devices of ALSA, which take any number.  The mix
//! itself runs on the thread of the stream, as [`mix`] tells.

mod config;
mod mix;
mod stream;
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
        self, Device, I24, SampleFormat, Stream,
        traits::{HostTrait, StreamTrait},
    },
};
use rtrb::{Consumer, Producer, RingBuffer};

use super::device;
use config::config;
use mix::{Command, Gone};
pub(super) use mix::{Gains, Mix};
use stream::{Tells, stream};

/// Commands that can wait for the stream at once, far more than one
/// update of the workspace sends
const COMMANDS: usize = 4096;

/// Sounds that one device plays at once before its list of them grows on
/// the thread of the stream
const SOUNDS: usize = 1024;

/// Time without a call for sound after which a stream that has called
/// counts as stopped, longer than the 1.9 seconds that a new stream on the
/// ALSA plugin of PulseAudio waits after its first calls
const STALL: Duration = Duration::from_secs(3);

/// Time that a new stream has for its first call for sound, longer than a
/// device that is slow to wake, as a Bluetooth sink that sleeps, takes
const STARTING: Duration = Duration::from_secs(10);

pub(super) struct Mixer {
    /// Stream of the device, which plays as long as the mixer is kept
    /// Absent only in a test, which plays through a mix that it renders
    _stream: Option<Stream>,
    commands: Producer<Command>,
    gone: Consumer<Gone>,
    sample_rate: u32,
    channels: u16,
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
    ) -> Result<Self, String> {
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

    fn start(found: &Device, channels: u16) -> Result<Self, String> {
        let (config, format) = config(found, channels)?;
        let (commands, taken) = RingBuffer::new(COMMANDS);
        let (handed, gone) = RingBuffer::new(COMMANDS);
        let mix = Mix::new(
            usize::from(config.channels),
            config.sample_rate,
            SOUNDS,
            taken,
            handed,
        );
        let error = Arc::new(Mutex::new(None));
        let lost = Arc::new(AtomicBool::new(false));
        let started = Instant::now();
        let called = Arc::new(AtomicU64::new(0));
        let tells = Tells {
            error: Arc::clone(&error),
            lost: Arc::clone(&lost),
            started,
            called: Arc::clone(&called),
        };
        let stream = match format {
            SampleFormat::I32 => stream::<i32>(found, config, mix, tells),
            SampleFormat::I24 => stream::<I24>(found, config, mix, tells),
            SampleFormat::I16 => stream::<i16>(found, config, mix, tells),
            _ => stream::<f32>(found, config, mix, tells),
        }?;
        stream.play().map_err(|error| error.to_string())?;
        Ok(Self {
            _stream: Some(stream),
            commands,
            gone,
            sample_rate: config.sample_rate,
            channels: config.channels,
            error,
            lost,
            started,
            called,
            next: 0,
        })
    }

    pub(super) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub(super) fn channels(&self) -> u16 {
        self.channels
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

    /// Returns why the stream stopped, if it has told an error after which
    /// it must be built anew, or has asked for no sound for [`STALL`]: the
    /// latest error that it told, if any.  Errors of the first kind tell at
    /// once, since an ASIO driver that asks to be reset may play on, and a
    /// device that goes or takes another sample rate need not hold the
    /// cues.  Other errors alone do not tell, since ALSA plays on after an
    /// underrun or a failed look at its buffer, where WASAPI ends the
    /// stream after any failed call.
    pub(super) fn failure(&self) -> Option<String> {
        // A stream that has called has done so after its start, so zero
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

/// Returns whether a stream that started `elapsed` ago has stalled, as it
/// asked for no sound for [`STALL`] since its latest call at `called`, or
/// for [`STARTING`] since it started if it has not called yet
fn stalled(elapsed: Duration, called: Option<Duration>) -> bool {
    match called {
        Some(called) => elapsed.saturating_sub(called) >= STALL,
        None => elapsed >= STARTING,
    }
}

#[cfg(test)]
impl Mixer {
    /// Returns a mixer without a device, whose ring of commands holds
    /// `capacity`, and the mix that the stream of a device would run
    pub(super) fn bench(channels: u16, sample_rate: u32, capacity: usize) -> (Self, Mix) {
        let (commands, taken) = RingBuffer::new(capacity);
        let (handed, gone) = RingBuffer::new(COMMANDS);
        let mix = Mix::new(usize::from(channels), sample_rate, 8, taken, handed);
        let mixer = Self {
            _stream: None,
            commands,
            gone,
            sample_rate,
            channels,
            error: Arc::default(),
            lost: Arc::default(),
            started: Instant::now(),
            called: Arc::default(),
            next: 0,
        };
        (mixer, mix)
    }
}
