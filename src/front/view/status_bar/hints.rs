// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The hints of the status bar: the keys typed so far, what the latest
//! change told, and the keys that can act now

use std::iter;

use iced::keyboard::{Key, Modifiers, key::Named};
use itertools::Itertools;

use super::{LINE, format};
use crate::{
    back::{Change, Turn},
    front::{
        App,
        keymap::{Action, Press, shown},
        update::Spot,
        view::{answered, widget::shortened},
    },
};

/// Returns the hints, each with the action whose keys it names when it can
/// give way
#[cfg(test)]
pub(super) fn hints(app: &App) -> Vec<(Option<Action>, String)> {
    hints_told(app, told(app))
}

/// Tells what a key remarked, why a field was left unapplied, or the latest
/// copy or cut, opening or saving, or change to the show told, whichever is
/// latest, and nothing while a menu is open, whose keys of picking and closing
/// come before any notice, since a notice outlasts every key when the bar is
/// short
pub(super) fn told(app: &App) -> Option<String> {
    asked(app)
        .or_else(|| notice(app))
        .filter(|_| !menu_open(app))
}

/// Returns whether a menu is open, which the help page covers
fn menu_open(app: &App) -> bool {
    app.menu && !app.help
}

/// Tells what the operator asked for or what went wrong, which comes before
/// the latest change to the show
fn asked(app: &App) -> Option<String> {
    clipped(app)
        .or_else(|| app.remark.map(str::to_owned))
        .or_else(|| app.unapplied.clone())
        .or_else(|| app.told.clone())
}

/// Returns the hints as [`hints`] does, with `told` for what the latest
/// change told
pub(super) fn hints_told(app: &App, told: Option<String>) -> Vec<(Option<Action>, String)> {
    let keymap = &app.config.keymap;
    let key =
        |action: Action, label: &str| keymap.hint(action, label).map(|hint| (Some(action), hint));
    // Key that is hinted only where it acts, which a field or a list that
    // has the keyboard may keep from it
    let here = |action: Action, label: &str| key(action, label).filter(|_| acts_here(app, action));
    let typed = (!app.keys.is_empty()).then(|| (None, shown(&app.keys)));
    let told = told.map(|told| (None, told));
    let back = going(app, Action::Back, app.back_to());
    let forward = going(app, Action::Forward, app.forward_to());
    // Two keys that a newcomer to the outputs needs, and none that destroys
    let teaching = (app.outputs_shown && app.editable()).then(|| {
        [
            here(Action::AddBelow, "add output"),
            here(Action::Edit, "rename"),
        ]
    });
    let save = key(Action::Save, Action::Save.name()).filter(|_| app.show.unsaved());
    // The page covers an open menu, and takes its keys.  Enter picks
    // whichever key is bound to pick, so that it stands in where none is.
    let in_menu = menu_open(app);
    let pick =
        key(Action::Pick, "pick").or_else(|| Some((Some(Action::Pick), "Enter: pick".to_owned())));
    let close = key(Action::CloseMenu, "close");
    // Escape would panic, so that the key that only cancels is told while it
    // can.  Cancelling keys typed or a search never gives way, while the
    // hint of leaving a field does, as it only teaches.
    let cancel = key(Action::Cancel, "cancel")
        .filter(|_| !app.keys.is_empty() || app.search.is_some())
        .map(|(_, hint)| (None, hint));
    // A pick or a close by key, and a Tab, leave the keyboard on a field or a
    // list, from which Escape would panic and no click is at hand
    let in_field = app.held && !app.help && !in_menu && app.search.is_none();
    let next_field = key(Action::NextField, "next field").filter(|_| in_field);
    // The keymap may bind no key to cancel, as CUA does, while Enter leaves a
    // text field, though it opens the menu of a drop-down list
    let leave = key(Action::Cancel, "leave")
        .or_else(|| {
            (app.text_held && enter_leaves(app))
                .then(|| (Some(Action::Cancel), "Enter: leave".to_owned()))
        })
        .filter(|_| in_field && app.keys.is_empty());
    // A cue just added bears a default name, which the key of renaming
    // selects whole, so that typing names it
    let rename =
        here(Action::ChangeName, "rename").filter(|_| renaming(app) && asked(app).is_none());
    // Tabs of the inspector show with the cues, and no field keeps the key
    let in_cues = !app.outputs_shown && !app.held && !app.help && !in_menu && app.search.is_none();
    let tab = key(Action::NextTab, "next tab").filter(|_| in_cues);
    // Enter moves standby to the selected cue, which no button of the toolbar
    // teaches by key, while the selection is elsewhere
    let standby = key(Action::Standby, "standby").filter(|_| {
        in_cues
            && app
                .selected_here()
                .is_some_and(|cue| app.current().standby != Some(cue))
    });
    // Tab skips the Choose button, so that Enter says how to reach the chooser
    // while the empty File field has the keyboard
    let choose = (in_field && app.file_held)
        .then(|| app.selected.and_then(|id| app.workspace.cue(id)))
        .flatten()
        .filter(|cue| cue.file.is_empty())
        .map(|_| (None, "Enter: choose a file".to_owned()));
    // A field or a list that has the keyboard keeps the key of help from
    // acting, and `?` types there, while an open menu and the page itself take it
    let help = key(Action::Help, Action::Help.name())
        .filter(|_| app.help || in_menu || acts_here(app, Action::Help));
    typed
        .into_iter()
        .chain(told)
        .chain(pick.filter(|_| in_menu))
        .chain(close.filter(|_| in_menu))
        .chain(cancel)
        .chain(choose)
        .chain(next_field)
        .chain(leave)
        .chain(teaching.into_iter().flatten().flatten())
        .chain(rename)
        .chain(back)
        .chain(forward)
        .chain(save)
        .chain(key(Action::Go, "GO"))
        .chain(key(Action::Panic, "Panic"))
        .chain(help)
        .chain(standby)
        .chain(tab)
        .collect()
}

/// Fewest columns of a place that stay, as the name of a cue list may be
/// long, so that the hint of going there still fits in a bar
const PLACE: usize = 16;

/// Names where a way back or forward goes: the outputs, the cue list, or
/// for a cue of the list on show, the cue
pub(super) fn place(app: &App, spot: Spot) -> String {
    let list = |index: usize| {
        app.workspace
            .lists()
            .get(index)
            .map_or_else(String::new, |list| shortened(&list.name, PLACE))
    };
    match spot {
        Spot::Outputs | Spot::Output(_) => "the outputs".to_owned(),
        Spot::List(index) => list(index),
        Spot::Cue(id) => match app.workspace.list_of(id) {
            Some(index) if index == app.list && !app.outputs_shown => app
                .workspace
                .cue(id)
                .map_or_else(String::new, |cue| format!("cue {}", format::number(cue))),
            Some(index) => list(index),
            None => String::new(),
        },
    }
}

/// Hints the key of an action that goes to `spot`, with the place
fn going(app: &App, action: Action, spot: Option<Spot>) -> Option<(Option<Action>, String)> {
    let spot = spot?;
    let label = format!("{} to {}", action.name(), place(app, spot));
    app.config
        .keymap
        .hint(action, &label)
        .map(|hint| (Some(action), hint))
}

/// Tells what the latest copy or cut took, with the key that pastes it and,
/// for a cut, the key that takes it back
pub(super) fn clipped(app: &App) -> Option<String> {
    let keymap = &app.config.keymap;
    let clipped = app.clipped.as_ref()?;
    let paste = keymap
        .hint(Action::PasteBelow, "paste")
        .filter(|_| app.editable() && acts_here(app, Action::PasteBelow));
    let undo = keymap.hint(Action::Undo, Action::Undo.name()).filter(|_| {
        clipped.cut && app.workspace.undoable().is_some() && acts_here(app, Action::Undo)
    });
    Some(
        iter::once(clipped.told.clone())
            .chain(paste)
            .chain(undo)
            .join(LINE),
    )
}

/// Tells the latest change to the show, with the key that takes it back,
/// or that makes it again after an undo, while that key can act.  While the
/// outputs show, only a change to the outputs is told, since a notice about
/// cues that are out of sight would take the room of the keys of the outputs,
/// which notices outrank when the bar is short.
pub(super) fn notice(app: &App) -> Option<String> {
    let workspace = &app.workspace;
    let (turn, change) = workspace
        .latest_change()
        .filter(|(_, change)| !app.outputs_shown || matches!(change, Change::Outputs { .. }))?;
    let told = format::change(workspace, change);
    let facts = format::consequences_of(workspace, change);
    let (told, action, next) = match turn {
        Turn::Done => (told, Action::Undo, workspace.undoable()),
        Turn::Undone => (format!("undid: {told}"), Action::Redo, workspace.redoable()),
        Turn::Redone => (format!("redid: {told}"), Action::Undo, workspace.undoable()),
    };
    let told = format::leading(facts, &told);
    // A text field keeps every key, so that no key takes the change back
    // there, while a closed list lets the keys of undo and redo act
    next.map(|_| {
        if acts_here(app, action) {
            answered(told, &app.config.keymap, action)
        } else {
            told
        }
    })
}

/// Returns whether Enter leaves a text field, which it does unless the keymap
/// binds it to an action that acts in fields, such as GO, which then takes it
fn enter_leaves(app: &App) -> bool {
    Press::from_key(&Key::Named(Named::Enter), Modifiers::empty())
        .is_none_or(|enter| !app.config.keymap.acts_in_fields(enter))
}

/// Returns whether a key of `action` acts where the keyboard is.  A text
/// field keeps every key but those of [`Action::acts_in_fields`], and a
/// closed drop-down list those but for [`Action::acts_in_lists`].
fn acts_here(app: &App, action: Action) -> bool {
    match (app.text_held, app.held) {
        (true, _) => action.acts_in_fields(),
        (false, true) => action.acts_in_lists(),
        (false, false) => true,
    }
}

/// Returns whether the latest change added a cue, whose default name the key
/// of renaming selects whole, while the cue list is in view
fn renaming(app: &App) -> bool {
    app.editable()
        && !app.outputs_shown
        && matches!(
            app.workspace.latest_change(),
            Some((Turn::Done, Change::Added { .. }))
        )
        && app.workspace.undoable().is_some()
}
