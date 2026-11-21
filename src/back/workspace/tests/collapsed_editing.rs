// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of adding, pasting, moving, and indenting cues next to collapsed
//! groups, which count as one row, as the cue list shows them

mod adding;
mod moving;

use super::*;
use crate::back::Change;

fn collapse(id: u32) -> Command {
    Command::Collapse {
        cues: vec![CueId(id)],
        collapsed: true,
    }
}

fn depths(workspace: &Workspace) -> Vec<u8> {
    workspace.lists()[0].depths()
}

fn raw_depths(workspace: &Workspace) -> Vec<u8> {
    workspace.lists()[0]
        .cues
        .iter()
        .map(|cue| cue.depth)
        .collect()
}

/// Returns a collapsed group of two levels, with a cue above and below it
fn closed() -> Workspace {
    workspace(vec![
        audio(1),
        group(2),
        within(1, audio(3)),
        within(1, group(4)),
        within(2, audio(5)),
        audio(6),
    ])
    .apply(collapse(2))
}

fn target(workspace: &Workspace, id: u32) -> Option<CueId> {
    workspace.cue(CueId(id)).and_then(|cue| cue.target)
}

fn paste(cues: Vec<Cue>, at: Place) -> Command {
    Command::Paste { list: 0, cues, at }
}

/// Asserts that undo and redo bring back the lists of `before`
/// and `after`, with their depths
fn assert_undoes(before: &Workspace, after: &Workspace) {
    let undone = after.clone().apply(Command::Undo);
    assert_eq!(ids(&undone), ids(before));
    assert_eq!(raw_depths(&undone), raw_depths(before));
    let redone = undone.apply(Command::Redo);
    assert_eq!(ids(&redone), ids(after));
    assert_eq!(raw_depths(&redone), raw_depths(after));
}
