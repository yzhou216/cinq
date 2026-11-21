// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of turning the stage window fullscreen and back with the key

use iced::window::{self, Mode};

use super::super::toggled;

#[test]
fn leaving_fullscreen_asks_the_main_window_for_the_keyboard() {
    let (stage, main) = (window::Id::unique(), window::Id::unique());
    assert_eq!(toggled(stage, Some(main), Mode::Fullscreen).units(), 2);
}

#[test]
fn entering_fullscreen_leaves_the_keyboard_alone() {
    let (stage, main) = (window::Id::unique(), window::Id::unique());
    assert_eq!(toggled(stage, Some(main), Mode::Windowed).units(), 1);
}

#[test]
fn leaving_fullscreen_with_no_main_window_only_switches_the_mode() {
    assert_eq!(
        toggled(window::Id::unique(), None, Mode::Fullscreen).units(),
        1
    );
}
