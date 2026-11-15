// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Scrolling the cue list a row at a time, with the keys of Vim
//!
//! Like paging, it measures the list as it is laid out, and takes the rows
//! to be as tall as the average, since the rows that a long name wraps are
//! taller than the rest.

use iced::Task;

use super::paging::{Landing, viewport::Shape};
use crate::front::{App, Message};

/// Slack for the rounding of a row height that is the quotient of a sum
const SLACK: f32 = 0.001;

impl App {
    /// Scrolls the cue list by `rows`, downwards when positive, and keeps
    /// the selection on its cue unless that leaves the view, when it moves to
    /// the nearest row in view, as the cursor does in Vim.
    pub(super) fn scrolled(self, rows: isize) -> (Self, Task<Message>) {
        let (cues, current) = (self.current().cues.len(), self.selected_row());
        let (list, from) = (self.list, self.selected);
        self.landing(
            move |row| Message::Landed { list, from, row },
            move |shape| scroll(rows, shape, cues, current),
        )
    }
}

/// Returns the first row, counted from zero, that is in view whole
fn first_in_view(shape: Shape, row_height: f32) -> usize {
    (shape.scrolled / row_height - SLACK).ceil().max(0.0) as usize
}

/// Returns the last row, counted from zero, that is in view whole, or the
/// first when not even one row fits
fn last_in_view(shape: Shape, row_height: f32) -> usize {
    let bottom = ((shape.scrolled + shape.view) / row_height + SLACK).floor() as usize;
    bottom
        .saturating_sub(1)
        .max(first_in_view(shape, row_height))
}

/// Returns where scrolling a list of `cues` rows by `rows` leaves the view
/// and the selection on row `current`, stopping at either end of the list
pub(super) fn scroll(
    rows: isize,
    shape: Shape,
    cues: usize,
    current: Option<usize>,
) -> Option<Landing> {
    let last = cues.checked_sub(1)?;
    let row_height = shape.content / cues as f32;
    let wanted = shape.scrolled + rows as f32 * row_height;
    let offset = wanted.clamp(0.0, (shape.content - shape.view).max(0.0));
    let moved = Shape {
        scrolled: offset,
        ..shape
    };
    let row = current.unwrap_or(0).clamp(
        first_in_view(moved, row_height),
        last_in_view(moved, row_height),
    );
    Some(Landing {
        offset,
        row: row.min(last),
    })
}
