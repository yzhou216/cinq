// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Checkbox that Tab reaches and Enter flips
//!
//! iced's checkbox cannot take the keyboard focus, so that no key reaches it.
//! This one draws iced's checkbox inside a thicker border while it has the
//! keyboard, as a text field is drawn, takes part in the operations that move
//! the keyboard from field to field, and flips on Enter, which no other
//! binding may take there, so the event is kept from the key bindings.  Space
//! stays GO and Escape stays Panic, as everywhere, since neither flips it.
//! A checkbox that cannot change takes no keyboard.

mod widget;
use iced::{Element, advanced::Shell, keyboard::Key};

use super::focus;
use crate::front::{Focus, Message, keymap::Keymap};

pub struct Check<'a> {
    /// iced's checkbox, which draws this one
    look: Element<'a, Message>,
    /// Message that Enter sends, which flips the box, unless it cannot change
    flip: Option<Message>,
    /// Whether Enter gives the keyboard back to the cue list after it flips
    /// the box
    leaves: bool,
    keymap: &'a Keymap,
}

/// Returns a checkbox that sends what `on_toggle` makes of the state that a
/// click or Enter flips it to, or one that cannot change without it
pub fn check<'a>(
    label: &'a str,
    checked: bool,
    on_toggle: Option<impl Fn(bool) -> Message + Clone + 'a>,
    keymap: &'a Keymap,
) -> Element<'a, Message> {
    checkbox(label, checked, on_toggle, false, keymap)
}

/// Returns a checkbox as [`check`] does, one that gives the keyboard back to
/// the cue list after Enter flips it, for a setting that the operator makes
/// before moving on to the cues
pub fn check_and_leave<'a>(
    label: &'a str,
    checked: bool,
    on_toggle: Option<impl Fn(bool) -> Message + Clone + 'a>,
    keymap: &'a Keymap,
) -> Element<'a, Message> {
    checkbox(label, checked, on_toggle, true, keymap)
}

fn checkbox<'a>(
    label: &'a str,
    checked: bool,
    on_toggle: Option<impl Fn(bool) -> Message + Clone + 'a>,
    leaves: bool,
    keymap: &'a Keymap,
) -> Element<'a, Message> {
    Element::new(Check {
        flip: on_toggle.clone().map(|on_toggle| on_toggle(!checked)),
        look: iced::widget::checkbox(checked)
            .label(label)
            .on_toggle_maybe(on_toggle)
            .into(),
        leaves,
        keymap,
    })
}

impl Check<'_> {
    /// Handles a key that arrives while the checkbox may have the keyboard,
    /// and returns whether it took the key.  Enter flips the box once however
    /// long it is held, and the key bindings never see it, where it would put a
    /// cue on standby.  A key that acts at once, such as one that a user binds
    /// to GO, is never taken.
    fn took(
        &self,
        focused: bool,
        key: &Key,
        at_once: bool,
        repeat: bool,
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        let taken = focus::took(focused, key, at_once, shell);
        if let (true, false, Some(flip)) = (taken, repeat, &self.flip) {
            shell.publish(flip.clone());
            if self.leaves {
                shell.publish(Message::Focus(Focus::Cues));
            }
        }
        taken
    }
}

#[cfg(test)]
mod tests;
