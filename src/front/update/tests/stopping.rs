// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of what an edit tells of the cues that it stopped

use super::{after, show};
use crate::{
    back::{CueId, loaded, sound},
    front::{App, Message, Setting},
};

const CUE: CueId = CueId(1);

/// Returns the interface with cue 1 playing and selected
fn playing() -> App {
    let app = show(Some(CUE));
    let workspace = app.workspace.apply(loaded(1, "sound.ogg", sound(10.0)));
    after(App { workspace, ..app }, [Message::Go])
}

fn told(app: &App) -> Option<&str> {
    app.clipped.as_ref().map(|clipped| clipped.told.as_str())
}

#[test]
fn a_cut_of_a_cue_that_plays_says_that_it_stopped() {
    let cut = after(playing(), [Message::Select(CUE), Message::Cut(1)]);
    assert!(cut.workspace.active_of(CUE).is_none());
    assert_eq!(told(&cut), Some("Cut cue 1, which stopped"));
    let several = after(playing(), [Message::Select(CUE), Message::Cut(3)]);
    assert_eq!(told(&several), Some("Cut 3 cues, 1 of which stopped"));
}

#[test]
fn a_cut_of_a_cue_that_is_silent_says_nothing_of_it() {
    let silent = after(show(Some(CUE)), [Message::Cut(1)]);
    assert_eq!(told(&silent), Some("Cut cue 1"));
    assert!(silent.stopped.is_empty());
}

#[test]
fn a_delete_remembers_the_cue_that_it_stopped_until_undo() {
    let deleted = after(playing(), [Message::Select(CUE), Message::Delete(1)]);
    assert_eq!(deleted.stopped, [CUE]);
    let undone = after(deleted, [Message::Undo(1)]);
    assert!(undone.stopped.is_empty());
    assert!(undone.workspace.active_of(CUE).is_none());
}

/// Returns the state after `text` goes into the File field of the cue, a key
/// at a time
fn typed_file(app: App, text: &str) -> App {
    text.char_indices().fold(app, |app, (at, key)| {
        let prefix = text[..at + key.len_utf8()].to_owned();
        after(
            app,
            [
                Message::SettingTyped(Setting::File(CUE), prefix),
                Message::Held(true),
            ],
        )
    })
}

#[test]
fn a_path_typed_into_the_file_of_a_cue_that_plays_leaves_it_playing_until_the_field_is_left() {
    let typing = typed_file(playing(), "bell.ogg");
    assert!(typing.workspace.active_of(CUE).is_some());
    assert_eq!(typing.workspace.cue(CUE).unwrap().file, "sound.ogg");
    assert!(typing.stopped.is_empty());
    // The widget reports that it lost the keyboard after the click that left it
    let left = after(typing, [Message::Clicked, Message::Held(false)]);
    assert!(left.workspace.active_of(CUE).is_none());
    assert_eq!(left.workspace.cue(CUE).unwrap().file, "bell.ogg");
    assert_eq!(left.stopped, [CUE]);
}

#[test]
fn the_file_of_a_cue_that_stopped_tells_that_it_did_only_for_the_change_that_stopped_it() {
    let left = after(
        typed_file(playing(), "bell.ogg"),
        [Message::Held(false), Message::Clicked],
    );
    let again = after(
        typed_file(left, "chime.ogg"),
        [Message::Held(false), Message::Clicked],
    );
    assert_eq!(again.workspace.cue(CUE).unwrap().file, "chime.ogg");
    assert!(again.stopped.is_empty());
}

#[test]
fn leaving_the_file_of_a_cue_unchanged_makes_no_change() {
    let typing = typed_file(playing(), "bell.ogg");
    let back = after(
        typing,
        [Message::SettingTyped(
            Setting::File(CUE),
            "sound.ogg".into(),
        )],
    );
    let left = after(back, [Message::Held(false), Message::Clicked]);
    assert!(left.workspace.active_of(CUE).is_some());
    assert!(left.stopped.is_empty());
}
