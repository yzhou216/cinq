// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of scrolling the sidebar to the list that Tab reached

use iced::{
    Point, Size,
    widget::operation::{AbsoluteOffset, RelativeOffset},
};

use super::*;
use crate::front::update::tests::Field;

/// Scrollable that tells where it was scrolled to
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

fn at(y: f32, height: f32) -> Rectangle {
    Rectangle::new(Point::new(10.0, y), Size::new(80.0, height))
}

/// Visits a window as iced does, with a field outside the scrollable `id` and
/// one inside it, and returns where it scrolls to, if it does, with `label`
/// above each field.  The content starts 300 pixels down and the view shows
/// 100 of it from `scrolled` on.
fn scrolling_in(
    id: &'static str,
    label: f32,
    outside: bool,
    field_y: f32,
    scrolled: f32,
) -> Option<f32> {
    let mut operation = Revealing::new(id, label);
    Operation::focusable(
        &mut operation,
        None,
        at(20.0, 30.0),
        &mut Field::new(outside),
    );
    let (bounds, content) = (at(300.0, 100.0), at(300.0, 1000.0));
    let mut list = Scrolled::default();
    Operation::scrollable(
        &mut operation,
        Some(&Id::new(id)),
        bounds,
        content,
        Vector::new(0.0, scrolled),
        &mut list,
    );
    Operation::traverse(&mut operation, &mut |inner: &mut dyn Operation| {
        inner.focusable(None, at(field_y, 30.0), &mut Field::new(!outside));
    });
    let Outcome::Chain(mut chained) = Operation::finish(&operation) else {
        return None;
    };
    chained.scrollable(
        Some(&Id::new(id)),
        bounds,
        content,
        Vector::new(0.0, scrolled),
        &mut list,
    );
    list.0
}

/// Title above a list and the gap that the column puts below it
fn title() -> f32 {
    view::line_height() + view::SPACING
}

fn scrolling(outside: bool, field_y: f32, scrolled: f32) -> Option<f32> {
    scrolling_in(view::SIDEBAR_SCROLL, title(), outside, field_y, scrolled)
}

#[test]
fn the_sends_scroll_only_to_a_field_of_theirs_that_lies_out_of_view() {
    // Field 450 pixels down is 150 below the start of the content, and
    // ends at 180, which a view of 100 shows from 80 on
    assert_eq!(scrolling(false, 450.0, 0.0), Some(80.0));
}

#[test]
fn a_field_above_the_view_scrolls_down_to_its_name() {
    assert_eq!(scrolling(false, 350.0, 200.0), Some(50.0 - title()));
}

#[test]
fn a_field_in_view_leaves_the_sidebar_where_it_is() {
    assert_eq!(scrolling(false, 300.0 + title() + 10.0, 0.0), None);
}

#[test]
fn a_field_outside_the_sidebar_leaves_it_where_it_is() {
    assert_eq!(scrolling(true, 450.0, 0.0), None);
}
