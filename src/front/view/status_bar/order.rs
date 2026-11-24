// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Order in which the hints of the status bar give way, which differs where a
//! field has the keyboard, the outputs show, or a list has ended

use super::hints;
use crate::front::{App, keymap::Action};

/// Hints in turn of giving way, the first to go first.  The key of renaming
/// outlasts GO and Panic, whose buttons the transport shows, saving, whose
/// mark the status shows, and the help page, which the first steps and a key
/// of every table teach, since it is told only right after a cue is added and
/// nowhere else.  The keys of the next cue list and of the next tab go first
/// of all, as they only teach, and that of hint mode follows them, so that a
/// narrow bar of the cua keymap, where its key is long, still shows it.
pub(super) const YIELDING: [Action; 15] = [
    Action::NextList,
    Action::NextTab,
    Action::Hint,
    Action::Standby,
    Action::Forward,
    Action::Back,
    Action::Go,
    Action::Panic,
    Action::Save,
    Action::Help,
    Action::ChangeName,
    Action::NextField,
    Action::Cancel,
    Action::CloseMenu,
    Action::Pick,
];

/// Hints in turn of giving way while standby lies past the end of its list
/// and a later list exists, where GO fires nothing, so that the key of the
/// next list, which is the way on, outlasts the keys that only teach, the
/// hints of GO and Panic, whose buttons the transport shows, and those of the
/// help page and of renaming a cue just added
const YIELDING_AT_END: [Action; 15] = [
    Action::Hint,
    Action::NextTab,
    Action::Standby,
    Action::Forward,
    Action::Back,
    Action::Go,
    Action::Panic,
    Action::Save,
    Action::Help,
    Action::ChangeName,
    Action::NextList,
    Action::NextField,
    Action::Cancel,
    Action::CloseMenu,
    Action::Pick,
];

/// Hints in turn of giving way while a text field, a checkbox, or a closed
/// drop-down list has the keyboard, the first to go first.  The key of the
/// next field only teaches, and a focus border shows the field, so that it
/// goes first.  What Enter does outlasts the other keys that only teach, but
/// gives way before GO and Panic, which act there, and those give way before
/// the key that leaves the field, which is the way back to the cue list and
/// panic.
pub(super) const YIELDING_IN_FIELD: [Action; 16] = [
    Action::NextField,
    Action::NextTab,
    Action::Standby,
    Action::NextList,
    Action::Forward,
    Action::Back,
    Action::ChangeName,
    Action::AddBelow,
    Action::Edit,
    Action::Help,
    Action::Pick,
    Action::Go,
    Action::Panic,
    Action::Save,
    Action::Cancel,
    Action::CloseMenu,
];

/// Hints in turn of giving way while the outputs show, the first to go
/// first.  The keys that add and rename an output go right after Forward,
/// as they only teach, and Back stays until after GO, Panic, and Save,
/// since it is the way out of the outputs, while the transport has buttons
/// for GO and Panic.
const YIELDING_IN_OUTPUTS: [Action; 14] = [
    Action::Hint,
    Action::ChangeName,
    Action::Forward,
    Action::AddBelow,
    Action::Edit,
    Action::Go,
    Action::Panic,
    Action::Save,
    Action::Back,
    Action::Help,
    Action::NextField,
    Action::Cancel,
    Action::CloseMenu,
    Action::Pick,
];

pub(super) fn yielding(app: &App) -> &'static [Action] {
    if hints::in_field(app) {
        &YIELDING_IN_FIELD
    } else if app.outputs_shown {
        &YIELDING_IN_OUTPUTS
    } else if hints::list_ended(app) {
        &YIELDING_AT_END
    } else {
        &YIELDING
    }
}

/// Returns the hints that outlast the words of the latest change, which are
/// the keys that leave a field, since a field takes every other key, and
/// Escape would panic there.
pub(super) fn outlasting(app: &App) -> &'static [Action] {
    if hints::in_field(app) {
        &[Action::Cancel, Action::CloseMenu]
    } else {
        &[]
    }
}
