// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the rules that a workspace applies, with the helpers that
//! build the shows they run on, which the tests of the sound and video
//! outputs borrow

mod adding;
mod collapsed_editing;
mod collapsing;
mod devamp_cues;
mod devamping;
mod editing;
mod files;
mod going;
mod group_control;
mod grouping;
mod groups_adding;
mod groups_deleting;
mod groups_moving;
mod groups_pasting;
mod groups_playing;
mod history;
mod holding;
mod levels;
mod light;
mod loop_progress;
mod network;
mod output_pasting;
mod outputs;
mod pasting;
mod picture;
mod playback;
mod show;
mod show_loops;
mod starting;
mod stopping;
mod video;

use std::path::Path;

use super::{Workspace, playback::PANIC_FADE};
use crate::back::{
    Advance, Command, Cue, CueId, CueList, Edit, Kind, Media, Phase, Place, Problem, SILENCE,
};

/// Returns a workspace with one list of cues, whose picture files have
/// loaded as pictures and every other file as ten seconds of sound
pub(in crate::back) fn workspace(cues: Vec<Cue>) -> Workspace {
    lists_of(vec![cues])
}

/// Returns a workspace with a list for each set of cues, loaded as for
/// [`workspace`]
fn lists_of(lists: Vec<Vec<Cue>>) -> Workspace {
    let workspace = Workspace::new(
        lists
            .into_iter()
            .enumerate()
            .map(|(index, cues)| match index {
                0 => CueList::new("Main", cues),
                _ => CueList::new(format!("List {}", index + 1), cues),
            })
            .collect(),
    );
    let loads: Vec<Command> = workspace
        .unloaded()
        .map(|cue| {
            let media = match cue.kind {
                Kind::Picture => silent(0.0),
                _ => sound(10.0),
            };
            loaded(cue.id.0, &cue.file, media)
        })
        .collect();
    loads.into_iter().fold(workspace, Workspace::apply)
}

pub(crate) fn sound(seconds: f32) -> Media {
    Media::Loaded {
        duration: seconds,
        sound: Some(seconds),
    }
}

/// Returns the media of a file that plays `seconds` of picture alone
pub(crate) fn silent(seconds: f32) -> Media {
    Media::Loaded {
        duration: seconds,
        sound: None,
    }
}

pub(crate) fn loaded(id: u32, file: &str, media: Media) -> Command {
    Command::Loaded {
        cue: CueId(id),
        file: file.to_owned(),
        media,
    }
}

fn add(list: usize, kind: Kind, at: Place) -> Command {
    Command::Add {
        list,
        kind,
        at,
        count: 1,
    }
}

fn moving(id: u32, by: isize) -> Command {
    Command::Move { cue: CueId(id), by }
}

fn edit(id: u32, edit: Edit) -> Command {
    Command::Edit(CueId(id), edit)
}

fn rename(id: u32, name: &str) -> Command {
    edit(id, Edit::Name(name.to_owned()))
}

fn delete(ids: &[u32]) -> Command {
    Command::Delete(ids.iter().map(|&id| CueId(id)).collect())
}

fn move_standby(id: u32) -> Command {
    Command::Standby {
        list: 0,
        cue: Some(CueId(id)),
    }
}

fn ids(workspace: &Workspace) -> Vec<u32> {
    workspace.lists()[0]
        .cues
        .iter()
        .map(|cue| cue.id.0)
        .collect()
}

fn standby(workspace: &Workspace) -> Option<u32> {
    workspace.lists()[0].standby.map(|id| id.0)
}

/// Returns the cues that GO has fired and that have not finished, in the
/// order that GO fired them
fn running(workspace: &Workspace) -> Vec<u32> {
    workspace
        .active()
        .iter()
        .map(|active| active.id.0)
        .collect()
}

pub(crate) fn audio(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Audio, "Sound")
        .file("sound.ogg")
        .build()
}

fn continued(cue: Cue) -> Cue {
    Cue {
        advance: Advance::Continue,
        ..cue
    }
}

fn continuing(id: u32) -> Cue {
    continued(audio(id))
}

fn continuing_after(id: u32, post_wait: f32) -> Cue {
    Cue {
        post_wait: post_wait.into(),
        ..continuing(id)
    }
}

/// Returns a cue of a control kind that acts on cue `target`
fn control(id: u32, kind: Kind, target: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), kind, "Control")
        .target(CueId(target))
        .build()
}

pub(in crate::back) fn start_cue(id: u32, target: u32) -> Cue {
    control(id, Kind::Start, target)
}

fn pause_cue(id: u32, target: u32) -> Cue {
    control(id, Kind::Pause, target)
}

fn goto_cue(id: u32, target: u32) -> Cue {
    control(id, Kind::GoTo, target)
}

pub(in crate::back) fn devamp_cue(id: u32, target: u32) -> Cue {
    control(id, Kind::Devamp, target)
}

pub(in crate::back) fn group(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Group, "Group").build()
}

/// Returns the cue at `depth` in groups
pub(in crate::back) fn within(depth: u8, cue: Cue) -> Cue {
    Cue { depth, ..cue }
}

/// Returns an audio cue that loops from 2 to 6 seconds into its file of ten
/// seconds, so that it reaches the end of its loop first after 6 seconds
/// and goes round the loop every 4
pub(in crate::back) fn vamp(id: u32) -> Cue {
    Cue {
        looping: true,
        loop_start: 2.0.into(),
        loop_end: 6.0.into(),
        ..audio(id)
    }
}

fn stop_cue(id: u32, target: u32) -> Cue {
    control(id, Kind::Stop, target)
}

/// Returns the standby of the list `list`
fn standby_of(workspace: &Workspace, list: usize) -> Option<u32> {
    workspace.lists()[list].standby.map(|id| id.0)
}

/// Returns a fade cue that fades cue 1 out over four seconds and stops it
fn fade_out(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Fade, "Fade")
        .target(CueId(1))
        .duration(4.0)
        .build()
}

/// Returns a fade cue that fades cue 1 to -18 dB and leaves it playing
fn fade_under(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Fade, "Under")
        .target(CueId(1))
        .level(-18.0)
        .stop_target(false)
        .build()
}

fn level(workspace: &Workspace, id: u32) -> Option<f32> {
    workspace.active_of(CueId(id)).map(|active| active.level)
}

fn opacity(workspace: &Workspace, id: u32) -> f32 {
    workspace
        .active_of(CueId(id))
        .expect("the cue runs")
        .opacity()
}

pub(in crate::back) const GO: Command = Command::Go { list: 0 };

fn shown(workspace: &Workspace) -> String {
    workspace.show(Some(Path::new("/show")))
}

/// Reads the text of a show in `/show`, which must write back as it was
fn read_back(text: &str) -> Workspace {
    let read = Workspace::from_show(text, "/show").expect("the show reads back");
    assert_eq!(shown(&read), text);
    read
}
