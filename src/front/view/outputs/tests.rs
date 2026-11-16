// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the columns of the outputs view, which give way to the device

use super::{Column, DEVICE_ROOM, device_room, fitting};
use crate::{
    back::{Command, CueList, Fault, OutputEdit, OutputId, Workspace},
    front::{App, view::char_width},
};

/// Returns an app whose Main has a long name and a state that tells that
/// its channels lie past those of its device
fn app() -> App {
    let name = "Far away speakers of the balcony".to_owned();
    App {
        workspace: Workspace::new(vec![CueList::new("Main", Vec::new())])
            .apply(Command::EditOutput(OutputId(1), OutputEdit::Name(name))),
        audio_problems: vec![(OutputId(1), Fault::Channels(64))],
        ..App::default()
    }
}

/// Returns the pixels of `columns` columns of text.  The tests size the
/// table in columns, since the monospace font that a build finds, and so the
/// width of a column, differs from one system to another.
fn wide(columns: f32) -> f32 {
    columns * char_width()
}

#[test]
fn device_keeps_its_room_while_the_other_columns_give_way() {
    let app = app();
    [69.0, 73.0, 83.0, 125.0]
        .into_iter()
        .map(wide)
        .for_each(|room| {
            let columns = fitting(&app, room);
            assert!(
                device_room(&columns, room) >= DEVICE_ROOM * char_width(),
                "{room}"
            );
        });
}

#[test]
fn nothing_gives_way_where_everything_fits() {
    let columns = fitting(&app(), wide(166.0));
    assert!(
        columns
            .iter()
            .any(|(column, _)| matches!(column, Column::NewCues))
    );
    let narrow = fitting(&app(), wide(69.0));
    assert!(
        narrow
            .iter()
            .all(|(column, _)| !matches!(column, Column::NewCues))
    );
}
