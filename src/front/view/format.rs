// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Text for back-end values, kept apart from the widgets that show it

mod chain;
mod change;
mod consequences;
#[cfg(test)]
mod tests;

use std::path::Path;

use crate::back::{Active, Advance, Cue, Groups, Kind, Media, Output, Phase, SILENCE, Workspace};
pub use chain::{chain, moving};
pub use change::change;
use consequences::short;
pub use consequences::{consequences, consequences_of, leading};

/// Formats seconds as `m:ss.s`
fn clock(seconds: f64) -> String {
    let tenths = (seconds.max(0.0) * 10.0).round() as u32;
    format!("{}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10)
}

/// Formats a time ahead as `4.5s`, or as a clock from a minute on, so that it
/// reads as the time left of a running cue does
fn span(seconds: f64) -> String {
    if seconds < 59.95 {
        format!("{seconds:.1}s")
    } else {
        clock(seconds)
    }
}

/// Tells the loop that a cue plays where it differs from the times typed, as
/// when a time lies outside the clip, and nothing where the cue plays what was
/// typed
pub fn loop_played(cue: &Cue) -> Option<String> {
    cue.loop_played()
        .map(|(start, end)| format!("Loop plays from {} to {}", clock(start), clock(end)))
}

/// Leaves zero blank, so that real waits stand out
pub fn wait(seconds: f64) -> String {
    if seconds > 0.0 {
        clock(seconds)
    } else {
        String::new()
    }
}

/// Formats the length of a cue's action, or a hold for a picture cue, whose
/// action lasts until something stops it.  The action of a group lasts until
/// the last of those of the cues that it holds ends, which a cue that never
/// ends makes a hold, and is none for a group that holds no cue.
pub fn action(groups: &Groups, cue: &Cue) -> String {
    let length = clock(groups.action_of(cue).into());
    match (cue.kind, &cue.media) {
        (Kind::Audio | Kind::Video, Media::Loaded { .. }) if cue.loops() => {
            format!("{length} loop")
        }
        (Kind::Audio | Kind::Video, Media::Loaded { .. })
        | (Kind::Light | Kind::Fade | Kind::Wait, _) => length,
        (Kind::Picture, Media::Loaded { .. }) => "hold".to_owned(),
        (Kind::Audio | Kind::Video | Kind::Picture, _) => "?".to_owned(),
        (
            Kind::Start
            | Kind::Stop
            | Kind::Pause
            | Kind::Devamp
            | Kind::GoTo
            | Kind::Network
            | Kind::Memo,
            _,
        ) => "-".to_owned(),
        (Kind::Group, _) if groups.endless(cue) => "hold".to_owned(),
        (Kind::Group, _) if groups.members(cue.id).is_empty() => "-".to_owned(),
        (Kind::Group, _) => length,
    }
}

/// Leaves the post-wait blank for auto-follow, which ignores it
pub fn post_wait(cue: &Cue) -> String {
    if cue.advance == Advance::Follow {
        String::new()
    } else {
        wait(cue.post_wait.value().into())
    }
}

/// Returns the short label of the Then column
pub fn advance(advance: Advance) -> &'static str {
    match advance {
        Advance::Manual => "",
        Advance::Continue => "continue",
        Advance::Follow => "follow",
    }
}

/// Formats what a cue acts on: its file, its target, or, for a group, how
/// many cues it holds
pub fn target(workspace: &Workspace, groups: &Groups, cue: &Cue) -> String {
    match cue.kind {
        Kind::Audio | Kind::Video | Kind::Picture => cue
            .file_name()
            .and_then(|name| Path::new(name).file_name())
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        Kind::Fade | Kind::Start | Kind::Stop | Kind::Pause | Kind::Devamp | Kind::GoTo => {
            workspace.target(cue).map_or_else(String::new, |target| {
                format!("{}{}", number(target), elsewhere(workspace, cue, target))
            })
        }
        // Counts every instrument that the cue names, patched or not, as the
        // Levels tab lists them
        Kind::Light => count(cue.lights.len(), "instrument"),
        Kind::Network => cue.message.trim().to_owned(),
        Kind::Group => count(groups.members(cue.id).len(), "cue"),
        Kind::Wait | Kind::Memo => String::new(),
    }
}

/// Tells ` in Effects` for a `target` that lies in a cue list other than
/// that of `cue`, where its number could be mistaken for one of this list,
/// and nothing for one in the same list
fn elsewhere(workspace: &Workspace, cue: &Cue, target: &Cue) -> String {
    let own = workspace.list_of(cue.id);
    workspace
        .list_of(target.id)
        .filter(|other| own.is_some_and(|own| own != *other))
        .map(|other| format!(" in {}", workspace.lists()[other].name))
        .unwrap_or_default()
}

/// Formats a level in dB without the unit, as the level field holds it
pub fn decibels(db: f32) -> String {
    format!("{db:+.1}")
}

/// Counts things by name, as in `1 cue` and `2 cues`
pub fn count(number: usize, noun: &str) -> String {
    let plural = if number == 1 { "" } else { "s" };
    format!("{number} {noun}{plural}")
}

pub fn phase(active: &Active) -> String {
    match active.phase() {
        Phase::Scheduled { starts_in: wait } | Phase::PreWait { remaining: wait } => {
            format!("in {}", span(wait))
        }
        Phase::Running { remaining } => format!("{} left", clock(remaining)),
        Phase::Looping => "looping".to_owned(),
        Phase::Showing => "showing".to_owned(),
        Phase::Endless => "running".to_owned(),
    }
}

/// Describes an active cue in full for the list of running cues, with the
/// time left and the level first, which an operator watches during a fade,
/// so that a cut at the end takes the words that say why first
pub fn activity(active: &Active, cue: &Cue) -> String {
    let paused = if active.paused { ", paused" } else { "" };
    let fading = active.fade.map_or("", |_| ", fading");
    let level = if cue.sounds() {
        format!(", {} dB", decibels(active.level.max(SILENCE)))
    } else {
        String::new()
    };
    format!("{}{level}{paused}{fading}", phase(active))
}

/// Tells what a copy or cut took with `verb`, by the number of the cue, or
/// else by how many cues
pub fn clipped(verb: &str, cues: &[Cue]) -> String {
    match cues {
        [cue] => format!("{verb} cue {}", number(cue)),
        cues => format!("{verb} {} cues", cues.len()),
    }
}

/// Tells what a paste of `cues` cues left behind, `sends` sends to outputs
/// and `targets` targets that the show has no namesake of, and how many of
/// them are `unpatched` light cues, or nothing where it left none
pub fn pasted_losses(
    cues: usize,
    sends: usize,
    targets: usize,
    unpatched: usize,
) -> Option<String> {
    let lost = [(sends, "send"), (targets, "target")]
        .into_iter()
        .filter(|&(number, _)| number > 0)
        .map(|(number, noun)| count(number, noun))
        .collect::<Vec<_>>();
    let told = [
        (!lost.is_empty()).then(|| format!("dropped {}", lost.join(" and "))),
        (unpatched > 0).then(|| format!("{unpatched} unpatched")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    (!told.is_empty()).then(|| format!("Pasted {}, {}", count(cues, "cue"), told.join(", ")))
}

/// Returns whether `told`, the notice of a change, says that a cue plays
/// nowhere or sends to no output now, which the operator cannot see or hear,
/// where an undo of it brings the sound back
pub fn silences(told: &str) -> bool {
    ["to play nowhere", "to no output"]
        .iter()
        .any(|phrase| told.contains(phrase))
        && !told.to_lowercase().starts_with("undid")
}

/// Tells after `told`, which names what an edit did to `cut` cues, that
/// `stopped` of them were running and stopped by it, since the silence has no
/// other cause to read from the bar
pub fn stopping(told: String, cut: usize, stopped: usize) -> String {
    match (cut, stopped) {
        (_, 0) => told,
        (1, _) => format!("{told}, which stopped"),
        (_, stopped) => format!("{told}, {stopped} of which stopped"),
    }
}

/// Tells what an undo or a redo, by `verb`, took back or made again, which is
/// `turns` changes, the last of them being `told`, or that one change alone
pub fn turned(verb: &str, turns: usize, told: &str) -> String {
    match turns {
        0 | 1 => format!("{verb}: {told}"),
        turns => format!("{verb} {turns} changes, the last: {told}"),
    }
}

/// Tells what a copy or cut of outputs took with `verb`, in lower case, by the
/// name of the output, or else by how many outputs, after `facts` of what it
/// did to the cues, which leave out the words that name the output as one
pub fn clipped_outputs(verb: &str, outputs: &[Output], facts: Option<String>) -> String {
    let told = match outputs {
        [output] => format!("{verb} {}", the_output(&output.name, facts.is_some())),
        outputs => format!("{verb} {} outputs", outputs.len()),
    };
    leading(facts, &told)
}

/// Names an output as "the output Sub", or as "Sub" where `terse`, which
/// leaves room for what a change did to the cues
fn the_output(name: &str, terse: bool) -> String {
    if terse {
        short(name)
    } else {
        format!("the output {}", short(name))
    }
}

pub(in crate::front::view) fn number(cue: &Cue) -> String {
    if cue.number.trim().is_empty() {
        format!("\"{}\"", Cue::one_line(&cue.name))
    } else {
        cue.number.clone()
    }
}
