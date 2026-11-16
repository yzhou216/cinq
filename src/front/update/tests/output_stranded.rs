// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the notices that count the cues which send to no output once
//! outputs are cut, deleted, or brought back

use super::{
    history::after,
    output_editing::{on, typed},
};
use crate::{
    back::{Command, CueId, Edit, OutputId, Sends},
    front::{App, Message, Setting, view::format},
};

/// Returns the app whose first cue sends to House L and House R alone
fn sending_to_the_house() -> App {
    let app = on(1);
    let sends: Sends = [(OutputId(1), 0.0), (OutputId(2), -3.0)].into();
    App {
        workspace: app
            .workspace
            .clone()
            .apply(Command::Edit(CueId(1), Edit::Sends(Some(sends)))),
        ..app
    }
}

fn told(app: &App) -> String {
    let (_, change) = app.workspace.latest_change().expect("a change");
    format::change(&app.workspace, change)
}

#[test]
fn a_cut_that_takes_every_output_of_a_cue_counts_the_cues_that_send_nowhere() {
    let cut = typed(sending_to_the_house(), "2dd");
    let clipped = cut.clipped.as_ref().map(|clipped| clipped.told.as_str());
    assert_eq!(
        clipped,
        Some("Cut 2 outputs, so that cues with no sends play on Sub, 1 cue now sends nowhere")
    );
    assert_eq!(cut.workspace.stranded().count(), 1);
    // One output of the two is enough for the cue to play
    let part = typed(sending_to_the_house(), "dd");
    assert_eq!(
        part.clipped.map(|clipped| clipped.told).as_deref(),
        Some("Cut the output House L, so that cues with no sends play on House R")
    );
}

#[test]
fn a_delete_counts_them_too_and_undo_and_redo_tell_only_what_they_bring_about() {
    let deleted = after(sending_to_the_house(), [Message::Delete(2)]);
    assert_eq!(
        told(&deleted),
        "deleted 2 outputs, so that cues with no sends play on Sub, 1 cue now sends nowhere"
    );
    // Undo brings the sound back, and the cue sends nowhere no more
    let undone = typed(deleted, "u");
    assert_eq!(undone.workspace.stranded().count(), 0);
    assert_eq!(
        told(&undone),
        "deleted 2 outputs, so that cues with no sends play on House L"
    );
    let redone = after(undone, [Message::Redo(1)]);
    assert!(told(&redone).ends_with("1 cue now sends nowhere"));
}

#[test]
fn cues_that_were_stranded_already_are_not_told_again() {
    let cut = typed(sending_to_the_house(), "2dd");
    let id = cut.workspace.outputs()[0].id;
    let renamed = after(
        cut,
        [Message::SettingTyped(
            Setting::OutputName(id),
            "Sub 2".to_owned(),
        )],
    );
    assert_eq!(told(&renamed), "renamed the output Sub to Sub 2");
    assert!(!told(&renamed).contains("nowhere"));
}
