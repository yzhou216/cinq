// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Button that Tab reaches and Enter presses
//!
//! It draws the button inside the border of a focused text field while it has
//! the keyboard, takes part in the operations that move the keyboard from
//! field to field, and sends its message on Enter, which the key bindings
//! never see, as for a checkbox.  Space stays GO and Escape stays Panic.

use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme,
    advanced::{
        Clipboard, Shell,
        layout::{self, Layout},
        mouse, overlay, renderer,
        widget::{
            Id, Operation, Widget,
            tree::{self, Tree},
        },
    },
};

use super::super::{
    focus::{self, State},
    key_pressed,
};
use crate::front::{
    Focus, Message,
    keymap::{KeyPress, Keymap},
    view,
};

pub struct Focused<'a> {
    /// The button, which draws this one and sends its message on a press
    look: Element<'a, Message>,
    press: Message,
    /// Whether Enter gives the keyboard back to the cue list after it presses
    /// the button
    leaves: bool,
    keymap: &'a Keymap,
}

impl<'a> Focused<'a> {
    pub fn new(
        look: Element<'a, Message>,
        press: Message,
        leaves: bool,
        keymap: &'a Keymap,
    ) -> Self {
        Self {
            look,
            press,
            leaves,
            keymap,
        }
    }

    /// Handles a key that arrives while the button may have the keyboard, and
    /// returns whether it took the key.  Enter presses the button once
    /// however long it is held.
    fn took(
        &self,
        focused: bool,
        key: &iced::keyboard::Key,
        at_once: bool,
        repeat: bool,
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        let taken = focus::took(focused, key, at_once, shell);
        if taken && !repeat {
            shell.publish(self.press.clone());
            if self.leaves {
                shell.publish(Message::Focus(Focus::Cues));
            }
        }
        taken
    }
}

impl Widget<Message, Theme, Renderer> for Focused<'_> {
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
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.focusable(
            Some(&Id::new(view::BUTTON)),
            layout.bounds(),
            tree.state.downcast_mut::<State>(),
        );
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
