// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the text that describes cues

mod chains;
mod changes;
mod playing;
mod standby;
mod targets;

use super::{activity, chain, change, clipped, moving, target};
use crate::back::{
    Advance, Command, Cue, CueId, CueList, Edit, InstrumentId, Kind, OutputId, Place, Sends,
    Workspace, loaded, sound,
};

fn memo(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Memo, "Note").build()
}

/// Returns what the latest change to `workspace` says of itself
fn told(workspace: &Workspace) -> String {
    let (_, latest) = workspace.latest_change().expect("a command is a change");
    change(workspace, latest)
}
