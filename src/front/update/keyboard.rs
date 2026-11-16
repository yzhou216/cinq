// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Finding out whether a text field or a drop-down list has the keyboard
//!
//! The widgets keep that to themselves, and the status bar hints how to leave
//! a field, so that one operation asks them after anything the operator does.

use iced::{
    Rectangle, Task,
    advanced::widget::{
        Id, Operation, operate,
        operation::{Focusable, Outcome},
    },
};

use crate::front::Message;

/// Asks the widgets whether one of them has the keyboard, and tells
/// [`Message::Held`] the answer
pub(super) fn held() -> Task<Message> {
    operate(Holding(false)).map(Message::Held)
}

/// Finds out whether any widget in any window has the keyboard focus
struct Holding(bool);

impl Operation<bool> for Holding {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<bool>)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.0 |= state.is_focused();
    }

    fn finish(&self) -> Outcome<bool> {
        Outcome::Some(self.0)
    }
}

#[cfg(test)]
mod tests;
