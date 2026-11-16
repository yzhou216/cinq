// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What a change to the outputs did to the cues, as the status bar tells it
//!
//! Cues that name no sends play on the first output, and cues that send only
//! to outputs that have gone play on none, so that a change to the outputs can
//! move the sound of cues that it never touched.

use itertools::Itertools;

use super::count;
use crate::back::{Output, Workspace};

/// Tells where the cues that name no outputs play and how many cues send
/// nowhere, when a change to the outputs, from `before` to `after`, changed
/// either
pub fn consequences(workspace: &Workspace, before: &[Output], after: &[Output]) -> Option<String> {
    let was = came_from(workspace, before, after);
    let told = [
        rerouting(workspace, was, workspace.outputs()),
        stranding(workspace, was),
    ]
    .into_iter()
    .flatten()
    .join(", ");
    (!told.is_empty()).then_some(told)
}

/// Returns the outputs that the change from `before` to `after` started
/// from, which are `after` while the show holds `before`, as it does once the
/// change is undone
fn came_from<'a>(workspace: &Workspace, before: &'a [Output], after: &'a [Output]) -> &'a [Output] {
    if workspace.outputs() == before {
        after
    } else {
        before
    }
}

fn rerouting(workspace: &Workspace, was: &[Output], now: &[Output]) -> Option<String> {
    let first = now.first()?;
    let moved = was.first().is_none_or(|was| was.id != first.id);
    let unrouted = workspace.cues().any(|cue| cue.sends.is_none());
    (moved && unrouted).then(|| format!("so that cues with no sends play on {}", first.name))
}

/// Tells how many cues send nowhere now, when the change added to them
fn stranding(workspace: &Workspace, was: &[Output]) -> Option<String> {
    let now = workspace.stranded().count();
    (now > workspace.stranded_among(was)).then(|| {
        let verb = if now == 1 { "sends" } else { "send" };
        format!("{} now {verb} nowhere", count(now, "cue"))
    })
}
