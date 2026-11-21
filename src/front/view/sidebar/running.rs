// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Entry of a running cue in the sidebar: its name, a progress bar, how it
//! runs, and the Stop button

use iced::{
    Element, Fill,
    widget::{button::DEFAULT_PADDING, column, progress_bar, row, text, text::Wrapping},
};

use super::super::{SPACING, button, char_width, format, widget::shortened};
use crate::{
    back::{Active, Cue},
    front::Message,
};

/// Thickness of the progress bars of running cues, thin enough that several
/// running cues fit in the sidebar at once
const BAR: f32 = 10.0;

/// Returns the name of a running cue and how it runs, each cut to its room
/// at the end, so that an entry keeps its three lines as a fade starts, and
/// the time left and the level, which come first, outlast the words after
/// them.  The name leaves room for the Stop button.
pub(super) fn entry(active: &Active, cue: &Cue, columns: usize) -> (String, String) {
    let stop = (4.0 + (2.0 * DEFAULT_PADDING.left + SPACING) / char_width()).ceil();
    (
        shortened(&cue.to_string(), columns.saturating_sub(stop as usize)),
        shortened(&format::activity(active, cue), columns),
    )
}

pub(super) fn active_cue<'a>(active: &Active, cue: &Cue, columns: usize) -> Element<'a, Message> {
    let (name, how) = entry(active, cue, columns);
    column![
        row![
            text(name).width(Fill).wrapping(Wrapping::None),
            button("Stop").on_press(Message::Stop(cue.id)),
        ]
        .spacing(SPACING),
        progress_bar(0.0..=1.0, active.progress()).girth(BAR),
        text(how).wrapping(Wrapping::None).style(if active.paused {
            text::warning
        } else {
            text::success
        }),
    ]
    .spacing(SPACING / 2.0)
    .into()
}
