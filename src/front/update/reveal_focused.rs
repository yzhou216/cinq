// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Scrolling the sidebar so that the list that Tab reached shows
//!
//! The lists of the sidebar scroll within it when the window is low, so Tab
//! can reach a list that lies below the part in view, or above it.  An
//! operation finds the focused field among those inside the sidebar, and
//! scrolls to show the list with its title above it.

use iced::{
    Rectangle, Task, Vector,
    advanced::widget::{
        Id, Operation, operate,
        operation::{Focusable, Outcome, Scrollable, scrollable},
    },
    widget::operation::AbsoluteOffset,
};

use crate::front::{reveal, view};

/// Scrolls the sidebar to show the list that has the keyboard, if it lies out
/// of view
pub(super) fn reveal_focused<T: Send + 'static>() -> Task<T> {
    // Title above a list of the sidebar, with the gap that the column puts
    // between them and below
    let sidebar = Revealing::new(view::SIDEBAR_SCROLL, view::line_height() + view::SPACING);
    operate(sidebar).discard()
}

/// Finds a scrollable and the field in it that has the keyboard
struct Revealing {
    /// Identity of the scrollable
    scrollable: &'static str,
    /// Height above a field that belongs to it and shows with it
    label: f32,
    /// Whether the next traversal enters the scrollable
    entering: bool,
    /// Whether the widgets visited now lie within it
    inside: bool,
    /// Top of the content, how far it has scrolled, and the height in view
    list: Option<(f32, f32, f32)>,
    field: Option<Rectangle>,
}

impl Revealing {
    fn new(scrollable: &'static str, label: f32) -> Self {
        Self {
            scrollable,
            label,
            entering: false,
            inside: false,
            list: None,
            field: None,
        }
    }
}

impl Operation for Revealing {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        let outside = self.inside;
        self.inside |= std::mem::take(&mut self.entering);
        operate(self);
        self.inside = outside;
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&Id::new(self.scrollable)) {
            self.entering = true;
            self.list = Some((content_bounds.y, translation.y, bounds.height));
        }
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        if self.inside && state.is_focused() {
            self.field = Some(bounds);
        }
    }

    fn finish(&self) -> Outcome<()> {
        let (Some((origin, scrolled, height)), Some(field)) = (self.list, self.field) else {
            return Outcome::None;
        };
        let top = field.y - origin;
        let Some(offset) = reveal(top - self.label, top + field.height, scrolled, height) else {
            return Outcome::None;
        };
        let to = AbsoluteOffset {
            x: None,
            y: Some(offset.max(0.0)),
        };
        Outcome::Chain(Box::new(scrollable::scroll_to(
            Id::new(self.scrollable),
            to,
        )))
    }
}

#[cfg(test)]
mod tests;
