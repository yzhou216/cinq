// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Labels of hint mode, drawn in one layer over the window

use iced::{
    Element, Point, Rectangle, Theme,
    widget::{container, pin, stack, text},
};

use crate::front::{
    Message,
    view::{FONT, TEXT_SIZE, char_width, theme},
};

/// Room between the letters of a label and the edge of its box
pub(super) const PADDING: f32 = 2.0;

/// Returns the point of the window that the box of a label of `letters` letters
/// begins at, for a place that lies in `place`.  The box lies left of the place,
/// so that it hides nothing of it, with its right edge at the left edge of the
/// place and its top at the top of the place, except where that would leave the
/// window, when it lies right of the place, with its left edge at the right
/// edge of the place, so that what lies at the left edge of the window stays in
/// view.  The font is monospace, so the width of the box needs no measuring of
/// the text.
pub fn corner(place: Rectangle, letters: usize) -> Point {
    let left = place.x - width(letters);
    Point::new(
        if left < 0.0 {
            place.x + place.width
        } else {
            left
        },
        place.y,
    )
}

pub fn width(letters: usize) -> f32 {
    letters as f32 * char_width() + 2.0 * PADDING
}

/// Returns a layer that draws each of `labels`, the letters still to type with
/// the point of the window that its box begins at.  The layer takes no event.
pub fn layer<'a>(labels: impl Iterator<Item = (&'a str, Point)>) -> Element<'a, Message> {
    stack(labels.map(|(letters, at)| pin(label(letters)).position(at).into())).into()
}

fn label(letters: &str) -> Element<'_, Message> {
    container(text(letters).font(FONT).size(TEXT_SIZE))
        .padding([0.0, PADDING])
        .style(|_: &Theme| container::Style {
            text_color: Some(theme::HINT_TEXT),
            background: Some(theme::HINT_BACKGROUND.into()),
            ..container::Style::default()
        })
        .into()
}
