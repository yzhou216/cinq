// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Entry of a running cue in the sidebar: its name, a progress bar, how it
//! runs, and the Stop button

use iced::{
    Element, Fill,
    widget::{button::DEFAULT_PADDING, column, progress_bar, row, text, text::Wrapping},
};
use unicode_width::UnicodeWidthStr;

use super::super::{SPACING, button, char_width, format, widget::shortened};
use crate::{
    back::{Active, Cue, Kind},
    front::Message,
};

/// Thickness of the progress bars of running cues, thin enough that several
/// running cues fit in the sidebar at once
const BAR: f32 = 10.0;

/// Returns the name of a running cue and how it runs, each cut to its room
/// at the end, so that an entry keeps its three lines as a fade starts, and
/// the time left and the level, which come first, outlast the words after
/// them.  The name leaves room for the Stop button.  A cue that is `silent`,
/// as every output it sends to fails, says so in place of how it runs.
pub(super) fn entry(
    active: &Active,
    cue: &Cue,
    target: Option<&Cue>,
    columns: usize,
    silent: bool,
) -> (String, String) {
    let stop = (4.0 + (2.0 * DEFAULT_PADDING.left + SPACING) / char_width()).ceil();
    let how = if silent {
        "silent: output failed".to_owned()
    } else {
        with_target(format::activity(active, cue), cue, target, columns)
    };
    (
        shortened(&cue.to_string(), columns.saturating_sub(stop as usize)),
        shortened(&how, columns),
    )
}

pub(super) fn active_cue<'a>(
    active: &Active,
    cue: &Cue,
    target: Option<&Cue>,
    columns: usize,
    silent: bool,
) -> Element<'a, Message> {
    let (name, how) = entry(active, cue, target, columns, silent);
    column![
        row![
            text(name).width(Fill).wrapping(Wrapping::None),
            button("Stop").on_press(Message::Stop(cue.id)),
        ]
        .spacing(SPACING),
        progress_bar(0.0..=1.0, active.progress()).girth(BAR),
        text(how)
            .wrapping(Wrapping::None)
            .style(if active.paused || silent {
                text::warning
            } else {
                text::success
            }),
    ]
    .spacing(SPACING / 2.0)
    .into()
}

/// Returns `how` with the cue that a fade or devamp cue acts on, named as the
/// cue table names a target, so that two running fades tell apart.  The
/// words are left out when they would not fit whole in `columns`, as a cut
/// name would name no cue.
fn with_target(how: String, cue: &Cue, target: Option<&Cue>, columns: usize) -> String {
    let verb = match cue.kind {
        Kind::Fade => "fades",
        Kind::Devamp => "devamps",
        _ => return how,
    };
    target
        .map(|target| format!("{how}, {verb} {}", format::number(target)))
        .filter(|told| told.width() <= columns)
        .unwrap_or(how)
}
