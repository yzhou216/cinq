// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Mixing on the thread of the stream of a device
//!
//! The thread must never wait, so it takes no lock and frees no memory.
//! The main thread sends it sounds and their gains over one ring, and it
//! hands back over another what it no longer needs, for the main thread to
//! free.  A change of gains ramps over ten milliseconds, and a sound that
//! goes fades out over as many, so that neither clicks.

use kira::backend::Renderer;
use rtrb::{Consumer, Producer};

/// Gain from the left and the right of a sound into each channel of a
/// device, in order
pub(in crate::back::audio) type Gains = Box<[[f32; 2]]>;

/// Frames that a sound renders at a time, which the scratch buffer holds
const CHUNK: usize = 512;

/// Sound that the stream mixes, by the identity that the main thread gave
/// it
pub(super) struct Slot {
    id: u64,
    renderer: Renderer,
    /// Gains as they stand, which ramp toward `target`
    gains: Gains,
    target: Gains,
    /// Frames left of the ramp
    ramp: usize,
    /// Whether the sound fades out and goes once its ramp ends
    leaving: bool,
}

pub(super) enum Command {
    Add(Box<Slot>),
    Route(u64, Gains),
    Remove(u64),
}

impl Command {
    pub(super) fn add(id: u64, renderer: Renderer, gains: Gains) -> Self {
        Self::Add(Box::new(Slot {
            id,
            renderer,
            gains: gains.clone(),
            target: gains,
            ramp: 0,
            leaving: false,
        }))
    }
}

/// What the stream no longer needs, which the main thread frees
pub(super) enum Gone {
    Slot(#[allow(dead_code)] Box<Slot>),
    Gains(#[allow(dead_code)] Gains),
}

pub(in crate::back::audio) struct Mix {
    channels: usize,
    /// Frames over which a sound fades out, as long as the default fade of kira
    ramp: usize,
    /// Sounds in the boxes that the main thread made, which stay boxed so
    /// that this thread neither frees nor moves them
    #[allow(clippy::vec_box)]
    slots: Vec<Box<Slot>>,
    /// Stereo frames of one chunk of a sound
    scratch: Box<[f32]>,
    commands: Consumer<Command>,
    gone: Producer<Gone>,
}

impl Mix {
    /// `capacity` sounds fit before the list of sounds grows, which would
    /// take memory on the thread of the stream.  Sounds fade out over ten
    /// milliseconds of the `rate` of the device.
    pub(super) fn new(
        channels: usize,
        rate: u32,
        capacity: usize,
        commands: Consumer<Command>,
        gone: Producer<Gone>,
    ) -> Self {
        Self {
            channels,
            ramp: rate as usize / 100,
            slots: Vec::with_capacity(capacity),
            scratch: vec![0.0; CHUNK * 2].into_boxed_slice(),
            commands,
            gone,
        }
    }

    /// Fills `out`, frames of interleaved channels, with every sound
    pub(in crate::back::audio) fn render(&mut self, out: &mut [f32]) {
        self.obey();
        out.fill(0.0);
        let channels = self.channels;
        let scratch = &mut self.scratch;
        self.slots.iter_mut().for_each(|slot| {
            slot.renderer.on_start_processing();
            out.chunks_mut(CHUNK * channels).for_each(|chunk| {
                let stereo = &mut scratch[..chunk.len() / channels * 2];
                slot.renderer.process(stereo, 2);
                slot.mix_into(chunk, channels, stereo);
            });
        });
        self.part();
    }

    fn obey(&mut self) {
        let ramp = self.ramp;
        while let Ok(command) = self.commands.pop() {
            match command {
                Command::Add(slot) => self.slots.push(slot),
                Command::Route(id, gains) => {
                    let old = match self.slot(id) {
                        Some(slot) if !slot.leaving => {
                            slot.ramp = ramp;
                            std::mem::replace(&mut slot.target, gains)
                        }
                        _ => gains,
                    };
                    let _ = self.gone.push(Gone::Gains(old));
                }
                Command::Remove(id) => {
                    if let Some(slot) = self.slot(id) {
                        slot.target.fill([0.0; 2]);
                        slot.ramp = ramp;
                        slot.leaving = true;
                    }
                }
            }
        }
    }

    fn slot(&mut self, id: u64) -> Option<&mut Slot> {
        self.slots
            .iter_mut()
            .find(|slot| slot.id == id)
            .map(|slot| &mut **slot)
    }

    /// Hands back the sounds that have faded out.  A full ring drops them
    /// here instead, which no ring of this size comes to.
    fn part(&mut self) {
        while let Some(index) = self
            .slots
            .iter()
            .position(|slot| slot.leaving && slot.ramp == 0)
        {
            let _ = self.gone.push(Gone::Slot(self.slots.swap_remove(index)));
        }
    }
}

impl Slot {
    /// Adds the stereo frames of `stereo` into `out`, frames of `channels`
    /// channels, at the gains of the slot
    fn mix_into(&mut self, out: &mut [f32], channels: usize, stereo: &[f32]) {
        let left = self.ramp;
        let span = left.min(stereo.len() / 2);
        self.gains
            .iter_mut()
            .zip(self.target.iter())
            .enumerate()
            .filter(|(_, (from, to))| **from != [0.0; 2] || **to != [0.0; 2])
            .for_each(|(channel, (from, to))| {
                let start = *from;
                let step = if left == 0 {
                    [0.0; 2]
                } else {
                    [
                        (to[0] - start[0]) / left as f32,
                        (to[1] - start[1]) / left as f32,
                    ]
                };
                let at = |frame: usize| {
                    if frame < span {
                        let done = (frame + 1) as f32;
                        [start[0] + step[0] * done, start[1] + step[1] * done]
                    } else {
                        *to
                    }
                };
                out.iter_mut()
                    .skip(channel)
                    .step_by(channels)
                    .zip(stereo.as_chunks::<2>().0)
                    .enumerate()
                    .for_each(|(frame, (sample, sound))| {
                        let gain = at(frame);
                        *sample += sound[0] * gain[0] + sound[1] * gain[1];
                    });
                *from = match span {
                    _ if span == left => *to,
                    0 => start,
                    _ => at(span - 1),
                };
            });
        self.ramp -= span;
    }
}

#[cfg(test)]
impl Mix {
    /// Returns how many sounds the mix holds, those that fade out included
    pub(in crate::back::audio) fn sounds(&self) -> usize {
        self.slots.len()
    }
}
