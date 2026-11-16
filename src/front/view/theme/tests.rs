// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the borders that show where the keyboard is, and of the contrast
//! of text on the selected row

use iced::widget::text_input::Status;

use super::{BLUE_COOLER, FOCUS_WIDTH, RED, field, modus_vivendi};

#[test]
fn a_focused_field_has_a_border_twice_as_thick_as_one_at_rest() {
    let theme = modus_vivendi();
    let focused = Status::Focused { is_hovered: false };
    let at_rest = field(true)(&theme, Status::Active).border;
    let held = field(true)(&theme, focused).border;
    assert_eq!(at_rest.width, 1.0);
    assert_eq!(held.width, FOCUS_WIDTH);
    assert_eq!(held.width, 2.0 * at_rest.width);
    assert_eq!(held.color, BLUE_COOLER);
    assert_eq!(
        field(true)(&theme, Status::Focused { is_hovered: true })
            .border
            .width,
        FOCUS_WIDTH,
        "the pointer over a focused field does not thin it"
    );
}

#[test]
fn a_field_that_holds_a_rejected_value_keeps_its_red_border_in_both_thicknesses() {
    let theme = modus_vivendi();
    let rejected = field(false);
    let at_rest = rejected(&theme, Status::Active).border;
    let held = rejected(&theme, Status::Focused { is_hovered: false }).border;
    assert_eq!((at_rest.color, at_rest.width), (RED, 1.0));
    assert_eq!((held.color, held.width), (RED, FOCUS_WIDTH));
}
