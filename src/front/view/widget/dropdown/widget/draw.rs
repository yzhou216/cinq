// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Look of a drop-down list when it is closed

use iced::{
    Point, Rectangle, Renderer, Size, Theme,
    advanced::{
        Renderer as _,
        layout::Layout,
        mouse, renderer,
        text::{self, Renderer as _, Text},
        widget::tree::Tree,
    },
    alignment,
    widget::{
        pick_list::{self, Status},
        text_input,
    },
};

use super::{
    super::{Dropdown, PADDING, shorten::shortened, state::State},
    ARROW,
};
use crate::front::view::{FONT, TEXT_SIZE, char_width, line_height};

/// Returns a label on one line in the default font and size.  It goes by
/// its top left corner, where iced's own pick list goes by its middle,
/// since iced's software renderer redraws a changed text only below and to
/// the right of the point it is drawn at.
fn label<C>(content: C, width: f32) -> Text<C> {
    Text {
        content,
        bounds: Size::new(width, line_height()),
        size: TEXT_SIZE,
        line_height: text::LineHeight::default(),
        font: FONT,
        align_x: text::Alignment::Default,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::default(),
        wrapping: text::Wrapping::None,
    }
}

impl Dropdown<'_> {
    pub(super) fn drawn(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let is_hovered = cursor.is_over(bounds);
        let status = match (state.shared.is_open(), is_hovered) {
            (true, _) => Status::Opened { is_hovered },
            (false, true) => Status::Hovered,
            (false, false) => Status::Active,
        };
        let look = pick_list::default(theme, status);
        // Border of a focused list is that of a focused text field
        let border = if state.focused {
            let focused = text_input::Status::Focused { is_hovered };
            look.border
                .color(text_input::default(theme, focused).border.color)
        } else {
            look.border
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border,
                ..renderer::Quad::default()
            },
            look.background,
        );
        // Arrow drawn by its right end as iced's pick list draws it, since
        // it never changes apart from the box, which redraws around it
        let arrow = Text {
            font: Renderer::ICON_FONT,
            align_x: text::Alignment::Right,
            align_y: alignment::Vertical::Center,
            shaping: text::Shaping::Basic,
            ..label(Renderer::ARROW_DOWN_ICON.to_string(), bounds.width)
        };
        let end = Point::new(bounds.x + bounds.width - PADDING.right, bounds.center_y());
        renderer.fill_text(arrow, end, look.handle_color, *viewport);
        let (name, color) = match self.chosen {
            Some(row) => (self.names[row].clone(), look.text_color),
            None => (
                self.placeholder.clone().unwrap_or_default(),
                look.placeholder_color,
            ),
        };
        let room = bounds.width - PADDING.x() - ARROW;
        let name = label(shortened(&name, (room / char_width()) as usize), room);
        let start = Point::new(
            bounds.x + PADDING.left,
            bounds.center_y() - line_height() / 2.0,
        );
        renderer.fill_text(name, start, color, *viewport);
    }
}
