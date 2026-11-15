// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What a change to the show says of itself, as the status bar tells undo and redo

use super::number;
use crate::back::{Change, Cue, Edit, Workspace};

/// Describes a change to the show in few words, starting in lower case, as
/// the status bar tells what undo or redo acts on
pub fn change(workspace: &Workspace, change: &Change) -> String {
    let named = |id| workspace.cue(id).map_or_else(String::new, number);
    match change {
        Change::Added { cues, .. } => of_cues("added", cues.iter().map(|placed| &placed.cue)),
        Change::Pasted { cues, .. } => of_cues("pasted", cues.iter().map(|placed| &placed.cue)),
        Change::Deleted { cues, cut, .. } => of_cues(
            if *cut { "cut" } else { "deleted" },
            cues.iter().map(|placed| &placed.cue),
        ),
        Change::Moved { cue, .. } => format!("moved cue {}", named(*cue)),
        Change::Edited { cue, redo, .. } => {
            let cue = named(*cue);
            match redo {
                Edit::Number(_) => format!("changed the number of cue {cue}"),
                Edit::Name(_) => format!("changed the name of cue {cue}"),
                Edit::File(_) => format!("changed the file of cue {cue}"),
                Edit::FileAndName { .. } => format!("chose the file of cue {cue}"),
                Edit::Start(_) => format!("changed the start of cue {cue}"),
                Edit::End(_) => format!("changed the end of cue {cue}"),
                Edit::Loop(_) => format!("changed whether cue {cue} loops"),
                Edit::Target(_) => format!("changed the target of cue {cue}"),
                Edit::Notes(_) => format!("changed the notes of cue {cue}"),
                Edit::PreWait(_) => format!("changed the pre-wait of cue {cue}"),
                Edit::Duration(_) => format!("changed the duration of cue {cue}"),
                Edit::PostWait(_) => format!("changed the post-wait of cue {cue}"),
                Edit::Advance(_) => format!("changed what follows cue {cue}"),
                Edit::Level(_) => format!("changed the level of cue {cue}"),
                Edit::StopTarget(_) => format!("changed whether cue {cue} stops its target"),
            }
        }
        Change::Standby(standby) => match standby.after.and_then(|id| workspace.cue(id)) {
            Some(cue) => format!("moved standby to cue {}", number(cue)),
            None => "moved standby past the end".to_owned(),
        },
        Change::Restored { .. } => "brought back the unsaved changes".to_owned(),
    }
}

/// Tells what `verb` did to `cues`, by the number of the cue, or else by how
/// many cues
fn of_cues<'a>(verb: &str, mut cues: impl ExactSizeIterator<Item = &'a Cue>) -> String {
    match (cues.len(), cues.next()) {
        (1, Some(cue)) => format!("{verb} cue {}", number(cue)),
        (count, _) => format!("{verb} {count} cues"),
    }
}
