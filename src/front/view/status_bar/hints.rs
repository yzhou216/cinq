// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The hints of the status bar: the keys typed so far, what the latest
//! change told, and the keys that can act now

pub(super) mod notices;

use iced::keyboard::{Key, Modifiers, key::Named};

use super::format;
use crate::{
    back::{Change, Turn},
    front::{
        App,
        keymap::{Action, Press, shown},
        update::Spot,
        view::widget::shortened,
    },
};

/// Returns the hints, each with the action whose keys it names when it can
/// give way
#[cfg(test)]
pub(super) fn hints(app: &App) -> Vec<(Option<Action>, String)> {
    hints_told(app, notices::told(app))
}

/// Returns whether a menu is open, which the help page covers
pub(super) fn menu_open(app: &App) -> bool {
    app.menu && !app.help
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
    // Only the keys that act in hint mode, since the others type a letter of a
    // label or leave it
    if let Some(hints) = &app.hints {
        let letters = Some(hints.typed()).filter(|typed| !typed.is_empty());
        // Backspace takes a letter back while any is typed, and leaves
        // only where none is
        let backspace = if letters.is_some() { "back" } else { "leave" };
        let leave = key(Action::Cancel, "cancel")
            .or_else(|| Some((None, format!("Backspace: {backspace}"))));
        return [
            Some((None, letters.unwrap_or("Type a label").to_owned())),
            leave.map(|(_, hint)| (None, hint)),
            key(Action::Go, "GO"),
            key(Action::Panic, "Panic"),
        ]
        .into_iter()
        .flatten()
        .collect();
    }
    let typed = (!app.keys.is_empty()).then(|| (None, shown(&app.keys)));
    let told = told.map(|told| (None, told));
    let back = going(app, Action::Back, app.back_to());
    let forward = going(app, Action::Forward, app.forward_to());
    // Two keys that a newcomer to the outputs needs, and none that destroys.
    // A keymap that binds no key to edit may bind one to change the name,
    // which enters the same field, and the hint gives way before add output
    // as that of renaming a cue does, so that the key to add still shows
    let teaching = (app.outputs_shown && app.editable()).then(|| {
        [
            here(Action::AddBelow, "add output"),
            here(Action::Edit, "rename").or_else(|| here(Action::ChangeName, "rename")),
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
    // hint of leaving a field does, as it only teaches.  Where the keymap binds
    // no key to cancel, the search line is left by Backspace on an empty line
    // and by Enter once text is typed, which accepts it.
    let cancel = key(Action::Cancel, "cancel")
        .map(|(_, hint)| hint)
        .or_else(|| {
            app.search
                .as_ref()
                .map(|search| match search.text.is_empty() {
                    true => "Backspace: leave".to_owned(),
                    false => "Enter: find".to_owned(),
                })
        })
        .filter(|_| !app.keys.is_empty() || app.search.is_some())
        .map(|hint| (None, hint));
    // A pick or a close by key, and a Tab, leave the keyboard on a field or a
    // list, from which Escape would panic and no click is at hand
    let in_field = in_field(app);
    let next_field = key(Action::NextField, "next field").filter(|_| in_field);
    // Enter flips a checkbox, presses a button, and opens a closed list, where
    // a newcomer may take Space, which is GO, for the key that does.  A text
    // field keeps Enter for leaving, and a key bound to act at once keeps it
    // from any of them.
    let enter = (in_field && !app.text_held && enter_leaves(app)).then(|| {
        let does = match (app.check_held, app.button_held) {
            (true, _) => "flip",
            (false, true) => "press",
            (false, false) => "open",
        };
        (Some(Action::Pick), format!("Enter: {does}"))
    });
    // Tab skips the Choose button, so that Enter says how to reach the chooser
    // while the empty File field has the keyboard
    let choose = choosing(app).then(|| (None, "Enter: choose a file".to_owned()));
    // A keymap of the file's own may bind no key to cancel, while Enter
    // leaves a text field, though it opens the menu of a drop-down list and
    // chooses a file in the empty File field
    let leave = key(Action::Cancel, "leave")
        .or_else(|| {
            (app.text_held && enter_leaves(app) && choose.is_none())
                .then(|| (Some(Action::Cancel), "Enter: leave".to_owned()))
        })
        .filter(|_| in_field && app.keys.is_empty());
    // A cue just added bears a default name, which the key of renaming
    // selects whole, so that typing names it
    let rename = here(Action::ChangeName, "rename")
        .filter(|_| renaming(app) && notices::asked(app).is_none());
    // Tabs of the inspector show with the cues, and no field keeps the key
    let in_cues = !app.outputs_shown && !app.held && !app.help && !in_menu && app.search.is_none();
    let tab = key(Action::NextTab, "next tab").filter(|_| in_cues);
    // The way to the next list, and at last to the outputs, which only the
    // keys teach
    let next_list = key(Action::NextList, "next list").filter(|_| in_cues);
    // Enter moves standby to the selected cue, which no button of the toolbar
    // teaches by key, while the selection is elsewhere
    let standby = key(Action::Standby, "standby").filter(|_| {
        in_cues
            && app
                .selected_here()
                .is_some_and(|cue| app.current().standby != Some(cue))
    });
    // Space types a space while a text field has the keyboard, so that GO
    // is hinted by a key that is no character, or by none
    let go = if app.text_held {
        keymap
            .hint_over_text(Action::Go, "GO")
            .map(|hint| (Some(Action::Go), hint))
    } else {
        key(Action::Go, "GO")
    };
    // A field or a list that has the keyboard keeps the key of help from
    // acting, and `?` types there, while an open menu and the page itself take it
    let help = key(Action::Help, Action::Help.name()).filter(|_| {
        ((app.help || in_menu) && !app.stage.has_keyboard()) || acts_here(app, Action::Help)
    });
    // The first step with no show open is to open one, which no button of the
    // toolbar teaches by key
    let open = key(Action::Open, "open")
        .filter(|_| in_cues && !app.show.has_file() && app.workspace.cues().next().is_none());
    // Key of fullscreen, taught where the operator needs it: to open the window
    // of the audience, closed while a cue shows on it, and to leave a
    // fullscreen stage window that covers the main window on a single display,
    // from which almost every other key is left to the main window
    let stage = here(
        Action::Fullscreen,
        if app.stage.has_keyboard() {
            "fullscreen"
        } else {
            "open stage"
        },
    )
    .filter(|_| {
        (app.stage.has_keyboard() || app.stage.missing(&app.workspace)) && !in_menu && !app.help
    });
    // Key of hint mode, taught where it acts
    let hint_mode = here(Action::Hint, "hint").filter(|_| !in_menu);
    typed
        .into_iter()
        .chain(told)
        .chain(pick.filter(|_| in_menu))
        .chain(close.filter(|_| in_menu))
        .chain(cancel)
        .chain(choose)
        .chain(enter)
        .chain(next_field)
        .chain(leave)
        .chain(teaching.into_iter().flatten().flatten())
        .chain(rename)
        .chain(back)
        .chain(forward)
        .chain(save)
        .chain(open)
        .chain(go)
        .chain(key(Action::Panic, "Panic"))
        .chain(stage)
        .chain(help)
        .chain(standby)
        .chain(tab)
        .chain(next_list)
        .chain(hint_mode)
        .collect()
}

/// Returns whether a text field, a checkbox, or a closed drop-down list has
/// the keyboard, from which a pick or a close by key, and a Tab, leave it, which
/// the stage window never does, as it holds no field
pub(super) fn in_field(app: &App) -> bool {
    app.held && !app.help && !menu_open(app) && app.search.is_none() && !app.stage.has_keyboard()
}

/// Returns whether standby lies past the end of the cue list on show while a
/// later list exists, where GO fires nothing
pub(super) fn list_ended(app: &App) -> bool {
    app.current().standby.is_none() && app.list + 1 < app.workspace.lists().len()
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
            Some(index) if index == app.list && !app.outputs_shown => {
                app.workspace.cue(id).map_or_else(String::new, |cue| {
                    format!("cue {}", shortened(&format::number(cue), PLACE))
                })
            }
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

/// Returns whether the empty File field has the keyboard, where Enter opens
/// the chooser of a file and does not leave
pub(super) fn choosing(app: &App) -> bool {
    in_field(app)
        && app.file_held
        && app
            .selected
            .and_then(|id| app.workspace.cue(id))
            .is_some_and(|cue| cue.file.is_empty())
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
pub(super) fn acts_here(app: &App, action: Action) -> bool {
    // The stage window holds no field, and leaves the keys of
    // [`Action::stays_in_main_window`] to the main window, as the update does
    if app.stage.has_keyboard() {
        return !action.stays_in_main_window();
    }
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
