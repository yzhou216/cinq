// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Why an output cannot play, which tells whether cinq mends it itself, as
//! it opens a device again every two seconds, or the show must change, as
//! for channels past those of the device

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Fault {
    /// The device is not there, as one unplugged or not started, and why
    Missing(String),
    /// The device is there but cannot open, and why
    Refused(String),
    /// The device has stopped while it played, and why
    Stopped(String),
    /// The channels of the output lie past the count that the device has
    Channels(u16),
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::Missing(reason) => formatter.write_str(reason),
            Fault::Refused(reason) => write!(formatter, "cannot open: {reason}"),
            Fault::Stopped(reason) => write!(formatter, "stopped: {reason}"),
            Fault::Channels(count) => {
                write!(
                    formatter,
                    "its channels lie past the {count} that the device has"
                )
            }
        }
    }
}
