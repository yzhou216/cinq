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
mod history;
mod inputs;
mod loading;
mod operators;
mod operators_pasting;
mod paging;
mod scrolling;
mod selection;
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

fn stays_open(key: Key) -> bool {
    let open = App {
        help: true,
        ..App::default()
    };
    open.typed(press(key), false).0.help
}

#[test]
fn help_page_closes_on_its_keys_and_on_go_and_panic() {
    assert!(!stays_open(Key::Character("q".into())));
    assert!(!stays_open(Key::Character("?".into())));
    assert!(!stays_open(Key::Named(Named::Space)));
    assert!(!stays_open(Key::Named(Named::Escape)));
    assert!(stays_open(Key::Character("j".into())));
}

#[test]
fn keys_that_move_the_selection_scroll_the_open_help_page() {
    let typed = |keys: &[&str], help: bool| {
        let app = App {
            help,
            ..show(Some(CueId(2)))
        };
        let (after, task) = keys.iter().fold((app, Task::none()), |(app, _), &key| {
            app.typed(press(Key::Character(key.into())), false)
        });
        (after.selected, task.units() > 0)
    };
    // Scrolling moves no selection, which the keys move with the page shut
    ["j", "k", "G"]
        .into_iter()
        .for_each(|key| assert_eq!(typed(&[key], true), (Some(CueId(2)), true), "{key}"));
    assert_eq!(typed(&["g", "g"], true), (Some(CueId(2)), true));
    assert_eq!(typed(&["j"], false).0, Some(CueId(3)));
    // Other keys that act on the cues do nothing over the page
    assert_eq!(typed(&["o"], true), (Some(CueId(2)), false));
}

#[test]
fn keys_that_act_in_fields_do_nothing_over_the_help_page_but_go_still_fires() {
    let typed = |help: bool, press| App { help, ..show(None) }.typed(press, false);
    field_keys().into_iter().for_each(|key| {
        let (over, ignored) = typed(true, key);
        assert_eq!((over.help, ignored.units()), (true, 0), "{key:?}");
    });
    // Keys of opening and saving would build a file chooser, which rfd
    // builds only on the main thread on Darwin, so only the moves between
    // fields are fired with the page shut
    field_keys()[5..].iter().for_each(|&key| {
        let (_, acting) = typed(false, key);
        assert_eq!(acting.units(), 1, "{key:?}");
    });
    let standby = |app: &App| app.workspace.lists()[0].standby;
    let (open, _) = typed(true, press(Key::Named(Named::Space)));
    assert_eq!(standby(&show(None)), Some(CueId(1)));
    assert_eq!((open.help, standby(&open)), (false, Some(CueId(2))));
}

#[test]
fn help_page_comes_and_goes_without_touching_the_main_window() {
    // What iced keeps in widgets, the field with the keyboard and the
    // scroll of the cue list, stays in the widgets that the view hides
    // under the page, so that no task has anything to save or to restore
    let app = App {
        list: 1,
        tab: Tab::Levels,
        ..show(Some(CueId(20)))
    };
    let (open, opening) = app.next(Message::ToggleHelp);
    assert_eq!((open.help, opening.units()), (true, 0));
    let (closed, closing) = open.next(Message::CloseHelp);
    assert_eq!((closed.help, closing.units()), (false, 0));
    assert_eq!(
        (closed.list, closed.tab, closed.selected),
        (1, Tab::Levels, Some(CueId(20)))
    );
}
