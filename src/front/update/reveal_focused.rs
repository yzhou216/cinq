// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Scrolling the sends of a cue and the sidebar so that the field that Tab
//! reached shows
//!
//! The fields of the sends scroll within the inspector, and the lists of the
//! sidebar scroll within it when the window is low, so Tab can reach a field
//! that lies below the part in view, or above it.  One operation per
//! scrollable finds the focused field among those inside it, and scrolls to
//! show the field with the label above it, where it has one.

use iced::{
    Rectangle, Task, Vector,
    advanced::widget::{
        Id, Operation, operate,
        operation::{Focusable, Outcome, Scrollable},
    },
};

use super::{reveal_list::reveal_current_list, scroll::scrolled_to};
use crate::front::{reveal, view};

/// Scrolls the sends and the sidebar to show the field that has the
/// keyboard, if it is in one of them and lies out of view.  The sidebar first
/// returns to its current list, so that Shift+Tab, which passes the Default
/// device list at its end on the way to Show mode, does not leave the sidebar
/// scrolled there.
pub(super) fn reveal_focused<T: Send + 'static>() -> Task<T> {
    // A send lies in one row with its name, which leaves nothing above its
    // field, while the title of a list of the sidebar lies above it, with the
    // gap that the column puts between them
    let sends = Revealing::new(view::SENDS, 0.0);
    let sidebar = Revealing::new(view::SIDEBAR_SCROLL, view::line_height() + view::SPACING);
    Task::batch([
        operate(sends).discard(),
        reveal_current_list().chain(operate(sidebar).discard()),
    ])
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
        reveal(top - self.label, top + field.height, scrolled, height)
            .map_or(Outcome::None, |offset| scrolled_to(self.scrollable, offset))
    }
}

#[cfg(test)]
mod tests;
