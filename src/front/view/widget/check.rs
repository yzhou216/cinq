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

use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme,
    advanced::{
        Clipboard, Shell,
        layout::{self, Layout},
        mouse, overlay, renderer,
        widget::{
            Operation, Widget,
            tree::{self, Tree},
        },
    },
    keyboard::Key,
};

use super::{
    focus::{self, State},
    key_pressed,
};
use crate::front::{
    Focus, Message,
    keymap::{KeyPress, Keymap},
};

/// Room that the label keeps after its last letter, within the border
const ROOM: f32 = 4.0;

/// Returns the layout of iced's checkbox, which lies in the room of this one
fn look(layout: Layout<'_>) -> Layout<'_> {
    layout
        .children()
        .next()
        .expect("the layout of a checkbox holds that of iced's")
}

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

impl Widget<Message, Theme, Renderer> for Check<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State { focused: false })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.look)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.look));
    }

    fn size(&self) -> Size<Length> {
        self.look.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let look = self.look.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &limits.shrink(Size::new(ROOM, 0.0)),
        );
        layout::Node::with_children(look.size().expand(Size::new(ROOM, 0.0)), vec![look])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.look.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            look(layout),
            cursor,
            viewport,
        );
        if tree.state.downcast_ref::<State>().focused {
            focus::mark(renderer, theme, layout.bounds());
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        focus::click_takes(state, event, shell);
        let taken = key_pressed(event).is_some_and(|KeyPress { key, press, repeat }| {
            let at_once = press.is_some_and(|press| self.keymap.acts_at_once(press));
            self.took(state.focused, key, at_once, repeat, shell)
        });
        if !taken {
            self.look.as_widget_mut().update(
                &mut tree.children[0],
                event,
                look(layout),
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    /// Lets the keys that move between fields move to the checkbox, unless
    /// it cannot change
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.flip.is_some() {
            operation.focusable(None, layout.bounds(), tree.state.downcast_mut::<State>());
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.look.as_widget().mouse_interaction(
            &tree.children[0],
            look(layout),
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        _tree: &'b mut Tree,
        _layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        _translation: iced::Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        None
    }
}

#[cfg(test)]
mod tests;
