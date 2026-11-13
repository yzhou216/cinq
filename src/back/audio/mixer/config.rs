// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sample formats and the configuration that a device opens with

use kira::backend::cpal::cpal::{
    BufferSize, Device, SampleFormat, StreamConfig, traits::DeviceTrait,
};

/// Samples that a device can take, those that kira renders first, then
/// integers from the widest, as ASIO drivers, the hw devices of ALSA, and
/// Inferno's Dante device take
pub(super) const FORMATS: [SampleFormat; 4] = [
    SampleFormat::F32,
    SampleFormat::I32,
    SampleFormat::I24,
    SampleFormat::I16,
];

/// Returns the configuration of the stream of `device`: its default sample
/// rate, and the fewest channels that hold `channels` in the first of
/// [`FORMATS`] that it takes, with that format
pub(super) fn config(
    device: &Device,
    channels: u16,
) -> Result<(StreamConfig, SampleFormat), String> {
    let rate = device
        .default_output_config()
        .map_err(|error| error.to_string())?
        .sample_rate();
    let ranges: Vec<_> = device
        .supported_output_configs()
        .map_err(|error| error.to_string())?
        .filter(|range| {
            (range.min_sample_rate()..=range.max_sample_rate()).contains(&rate)
                && range.channels() >= channels
        })
        .collect();
    let (format, fewest) = FORMATS
        .into_iter()
        .find_map(|format| {
            let fewest = ranges
                .iter()
                .filter(|range| range.sample_format() == format)
                .map(|range| range.channels())
                .min()?;
            Some((format, fewest))
        })
        .ok_or_else(|| {
            format!("it takes no samples that cinq can play on {channels} channels at {rate} Hz")
        })?;
    let config = StreamConfig {
        channels: fewest,
        sample_rate: rate,
        buffer_size: BufferSize::Default,
    };
    Ok((config, format))
}
