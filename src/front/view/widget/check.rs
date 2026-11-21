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
    Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme,
    advanced::{
        Clipboard, Renderer as _, Shell,
        layout::{self, Layout},
        mouse, overlay, renderer,
        widget::{
            Operation, Widget,
            operation::Focusable,
            tree::{self, Tree},
        },
    },
    keyboard::{Key, key::Named},
    widget::text_input,
};

use super::{key_pressed, pressed};
use crate::front::{
    Message,
    keymap::{KeyPress, Keymap},
    view::theme,
};

/// Radius of the border that marks the checkbox as it has the keyboard
const RADIUS: f32 = 2.0;

pub struct Check<'a> {
    /// iced's checkbox, which draws this one
    look: Element<'a, Message>,
    /// Message that Enter sends, which flips the box, unless it cannot change
    flip: Option<Message>,
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
    Element::new(Check {
        flip: on_toggle.clone().map(|on_toggle| on_toggle(!checked)),
        look: iced::widget::checkbox(checked)
            .label(label)
            .on_toggle_maybe(on_toggle)
            .into(),
        keymap,
    })
}

struct State {
    /// Whether the checkbox has the keyboard, which any click takes away, as
    /// it does from a text field
    focused: bool,
}

impl Focusable for State {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
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
        let taken = focused && !at_once && *key == Key::Named(Named::Enter);
        if let (true, false, Some(flip)) = (taken, repeat, &self.flip) {
            shell.publish(flip.clone());
        }
        if taken {
            shell.capture_event();
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
        self.look
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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
            layout,
            cursor,
            viewport,
        );
        if tree.state.downcast_ref::<State>().focused {
            // Color of the border of a focused text field
            let focused = text_input::Status::Focused { is_hovered: false };
            let border = Border {
                color: text_input::default(theme, focused).border.color,
                width: theme::FOCUS_WIDTH,
                radius: RADIUS.into(),
            };
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds().expand(theme::FOCUS_WIDTH),
                    border,
                    ..renderer::Quad::default()
                },
                Color::TRANSPARENT,
            );
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
        if pressed(event) && state.focused {
            state.focused = false;
            shell.request_redraw();
        }
        let taken = key_pressed(event).is_some_and(|KeyPress { key, press, repeat }| {
            let at_once = press.is_some_and(|press| self.keymap.acts_at_once(press));
            self.took(state.focused, key, at_once, repeat, shell)
        });
        if !taken {
            self.look.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout,
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
            layout,
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
