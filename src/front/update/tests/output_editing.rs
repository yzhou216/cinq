// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the keys that edit cues editing the outputs while they show: the
//! register of outputs, the identity that a cut output takes back, the
//! notices, undo and redo, and the keys that enter the name

mod cutting;
mod notices;
mod registers;

use super::{after, output_rows::outputs, paging::letter, press};
use crate::front::{App, Message};

/// Types the keys one after the other, each a character without modifiers
pub(super) fn typed(app: App, keys: &str) -> App {
    keys.chars().fold(app, |app, key| {
        app.typed(press(letter(&key.to_string())), false).0
    })
}

/// Returns what the status bar tells of the last copy or cut, and whether it was a cut
pub(super) fn clipped(app: &App) -> Option<(&str, bool)> {
    app.clipped
        .as_ref()
        .map(|clipped| (clipped.told.as_str(), clipped.cut))
}

pub(super) fn names(app: &App) -> Vec<&str> {
    app.workspace
        .outputs()
        .iter()
        .map(|output| output.name.as_str())
        .collect()
}

/// Returns the app with the outputs showing and the row counted from one
/// selected
pub(super) fn on(row: usize) -> App {
    after(outputs(), [Message::Row(row)])
}

fn locked(app: App) -> App {
    after(app, [Message::ShowMode(true)])
}

const ALL: [&str; 10] = [
    "House L",
    "House R",
    "Sub",
    "Monitor",
    "Front fill",
    "Balcony",
    "Lobby",
    "Stage",
    "FX",
    "Booth",
];

fn without(rows: &[usize]) -> Vec<&'static str> {
    ALL.iter()
        .enumerate()
        .filter(|(row, _)| !rows.contains(row))
        .map(|(_, name)| *name)
        .collect()
}
