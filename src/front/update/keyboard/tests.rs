// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of finding the widget that has the keyboard

use iced::Rectangle;

use super::*;

struct Field(bool);

impl Focusable for Field {
    fn is_focused(&self) -> bool {
        self.0
    }

    fn focus(&mut self) {}

    fn unfocus(&mut self) {}
}

/// Visits one widget per entry of `fields`, as iced does for each window in
/// turn, and returns what the operation finds
fn holding(fields: &[bool]) -> bool {
    let mut operation = Holding(false);
    fields.iter().for_each(|&focused| {
        operation.focusable(None, Rectangle::default(), &mut Field(focused));
    });
    matches!(operation.finish(), Outcome::Some(true))
}

#[test]
fn a_focused_widget_anywhere_means_the_keyboard_is_held() {
    assert!(holding(&[false, true, false]));
    assert!(!holding(&[false, false]));
    assert!(!holding(&[]));
}
