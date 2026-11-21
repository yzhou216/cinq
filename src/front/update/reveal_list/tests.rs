// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of scrolling the sidebar back to its current list

use iced::{
    Point, Size,
    widget::operation::{AbsoluteOffset, RelativeOffset},
};

use super::*;

#[derive(Default)]
struct Scrolled(Option<f32>);

impl Scrollable for Scrolled {
    fn snap_to(&mut self, _offset: RelativeOffset<Option<f32>>) {}

    fn scroll_to(&mut self, offset: AbsoluteOffset<Option<f32>>) {
        self.0 = offset.y;
    }

    fn scroll_by(
        &mut self,
        _offset: AbsoluteOffset,
        _bounds: Rectangle,
        _content_bounds: Rectangle,
    ) {
    }
}

/// Visits a sidebar whose content starts 300 pixels down and whose view shows
/// 100 of it from `scrolled` on, with the current list `entry_y` pixels down,
/// and returns where it scrolls to, if it does
fn scrolling(entry_y: f32, scrolled: f32) -> Option<f32> {
    let at = |y, height| Rectangle::new(Point::new(10.0, y), Size::new(80.0, height));
    let (bounds, content) = (at(300.0, 100.0), at(300.0, 1000.0));
    let id = Id::new(view::SIDEBAR_SCROLL);
    let mut operation = Current::default();
    operation.scrollable(
        Some(&id),
        bounds,
        content,
        Vector::new(0.0, scrolled),
        &mut Scrolled::default(),
    );
    operation.container(Some(&Id::new(view::CURRENT_LIST)), at(entry_y, 20.0));
    let Outcome::Chain(mut chained) = operation.finish() else {
        return None;
    };
    let mut list = Scrolled::default();
    chained.scrollable(
        Some(&id),
        bounds,
        content,
        Vector::new(0.0, scrolled),
        &mut list,
    );
    list.0
}

#[test]
fn a_sidebar_scrolled_past_its_current_list_scrolls_back_to_it() {
    // List 20 pixels down lies above a view that starts 500 down, and the
    // gap of the column stays above it
    assert_eq!(scrolling(320.0, 500.0), Some(20.0 - view::SPACING));
}

#[test]
fn a_current_list_in_view_leaves_the_sidebar_where_it_is() {
    assert_eq!(scrolling(320.0, 0.0), None);
}
