// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The [`Widget`] implementation of the checkbox, which draws iced's inside a
//! border and takes the keys that flip it

use iced::{
    Event, Length, Rectangle, Renderer, Size, Theme,
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

use super::{
    super::{
        focus::{self, State},
        key_pressed,
    },
    Check,
};
use crate::front::{Message, keymap::KeyPress, view};

/// Room that the label keeps after its last letter, within the border
const ROOM: f32 = 4.0;

/// Returns the layout of iced's checkbox, which lies in the room of this one
fn look(layout: Layout<'_>) -> Layout<'_> {
    layout
        .children()
        .next()
        .expect("the layout of a checkbox holds that of iced's")
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
            operation.focusable(
                Some(&Id::new(view::CHECK)),
                layout.bounds(),
                tree.state.downcast_mut::<State>(),
            );
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
