// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Playback clock, which hands the back end the time between its ticks

use std::time::Instant;

use crate::{
    back::{Command, TICK},
    front::App,
};

impl App {
    /// Counts the time since the last tick.  Ticks of the timer and those
    /// that wake for pictures may come out of order, and an earlier one
    /// counts no time.
    pub(super) fn ticked(self, now: Instant) -> Self {
        let elapsed = self
            .last_tick
            .map_or(TICK, |last| now.saturating_duration_since(last));
        Self {
            last_tick: Some(self.last_tick.map_or(now, |last| last.max(now))),
            ..self.send(Command::Tick(elapsed.as_secs_f64()))
        }
    }
}
