// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the State column of the cue list and the columns that fit

mod collapsed;
mod fitting;
mod groups;
mod output_failed;
mod send_failed;
mod state;

use super::Column;
use crate::back::{Command, Cue, CueId, CueList, Edit, Kind, Media, OutputId, Sends, Workspace};

fn show(sends: Option<Sends>) -> Workspace {
    let cue = Cue::new(CueId(1), "1", Kind::Audio, "Rain")
        .file("rain.ogg")
        .build();
    Workspace::new(vec![CueList::new("Main", vec![cue])])
        .apply(Command::Loaded {
            cue: CueId(1),
            file: "rain.ogg".to_owned(),
            media: Media::Loaded {
                duration: 10.0,
                sound: Some(10.0),
            },
            seen: None,
        })
        .apply(Command::add_output(1, 1))
        .apply(Command::Edit(CueId(1), Edit::Sends(sends)))
        .apply(Command::DeleteOutputs(vec![OutputId(2)]))
}

fn texts(workspace: &Workspace, column: Column) -> Vec<String> {
    let groups = workspace.lists()[0].groups();
    groups
        .list()
        .cues
        .iter()
        .map(|cue| column.text(workspace, &groups, cue))
        .collect()
}
