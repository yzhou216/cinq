// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! GO button, standby display, and show-wide playback controls

use iced::{
    Element, Fill,
    widget::{
        column, container, responsive, row, text,
        text::{LineHeight, Wrapping},
    },
};
use itertools::Itertools;

use super::{SPACING, TEXT_SIZE, button, char_width, faults, format, theme, widget::shortened};
use crate::front::{App, Message};

/// Size of the title, which names the standby cue
const TITLE: f32 = 24.0;

fn height_of(size: f32) -> f32 {
    f32::from(LineHeight::default().to_absolute(size.into()))
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let list = app.current();
    let standby = list.standby.and_then(|id| app.workspace.cue(id));
    // Summary keeps to three lines, so that the parts below stay where they are
    // as standby moves
    let lines = match standby {
        Some(cue) => {
            let notes = if cue.notes.trim().is_empty() {
                "none".to_owned()
            } else {
                cue.notes.lines().join(" ")
            };
            [
                format!("Standby: {cue}"),
                format!("Notes: {notes}"),
                format::chain(&app.workspace, list, cue, &faults::failing(app)),
            ]
        }
        None => [
            "Standby: end of list".to_owned(),
            "Notes: none".to_owned(),
            "GO fires nothing".to_owned(),
        ],
    };
    let [title, notes, chain] = lines;
    let paused = app.workspace.active().iter().any(|active| active.paused);
    let buttons = row![
        button("Pause all").on_press_maybe(app.running().then_some(Message::PauseAll)),
        button("Resume all").on_press_maybe(paused.then_some(Message::ResumeAll)),
        button("Panic").style(theme::panic).on_press(Message::Panic),
    ]
    .spacing(SPACING);
    // The notes and the chain take the width under the title and the buttons
    let summary = column![
        row![line(title, TITLE), buttons].spacing(SPACING),
        line(notes, TEXT_SIZE.0),
        line(chain, TEXT_SIZE.0),
    ];
    // GO still presses with nothing to fire, since it steps standby past cues that
    // cannot play, but it looks as a disabled button does
    let fires = app.workspace.next_go(list).next().is_some();
    row![
        button(text("GO").size(40))
            .style(if fires {
                theme::go
            } else {
                theme::go_without_cues
            })
            .on_press_maybe(standby.map(|_| Message::Go)),
        container(summary).width(Fill).clip(true),
    ]
    .spacing(SPACING)
    .padding(SPACING)
    .into()
}

/// Returns `told` of text `size`, which ends in an ellipsis where the room
/// runs out, so that the operator sees that a note goes on
fn line<'a>(told: String, size: f32) -> Element<'a, Message> {
    responsive(move |room| {
        let columns = (room.width / (char_width() * size / TEXT_SIZE.0)) as usize;
        text(shortened(&told, columns))
            .size(size)
            .wrapping(Wrapping::None)
            .into()
    })
    .height(height_of(size))
    .into()
}
