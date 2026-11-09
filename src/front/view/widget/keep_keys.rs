// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Region of fields that keeps the keys typed into them
//!
//! iced's text fields let some keys through, such as the arrows, and a
//! read-only field lets every key through, as does a drop-down list apart
//! from Enter, which opens its menu.  Those keys would reach the key
//! bindings and act on the cue list while the operator types.  While one of
//! the fields has the keyboard focus, this region keeps the keys that the
//! field let through, apart from the keys that act in fields, which are
//! those of GO and Panic, those that move between fields, and those that
//! open and save shows.  Enter leaves a text field.  A field that takes a
//! key of GO or Panic without typing it, as iced's text fields take Escape
//! to give up the keyboard and Control with `c`, `x`, `v`, or `a` for the
//! clipboard and the selection, leaves the key to act all the same.  The
//! region decides as each key arrives, so keys act in the order the
//! operator typed them.

#[cfg(test)]
mod tests;

use iced::{
    Element, Event, Rectangle, Renderer,
    advanced::{
        Shell,
        layout::Layout,
        widget::{Id, Operation, Tree, operation::Focusable},
    },
    keyboard::{Key, key::Named},
};

use super::{
    KeyPress, key_pressed,
    wrap::{Intercept, wrap},
};
use crate::front::{
    Focus, Message,
    keymap::{Keymap, Press},
};

pub fn keep_keys<'a>(
    content: impl Into<Element<'a, Message>>,
    keymap: &'a Keymap,
) -> Element<'a, Message> {
    wrap(content, Keep(keymap))
}

struct Keep<'a>(&'a Keymap);

impl Intercept for Keep<'_> {
    fn after(
        &self,
        content: &mut Element<'_, Message>,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(pressed) = key_pressed(event) else {
            return;
        };
        let KeyPress {
            key, press, repeat, ..
        } = pressed;
        if shell.is_event_captured() {
            if let Some(press) = left_to_act(self.0, &pressed) {
                shell.publish(Message::Typed { press, repeat });
            }
            return;
        }
        let mut focus = HasFocus(false);
        content
            .as_widget_mut()
            .operate(tree, layout, renderer, &mut focus);
        if !focus.0 {
            return;
        }
        let acts = press.is_some_and(|press| self.0.acts_in_fields(press));
        match key {
            _ if acts => {}
            Key::Named(Named::Enter) => {
                shell.publish(Message::Focus(Focus::Cues));
                shell.capture_event();
            }
            _ => shell.capture_event(),
        }
    }
}

/// Returns the press of a key that a widget captured, when it is a key of
/// GO or Panic that the widget did not type, which therefore acts as well
fn left_to_act(keymap: &Keymap, key: &KeyPress) -> Option<Press> {
    key.press
        .filter(|&press| !types_text(key) && keymap.acts_at_once(press))
}

/// Returns whether iced's text input inserts the key's text.  It inserts
/// the first character of the text unless that is a control character,
/// once the keys of the clipboard and the selection have taken their turn,
/// which type nothing, whatever text a system reports for them.
fn types_text(key: &KeyPress) -> bool {
    !takes_clipboard_key(key)
        && key
            .text
            .and_then(|text| text.chars().next())
            .is_some_and(|first| !first.is_control())
}

/// Returns whether the key is one that iced's text input takes for the
/// clipboard or the selection: the command modifier with `c`, `x`, `v`
/// unless Alt is held, or `a`, as the letter reads on a Latin layout
fn takes_clipboard_key(key: &KeyPress) -> bool {
    key.modifiers.command()
        && match key.key.to_latin(key.physical) {
            Some('c' | 'x' | 'a') => true,
            Some('v') => !key.modifiers.alt(),
            _ => false,
        }
}

/// Operation that finds out whether a text field or a drop-down list has
/// the keyboard focus, read-only fields included
struct HasFocus(bool);

impl Operation for HasFocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.0 |= state.is_focused();
    }
}
