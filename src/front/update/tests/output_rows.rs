// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the keys that move through rows acting on the outputs while they
//! show, by the code that the cue list uses, and never on the cue list out of
//! sight

mod in_view;
mod search;
mod stepping;

use super::{
    after,
    history::{cue, place},
    show,
};
use crate::{
    back::Place,
    front::{App, Message, Setting},
};

/// Returns the show with ten outputs showing, the first selected, over a cue
/// list that has the third cue selected.  The outputs are named for the
/// searches, from House L to Booth.
pub(super) fn outputs() -> App {
    let names = [
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
    let added = after(
        show(cue(3)),
        std::iter::once(Message::ShowOutputs)
            .chain((1..names.len()).map(|_| Message::Add(Place::After, 1))),
    );
    let named = names.iter().enumerate().fold(added, |app, (row, name)| {
        let id = app.workspace.outputs()[row].id;
        after(
            app,
            [Message::SettingTyped(
                Setting::OutputName(id),
                (*name).to_owned(),
            )],
        )
    });
    after(named, [Message::Row(1)])
}

/// Returns the row of the selected output, counted from one
pub(super) fn at(app: &App) -> usize {
    app.output_row().map_or(0, |row| row + 1)
}

/// Asserts that the cue list out of sight is where it was
pub(super) fn cue_list_kept(app: &App) {
    assert_eq!(place(app), (0, cue(3)));
    assert!(app.outputs_shown);
}
