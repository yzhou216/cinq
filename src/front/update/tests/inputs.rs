// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of a file chosen for a new cue and a failed link

use super::after;
use crate::{
    back::{CueId, Edit, Place, Workspace},
    front::{App, Message},
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

/// Returns the state after a new cue gets the keys of `path` one at a time in
/// its File field, the way the field applies each key, and the cue
fn typed_path(app: App, id: CueId, path: &str) -> App {
    path.char_indices().fold(app, |app, (at, key)| {
        let typed = Edit::File(path[..at + key.len_utf8()].to_owned());
        after(app, [Message::Edit(id, typed), Message::Held(true)])
    })
}

fn added() -> (App, CueId) {
    let (added, _) = App::default().handled(Message::Add(Place::After, 1));
    let id = added.selected.expect("a new cue is selected");
    (added, id)
}

fn named(app: &App, id: CueId) -> (String, String) {
    let cue = app.workspace.cue(id).unwrap();
    (cue.file.clone(), cue.name.clone())
}

#[test]
fn a_typed_path_names_the_new_cue_after_its_file_as_the_path_grows() {
    let (app, id) = added();
    let partway = typed_path(app, id, "sounds/ra");
    assert_eq!(named(&partway, id).1, "ra");
    let (app, id) = added();
    let typed = typed_path(app, id, "sounds/rain.flac");
    assert_eq!(
        named(&typed, id),
        ("sounds/rain.flac".to_owned(), "rain".to_owned())
    );
}

#[test]
fn undo_takes_the_whole_typed_path_back_in_one_step() {
    let (app, id) = added();
    let before = named(&app, id);
    let typed = typed_path(app, id, "sounds/rain.flac");
    assert_eq!(named(&after(typed, [Message::Undo(1)]), id), before);
}

#[test]
fn a_name_that_the_operator_typed_stays_when_a_path_is_typed() {
    let (app, id) = added();
    let renamed = after(app, [Message::Edit(id, Edit::Name("Thunder".to_owned()))]);
    let typed = typed_path(renamed, id, "rain.flac");
    assert_eq!(
        named(&typed, id),
        ("rain.flac".to_owned(), "Thunder".to_owned())
    );
}

#[test]
fn a_path_typed_again_after_the_field_was_emptied_names_the_cue_again() {
    let (app, id) = added();
    let emptied = after(
        typed_path(app, id, "rain.flac"),
        [Message::Edit(id, Edit::File(String::new()))],
    );
    assert_eq!(named(&emptied, id).1, "New Audio cue");
    let again = typed_path(emptied, id, "wind.flac");
    assert_eq!(named(&again, id).1, "wind");
}
