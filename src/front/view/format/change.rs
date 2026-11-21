// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What a change to the show says of itself, as the status bar tells undo and redo

use super::{consequences, number, short, the_output};
use crate::back::{Change, Cue, Edit, Output, OutputId, Workspace, moved_output};

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
        Change::Regrouped {
            cues, moved: true, ..
        } => cues
            .first()
            .map_or_else(String::new, |&cue| format!("moved cue {}", named(cue))),
        Change::Regrouped { cues, deeper, .. } => {
            let way = if *deeper { "into" } else { "out of" };
            match cues.as_slice() {
                [cue] => format!("moved cue {} {way} a group", named(*cue)),
                cues => format!("moved {} cues {way} groups", cues.len()),
            }
        }
        Change::Edited { cue, redo, .. } => {
            let cue = named(*cue);
            match redo {
                Edit::Number(_) => format!("changed the number of cue {cue}"),
                Edit::Name(_) => format!("changed the name of cue {cue}"),
                Edit::File(_) | Edit::FileAndName { .. } => {
                    format!("changed the file of cue {cue}")
                }
                Edit::Start(_) => format!("changed the start of cue {cue}"),
                Edit::End(_) => format!("changed the end of cue {cue}"),
                Edit::Loop(_) => format!("changed whether cue {cue} loops"),
                Edit::LoopStart(_) => format!("changed the loop start of cue {cue}"),
                Edit::LoopEnd(_) => format!("changed the loop end of cue {cue}"),
                Edit::Target(_) => format!("changed the target of cue {cue}"),
                Edit::Notes(_) => format!("changed the notes of cue {cue}"),
                Edit::PreWait(_) => format!("changed the pre-wait of cue {cue}"),
                Edit::Duration(_) => format!("changed the duration of cue {cue}"),
                Edit::PostWait(_) => format!("changed the post-wait of cue {cue}"),
                Edit::Advance(_) => format!("changed what follows cue {cue}"),
                Edit::Level(_) => format!("changed the level of cue {cue}"),
                Edit::StopTarget(_) => format!("changed whether cue {cue} stops its target"),
                Edit::Sends(_) => format!("changed where cue {cue} plays"),
                Edit::Light(..) => format!("changed the light levels of cue {cue}"),
                Edit::Destination(_) => format!("changed the destination of cue {cue}"),
                Edit::Message(_) => format!("changed the message of cue {cue}"),
            }
        }
        Change::Standby(standby) => match standby.after.and_then(|id| workspace.cue(id)) {
            Some(cue) => format!("moved standby to cue {}", number(cue)),
            None => "moved standby past the end".to_owned(),
        },
        Change::Patched { .. } => "changed the light patch".to_owned(),
        Change::Restored { .. } => "brought back the unsaved changes".to_owned(),
        Change::Outputs {
            before,
            after,
            cut,
            pasted,
            anchor,
            ..
        } => outputs(workspace, before, after, (*cut, *pasted), *anchor),
    }
}

/// Describes a change to the outputs by what differs between them before
/// and after it, leaving the words that name an output as one where the
/// change did something to the cues, which [`consequences`] tells first
fn outputs(
    workspace: &Workspace,
    before: &[Output],
    after: &[Output],
    (cut, pasted): (bool, bool),
    selected: Option<OutputId>,
) -> String {
    let terse = consequences(workspace, before, after).is_some();
    let the = |name: &str| the_output(name, terse);
    let removed = if cut { "cut" } else { "deleted" };
    let added = if pasted { "pasted" } else { "added" };
    let pairs = || {
        before.iter().filter_map(|was| {
            let now = after.iter().find(|now| now.id == was.id)?;
            Some((was, now))
        })
    };
    let edited = pairs().find_map(|(was, now)| {
        let name = &short(&now.name);
        if was.name != now.name {
            Some(format!("renamed the output {} to {name}", short(&was.name)))
        } else if was.device != now.device {
            Some(format!("changed the device of the output {name}"))
        } else if was.channels != now.channels {
            Some(format!("changed the channels of the output {name}"))
        } else if was.new_cues != now.new_cues {
            Some(format!("changed what new cues send to the output {name}"))
        } else {
            None
        }
    });
    let moved =
        || moved_output(before, after, selected).map(|now| format!("moved {}", the(&now.name)));
    match (
        missing(after, before).as_slice(),
        missing(before, after).as_slice(),
    ) {
        ([output], []) => Some(format!("{added} {}", the(&output.name))),
        ([], [output]) => Some(format!("{removed} {}", the(&output.name))),
        (put, []) if !put.is_empty() => Some(format!("{added} {} outputs", put.len())),
        ([], gone) if !gone.is_empty() => Some(format!("{removed} {} outputs", gone.len())),
        (put, deleted) if !put.is_empty() || !deleted.is_empty() => {
            Some("changed the outputs".to_owned())
        }
        _ => edited.or_else(moved),
    }
    .unwrap_or_else(|| "changed the outputs".to_owned())
}

/// Returns the outputs of `from` that `of` lacks
fn missing<'a>(from: &'a [Output], of: &[Output]) -> Vec<&'a Output> {
    from.iter()
        .filter(|output| of.iter().all(|other| other.id != output.id))
        .collect()
}

/// Tells what `verb` did to `cues`, by the number of the cue, or else by how
/// many cues
fn of_cues<'a>(verb: &str, mut cues: impl ExactSizeIterator<Item = &'a Cue>) -> String {
    match (cues.len(), cues.next()) {
        (1, Some(cue)) => format!("{verb} cue {}", number(cue)),
        (count, _) => format!("{verb} {count} cues"),
    }
}
