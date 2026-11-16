// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the columns of the outputs view, which give way to the device

use super::{Column, DEVICE_ROOM, device_room, fitting};
use crate::{
    back::{Command, CueList, Device, Fault, OutputEdit, OutputId, Workspace},
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

/// Returns an app whose Main plays on a device named `name`
fn on_device(name: &str) -> App {
    let device = Device::Named {
        id: name.to_owned(),
        name: name.to_owned(),
    };
    App {
        workspace: Workspace::new(vec![CueList::new("Main", Vec::new())])
            .apply(Command::EditOutput(OutputId(1), OutputEdit::Device(device))),
        ..App::default()
    }
}

/// Returns the pixels of `columns` columns of text.  The tests size the
/// table in columns, since the monospace font that a build finds, and so the
/// width of a column, differs from one system to another.
fn wide(columns: f32) -> f32 {
    columns * char_width()
}

/// Returns whether the table of `room` columns keeps the New cues column
fn shows_new_cues(app: &App, room: f32) -> bool {
    fitting(app, wide(room))
        .iter()
        .any(|(column, _)| matches!(column, Column::NewCues))
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
fn the_new_cues_column_shows_in_a_wide_table_and_goes_from_a_narrow_one() {
    assert!(shows_new_cues(&app(), 166.0));
    assert!(!shows_new_cues(&app(), 69.0));
}

#[test]
fn new_cues_stays_only_while_the_device_shows_on_one_line() {
    // Device name of 23 columns, which a table narrower than the whole of its
    // columns wraps
    let app = on_device("Dante Virtual Soundcard");
    let kept: Vec<f32> = (40..200)
        .map(|room| room as f32)
        .filter(|&room| shows_new_cues(&app, room))
        .collect();
    assert!(!kept.is_empty(), "a wide table keeps the column");
    assert!(shows_new_cues(&app, 199.0) && !shows_new_cues(&app, 40.0));
    let narrowest = kept[0];
    assert!(!shows_new_cues(&app, narrowest - 1.0));
    // Every room from the narrowest that keeps it up keeps it, so that
    // the column does not come and go as the window grows
    assert!(kept.windows(2).all(|pair| pair[1] == pair[0] + 1.0));
    let columns = fitting(&app, wide(narrowest));
    assert!(device_room(&columns, wide(narrowest)) >= 23.0 * char_width());
}

#[test]
fn a_device_name_too_long_for_any_table_does_not_drive_new_cues_away() {
    assert!(shows_new_cues(&on_device(&"x".repeat(60)), 166.0));
}
