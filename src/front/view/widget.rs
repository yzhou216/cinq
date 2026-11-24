// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Widgets that iced lacks
//!
//! All but [`check`](mod@check), [`dropdown`](mod@dropdown),
//! [`hint`](mod@hint), and [`pictures`](mod@pictures) work around an iced
//! widget with [`wrap`], while the checkbox and the drop-down hold the keyboard
//! focus themselves around iced's checkbox and in the look of iced's pick list,
//! the pictures draw through a shader of their own, and [`hinted`] marks a
//! place that a click acts on for hint mode.

mod button;
mod check;
mod dropdown;
pub(in crate::front::view) mod focus;
mod hide;
pub mod hint;
mod keep_keys;
mod line_keys;
mod marked_row;
mod pictures;
mod radio;
mod wrap;

use iced::{Event, Rectangle, mouse, touch};

use crate::front::keymap::KeyPress;
pub use button::{Button, button};
pub use check::{check, check_and_leave};
pub(in crate::front::view) use dropdown::shortened;
pub use dropdown::{dropdown, width as dropdown_width};
pub use hide::hide;
pub use hint::{Act, Site, Target, hinted, reached, window};
pub use keep_keys::keep_keys;
pub use line_keys::line_keys;
pub use marked_row::marked_row;
pub use pictures::{Layer, pictures};
pub use radio::radio;

fn key_pressed(event: &Event) -> Option<KeyPress<'_>> {
    match event {
        Event::Keyboard(event) => KeyPress::of(event),
        _ => None,
    }
}

fn pressed(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. })
    )
}

fn pointer_over(cursor: mouse::Cursor, bounds: Rectangle) -> mouse::Interaction {
    if cursor.is_over(bounds) {
        mouse::Interaction::Pointer
    } else {
        mouse::Interaction::default()
    }
}
