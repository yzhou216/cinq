// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of a file chosen for a new cue, a level half typed, and a failed link

use iced::window;

use super::show;
use crate::{
    back::{CueId, Place, Workspace},
    front::{App, MAIN_SIZE, Message},
};

#[test]
fn sound_file_chosen_for_a_new_cue_names_it_by_its_path_from_the_show() {
    let app = App {
        workspace: Workspace::new(Vec::new()).directory("/show"),
        ..App::default()
    };
    let (added, _) = app.handled(Message::Add(Place::After, 1));
    let id = added.selected.expect("a new cue is selected");
    let chosen = Message::FileChosen(id, "/show/sounds/storm.ogg".into());
    let (chosen, _) = added.handled(chosen);
    let cue = chosen.workspace.cue(id).unwrap();
    assert_eq!(
        (cue.file.as_str(), cue.name.as_str()),
        ("sounds/storm.ogg", "storm")
    );
}

#[test]
fn level_typed_so_far_shows_until_the_operator_does_anything_else() {
    let app = show(Some(CueId(1)));
    let typed = |text: &str| Message::LevelTyped(CueId(1), text.to_owned());
    let (half, _) = app.next(typed("-"));
    assert_eq!(half.typed_level, Some((CueId(1), "-".to_owned())));
    let (clicked, _) = half.next(Message::Clicked);
    assert!(clicked.typed_level.is_some());
    let (typed, _) = clicked.next(typed("-6"));
    assert_eq!(typed.workspace.cue(CueId(1)).unwrap().level, -6.0);
    let (resized, _) = typed.next(Message::Resized(window::Id::unique(), MAIN_SIZE));
    assert!(resized.typed_level.is_some());
    let (moved, _) = resized.next(Message::Step(1));
    assert_eq!(moved.typed_level, None);
}

#[test]
fn link_that_fails_to_open_shows_until_the_help_page_closes() {
    let failure = || Message::LinkOpened(Err("Cannot open it".to_owned()));
    let open = App {
        help: true,
        ..App::default()
    };
    let (failed, _) = open.next(failure());
    assert_eq!(failed.link_problem.as_deref(), Some("Cannot open it"));
    let (closed, _) = failed.next(Message::CloseHelp);
    assert_eq!(closed.link_problem, None);
    let (unseen, _) = closed.next(failure());
    assert_eq!(unseen.link_problem, None);
}
