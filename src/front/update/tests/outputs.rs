// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the outputs view: reaching it past the last cue list, the keys
//! of cues acting on outputs there, the messages for rows that act on nothing
//! there, and the settings typed into fields

mod reaching;
mod settings;

use crate::front::App;

pub(super) fn names(app: &App) -> Vec<&str> {
    app.workspace
        .outputs()
        .iter()
        .map(|output| output.name.as_str())
        .collect()
}
