// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What a checkbox and a button share when Tab reaches them and Enter acts
//!
//! The state that tells whether the keyboard is theirs, the border that marks
//! it as a text field is marked, and the rule for Enter, which they take
//! unless a key that acts at once is bound to it, and which the key bindings
//! never see, where it would put a cue on standby.

use iced::{
    Border, Color, Event, Rectangle, Renderer, Theme,
    advanced::{Renderer as _, Shell, renderer, widget::operation::Focusable},
    keyboard::{Key, key::Named},
    widget::text_input,
};

use super::pressed;
use crate::front::{Message, view::theme};

/// Radius of the border that marks a widget as it has the keyboard
const RADIUS: f32 = 2.0;

pub struct State {
    /// Whether the widget has the keyboard, which any click takes away, as it
    /// does from a text field
    pub focused: bool,
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

/// Draws the border of a focused text field around `bounds`
pub fn mark(renderer: &mut Renderer, theme: &Theme, bounds: Rectangle) {
    let focused = text_input::Status::Focused { is_hovered: false };
    let border = Border {
        color: text_input::default(theme, focused).border.color,
        width: theme::FOCUS_WIDTH,
        radius: RADIUS.into(),
    };
    renderer.fill_quad(
        renderer::Quad {
            bounds: bounds.expand(theme::FOCUS_WIDTH),
            border,
            ..renderer::Quad::default()
        },
        Color::TRANSPARENT,
    );
}

/// Returns whether a widget that may have the keyboard takes `key`, which it
/// does for Enter alone, unless a key that acts at once is bound to it, and
/// keeps the event from the widgets after it when it does
pub fn took(focused: bool, key: &Key, at_once: bool, shell: &mut Shell<'_, Message>) -> bool {
    let taken = focused && !at_once && *key == Key::Named(Named::Enter);
    if taken {
        shell.capture_event();
    }
    taken
}

/// Takes the keyboard from the widget when any button of the mouse or a touch
/// goes down, as a click does from a text field
pub fn click_takes(state: &mut State, event: &Event, shell: &mut Shell<'_, Message>) {
    if pressed(event) && state.focused {
        state.focused = false;
        shell.request_redraw();
    }
}
