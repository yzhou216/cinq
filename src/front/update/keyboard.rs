// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Finding out whether a text field or a drop-down list has the keyboard
//!
//! The widgets keep that to themselves, and the status bar hints how to leave
//! a field, so that one operation asks them after anything the operator does.
//! A second asks whether the File field of a cue is the one, since its hint
//! differs, a third whether an address of the light patch is, since the patch
//! shows help on addresses only then, and a fourth whether a text field is,
//! since it keeps every key but those that act in fields, while a closed
//! drop-down list lets the keys of undo and redo act as well, which the status
//! bar hints accordingly.

use iced::{
    Rectangle, Task,
    advanced::widget::{
        Id, Operation, operate,
        operation::{Focusable, Outcome, TextInput},
    },
};

use crate::front::{Message, view};

/// Asks the widgets whether one of them has the keyboard, and whether it is
/// the File field, and whether it is an address of the light patch, and
/// whether it is a text field, and tells [`Message::Held`],
/// [`Message::FileHeld`], [`Message::AddressHeld`], and [`Message::TextHeld`]
/// the answers
fn held() -> Task<Message> {
    Task::batch([
        operate(Holding::anywhere()).map(Message::Held),
        operate(Holding::in_field(view::FILE)).map(Message::FileHeld),
        operate(Holding::in_field(view::CHECK)).map(Message::CheckHeld),
        operate(Holding::in_field(view::BUTTON)).map(Message::ButtonHeld),
        operate(Holding::in_field(view::ADDRESS)).map(Message::AddressHeld),
        operate(Holding::in_text()).map(Message::TextHeld),
    ])
}

/// Runs `task`, which may move the keyboard, and then asks [`held`].  A move
/// by Tab takes two operations, so that a question asked beside it would be
/// answered between them, for the field that had the keyboard before.
pub(super) fn held_after(task: Task<Message>) -> Task<Message> {
    task.chain(held())
}

/// Finds out whether any widget in any window has the keyboard focus, or
/// the one that goes by an identity, or a text field
struct Holding {
    field: Option<Id>,
    /// Whether only a text field counts
    text_only: bool,
    /// Whether the widget visited last is a text field, which tells itself
    /// just before it offers its focus
    text: bool,
    found: bool,
}

impl Holding {
    fn anywhere() -> Self {
        Self {
            field: None,
            text_only: false,
            text: false,
            found: false,
        }
    }

    fn in_field(field: &'static str) -> Self {
        Self {
            field: Some(Id::new(field)),
            ..Self::anywhere()
        }
    }

    fn in_text() -> Self {
        Self {
            text_only: true,
            ..Self::anywhere()
        }
    }
}

impl Operation<bool> for Holding {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<bool>)) {
        operate(self);
    }

    fn text_input(&mut self, _id: Option<&Id>, _bounds: Rectangle, _state: &mut dyn TextInput) {
        self.text = true;
    }

    fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.found |= state.is_focused()
            && self.field.as_ref().is_none_or(|field| id == Some(field))
            && (self.text || !self.text_only);
        self.text = false;
    }

    fn finish(&self) -> Outcome<bool> {
        Outcome::Some(self.found)
    }
}

#[cfg(test)]
mod tests;
