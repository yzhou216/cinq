// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of turning the stage window fullscreen and back with the key

use iced::window::{self, Mode};

use super::{super::toggled, App, Kind, Message, show};

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

#[test]
fn key_of_fullscreen_opens_the_closed_stage_window_fullscreen() {
    let (open, _) = show(Kind::Video).staged();
    let stage = open.stage.window().expect("the stage window opens");
    let (closed, _) = open.closed(stage);
    let (reopening, opening) = closed.toggled_fullscreen();
    assert!(reopening.stage.window().is_some());
    assert_eq!(opening.units(), 1);
    assert!(!reopening.stage.closed);
}

#[test]
fn key_of_fullscreen_opens_no_window_where_no_cue_shows_on_it() {
    let (audio, opening) = show(Kind::Audio).toggled_fullscreen();
    assert_eq!((audio.stage.window(), opening.units()), (None, 0));
    let (open, _) = show(Kind::Video).staged();
    let stage = open.stage.window().expect("the stage window opens");
    let (closed, _) = open.closed(stage);
    let emptied = App {
        workspace: show(Kind::Audio).workspace,
        ..closed
    };
    let (emptied, opening) = emptied.toggled_fullscreen();
    assert_eq!((emptied.stage.window(), opening.units()), (None, 0));
    assert!(!emptied.stage.cued);
}

#[test]
fn closed_stage_window_is_missing_only_while_a_cue_shows_on_it() {
    let (open, _) = show(Kind::Video).staged();
    let (gone, _) = open.handled(Message::Go);
    let stage = gone.stage.window().expect("the stage window opens");
    assert!(!gone.stage.missing(&gone.workspace));
    let (closed, _) = gone.closed(stage);
    assert!(closed.stage.missing(&closed.workspace));
    let (reopened, _) = closed.toggled_fullscreen();
    assert!(!reopened.stage.missing(&reopened.workspace));
    let (open, _) = show(Kind::Video).staged();
    let stage = open.stage.window().expect("the stage window opens");
    let (closed, _) = open.closed(stage);
    assert!(!closed.stage.missing(&closed.workspace));
}
