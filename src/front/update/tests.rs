// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the update cycle: the keys that act while the help page is
//! open, the help page that comes and goes without touching the main
//! window, the selection after deleting or undo takes the selected cue
//! away, the selection that follows standby, the sound file chosen for a
//! new cue, a level typed halfway, a link of the help page that fails to
//! open, and the clipboard

mod adding;
mod cancel;
mod clipboard;
mod clipped;
mod clock;
mod counts;
mod help_page;
mod history;
mod inputs;
mod loading;
mod matches;
mod operators;
mod operators_pasting;
mod paging;
mod scrolling;
mod search;
mod search_columns;
mod search_line;
mod search_places;
mod selection;
mod side_buttons;
mod viewing;

use std::ops::RangeInclusive;

use iced::{
    Task,
    keyboard::{Key, Modifiers, key::Named},
};

use crate::{
    back::{CueId, CueList, Place, Workspace, audio},
    front::{
        App, Message, Tab,
        keymap::{Press, sequence},
        view::format,
    },
};

/// Returns the state of the interface with `selected` selected in a show of
/// two lists of audio cues, numbered as they are identified: twelve in
/// Main, and two from twenty in Preshow
fn show(selected: Option<CueId>) -> App {
    let cues = |ids: RangeInclusive<u32>| ids.map(audio).collect();
    App {
        workspace: Workspace::new(vec![
            CueList::new("Main", cues(1..=12)),
            CueList::new("Preshow", cues(20..=21)),
        ]),
        selected,
        ..App::default()
    }
}

pub(super) fn chord(key: Key, modifiers: Modifiers) -> Press {
    Press::from_key(&key, modifiers).expect("keys in the tests are named")
}

pub(super) fn press(key: Key) -> Press {
    chord(key, Modifiers::empty())
}

/// Returns how the latest change of the show reads in its notice
pub(super) fn told(app: &App) -> String {
    let (_, change) = app.workspace.latest_change().expect("a change");
    format::change(&app.workspace, change)
}

/// Returns the state after the messages, which a test sends one at a time
pub(super) fn after(app: App, messages: impl IntoIterator<Item = Message>) -> App {
    messages
        .into_iter()
        .fold(app, |app, message| app.next(message).0)
}

/// Types the keys one after another, as the window delivers them, and returns
/// the state with the task that the last key started.  The keys are written as
/// the configuration file writes a sequence, apart by spaces, as in `3 o` or
/// `Control+r`.
pub(super) fn typing_task(app: App, keys: &str) -> (App, Task<Message>) {
    sequence(keys)
        .expect("keys in the tests are valid")
        .into_iter()
        .fold((app, Task::none()), |(app, _), press| {
            app.typed(press, false)
        })
}

/// Returns the state after the keys of [`typing_task`]
pub(super) fn typing(app: App, keys: &str) -> App {
    typing_task(app, keys).0
}

/// Returns the keys that act in text fields, apart from those of GO and
/// Panic: those that open and save shows, go back and forward, and move
/// between fields
pub(super) fn field_keys() -> [Press; 7] {
    let tab = || Key::Named(Named::Tab);
    let control = |key: &str| chord(Key::Character(key.into()), Modifiers::CTRL);
    [
        control("l"),
        control("s"),
        control("S"),
        control("o"),
        control("i"),
        press(tab()),
        chord(tab(), Modifiers::SHIFT),
    ]
}
