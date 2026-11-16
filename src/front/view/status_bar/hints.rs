// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The hints of the status bar: the keys typed so far, what the latest
//! change told, and the keys that can act now

use std::iter;

use itertools::Itertools;

use super::{LINE, format};
use crate::{
    back::Turn,
    front::{
        App,
        keymap::{Action, written},
        view::answered,
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
/// latest
pub(super) fn told(app: &App) -> Option<String> {
    clipped(app)
        .or_else(|| app.remark.map(str::to_owned))
        .or_else(|| app.unapplied.clone())
        .or_else(|| app.told.clone())
        .or_else(|| notice(app))
}

/// Returns the hints as [`hints`] does, with `told` for what the latest
/// change told
pub(super) fn hints_told(app: &App, told: Option<String>) -> Vec<(Option<Action>, String)> {
    let keymap = &app.config.keymap;
    let key =
        |action: Action, label: &str| keymap.hint(action, label).map(|hint| (Some(action), hint));
    let typed = (!app.keys.is_empty()).then(|| (None, written(&app.keys)));
    let told = told.map(|told| (None, told));
    let back = key(Action::Back, Action::Back.name()).filter(|_| app.can_go_back());
    let forward = key(Action::Forward, Action::Forward.name()).filter(|_| app.can_go_forward());
    // Two keys that a newcomer to the outputs needs, and none that destroys
    let teaching = (app.outputs_shown && app.editable()).then(|| {
        [
            key(Action::AddBelow, "add output"),
            key(Action::Edit, "rename"),
        ]
    });
    let save = key(Action::Save, Action::Save.name()).filter(|_| app.show.unsaved());
    // The page covers an open menu, and takes its keys.  Enter picks
    // whichever key is bound to pick, so that it stands in where none is.
    let in_menu = app.menu && !app.help;
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
    let leave = key(Action::Cancel, "leave").filter(|_| in_field && app.keys.is_empty());
    typed
        .into_iter()
        .chain(told)
        .chain(pick.filter(|_| in_menu))
        .chain(close.filter(|_| in_menu))
        .chain(cancel)
        .chain(next_field)
        .chain(leave)
        .chain(teaching.into_iter().flatten().flatten())
        .chain(back)
        .chain(forward)
        .chain(save)
        .chain(key(Action::Go, "GO"))
        .chain(key(Action::Panic, "Panic"))
        .chain(key(Action::Help, Action::Help.name()))
        .collect()
}

/// Tells what the latest copy or cut took, with the key that pastes it and,
/// for a cut, the key that takes it back
pub(super) fn clipped(app: &App) -> Option<String> {
    let keymap = &app.config.keymap;
    let clipped = app.clipped.as_ref()?;
    let paste = keymap
        .hint(Action::PasteBelow, "paste")
        .filter(|_| app.editable());
    let undo = keymap
        .hint(Action::Undo, Action::Undo.name())
        .filter(|_| clipped.cut && app.workspace.undoable().is_some());
    Some(
        iter::once(clipped.told.clone())
            .chain(paste)
            .chain(undo)
            .join(LINE),
    )
}

/// Tells the latest change to the show, with the key that takes it back,
/// or that makes it again after an undo, while that key can act
pub(super) fn notice(app: &App) -> Option<String> {
    let workspace = &app.workspace;
    let (turn, change) = workspace.latest_change()?;
    let told = format::change(workspace, change);
    let (told, action, next) = match turn {
        Turn::Done => (capitalized(&told), Action::Undo, workspace.undoable()),
        Turn::Undone => (format!("Undid: {told}"), Action::Redo, workspace.redoable()),
        Turn::Redone => (format!("Redid: {told}"), Action::Undo, workspace.undoable()),
    };
    next.map(|_| answered(told, &app.config.keymap, action))
}

fn capitalized(text: &str) -> String {
    text.chars()
        .take(1)
        .flat_map(char::to_uppercase)
        .chain(text.chars().skip(1))
        .collect()
}
