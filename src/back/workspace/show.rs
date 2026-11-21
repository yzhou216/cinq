// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Show files, which hold the light patch and the cue lists of a workspace
//! as TOML
//!
//! A show file lists the instruments of the light patch, then the cue lists
//! of a show in order, each with its cues, and each cue with the settings
//! that its kind uses, leaving out those that keep their defaults, apart
//! from the level, the duration, and whether a fade stops its target, which
//! it holds for every cue whose kind uses them.  Cues and instruments keep
//! their identities, by which fade and stop cues name their targets and
//! light cues their instruments.  What playback and loading find, such as
//! the cue on standby, the cues that run, the light levels, and how long
//! each sound or video file plays, belongs to the session rather than to
//! the show, and the file leaves it out.  The outputs come first, as
//! [`outputs`] tells.  What keeps a file from opening lives in [`check`].

mod check;
mod outputs;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::Workspace;
use crate::back::{Advance, Cue, CueId, CueList, Instrument, InstrumentId, Kind, Output};
use outputs::OutputFile;

#[derive(Serialize, Deserialize)]
struct ShowFile {
    #[serde(default, rename = "output", skip_serializing_if = "Vec::is_empty")]
    outputs: Vec<OutputFile>,
    #[serde(default, rename = "instrument", skip_serializing_if = "Vec::is_empty")]
    instruments: Vec<Instrument>,
    #[serde(default, rename = "list")]
    lists: Vec<ListFile>,
}

#[derive(Serialize, Deserialize)]
struct ListFile {
    name: String,
    #[serde(default, rename = "cue", skip_serializing_if = "Vec::is_empty")]
    cues: Vec<CueFile>,
}

/// Setting that is `None` stays out of the file, since TOML has no null,
/// and one that the file leaves out reads as `None`.
#[derive(Serialize, Deserialize)]
struct CueFile {
    id: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    number: String,
    kind: Kind,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    file: String,
    start: Option<f32>,
    end: Option<f32>,
    #[serde(default, rename = "loop", skip_serializing_if = "std::ops::Not::not")]
    looping: bool,
    loop_start: Option<f32>,
    loop_end: Option<f32>,
    #[serde(default, skip_serializing_if = "is_top")]
    depth: u8,
    target: Option<u32>,
    pre_wait: Option<f32>,
    duration: Option<f32>,
    post_wait: Option<f32>,
    #[serde(default, skip_serializing_if = "is_manual")]
    advance: Advance,
    level: Option<f32>,
    stop_target: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    notes: String,
    /// Levels at which the cue sends to outputs, by their names
    sends: Option<BTreeMap<String, f32>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    lights: BTreeMap<InstrumentId, f32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    destination: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    message: String,
}

impl Workspace {
    /// Reads a show from the text of its file, finding the files of its
    /// cues from `directory`.  Each list starts with standby on its
    /// first cue, and a file without lists holds one empty list, as every
    /// show does.  A file in which two cues or two instruments share an
    /// identity cannot be read, nor can one with an address that no
    /// instrument could be given, or a level that no cue could, since a
    /// level typed wrong by hand, such as 60 for -60, would play at a
    /// thousand times the amplitude of its file.
    pub fn from_show(text: &str, directory: impl Into<PathBuf>) -> Result<Self, String> {
        let show: ShowFile = toml::from_str(text).map_err(|error| error.to_string())?;
        let outputs = outputs::read(show.outputs)?;
        let lists = show
            .lists
            .into_iter()
            .map(|list| list.list(&outputs))
            .collect::<Result<_, _>>()?;
        let workspace = Self::new(lists)
            .directory(directory)
            .with_outputs(outputs)
            .with_patch(show.instruments);
        check::problem(&workspace).map_or(Ok(workspace), Err)
    }

    /// Returns the text of the show as a file in `directory` holds it, with
    /// the relative paths of the files of cues going from that directory,
    /// or as absolute paths without one
    pub fn show(&self, directory: Option<&Path>) -> String {
        let show = ShowFile {
            outputs: outputs::written(&self.outputs),
            instruments: self.patch.clone(),
            lists: self
                .lists
                .iter()
                .map(|list| ListFile {
                    name: list.name.clone(),
                    cues: list
                        .cues
                        .iter()
                        .map(|cue| {
                            CueFile::of(cue, self.written_file(cue, directory), &self.outputs)
                        })
                        .collect(),
                })
                .collect(),
        };
        toml::to_string(&show).expect("TOML holds every setting of a show")
    }

    /// Returns the file that `cue` plays as a show file in `directory`
    /// names it: an absolute path as it is, and a relative one from
    /// `directory` rather than from the directory of the workspace, or as
    /// an absolute path without one
    fn written_file(&self, cue: &Cue, directory: Option<&Path>) -> String {
        let Some(name) = cue.file_name().filter(|_| cue.kind.has_file()) else {
            return String::new();
        };
        if Path::new(name).is_absolute() || directory == Some(self.directory.as_path()) {
            return name.to_owned();
        }
        let absolute = self.directory.join(name);
        directory
            .and_then(|directory| pathdiff::diff_paths(&absolute, directory))
            .unwrap_or(absolute)
            .to_string_lossy()
            .into_owned()
    }
}

impl ListFile {
    fn list(self, outputs: &[Output]) -> Result<CueList, String> {
        let cues = self
            .cues
            .into_iter()
            .map(|cue| cue.cue(outputs))
            .collect::<Result<_, _>>()?;
        Ok(CueList::new(self.name, cues))
    }
}

impl CueFile {
    fn of(cue: &Cue, file: String, outputs: &[Output]) -> Self {
        let set = |seconds: f32| Some(seconds).filter(|&seconds| seconds > 0.0);
        let plays = cue.kind.plays_file();
        let network_only = |text: &str| {
            if cue.kind == Kind::Network {
                text.to_owned()
            } else {
                String::new()
            }
        };
        Self {
            id: cue.id.0,
            number: cue.number.clone(),
            kind: cue.kind,
            name: cue.name.clone(),
            file,
            start: set(cue.start.value()).filter(|_| plays),
            end: cue.end.time().filter(|_| plays),
            looping: plays && cue.looping,
            loop_start: cue.loop_start.time().filter(|_| plays),
            loop_end: cue.loop_end.time().filter(|_| plays),
            depth: cue.depth,
            target: cue
                .target
                .filter(|_| cue.kind.has_target())
                .map(|target| target.0),
            pre_wait: set(cue.pre_wait.value()),
            duration: cue.kind.has_duration().then(|| cue.duration.value()),
            post_wait: set(cue.post_wait.value()),
            advance: cue.advance,
            level: cue.kind.has_level().then_some(cue.level),
            stop_target: (cue.kind == Kind::Fade).then_some(cue.stop_target),
            notes: cue.notes.clone(),
            sends: cue
                .sends
                .as_ref()
                .filter(|_| plays)
                .map(|sends| outputs::sends_by_name(sends, outputs)),
            lights: if cue.kind == Kind::Light {
                cue.lights.clone()
            } else {
                BTreeMap::new()
            },
            destination: network_only(&cue.destination),
            message: network_only(&cue.message),
        }
    }

    fn cue(self, outputs: &[Output]) -> Result<Cue, String> {
        let sends = self
            .sends
            .map(|sends| outputs::sends_by_id(sends, outputs, self.id))
            .transpose()?;
        Ok(Cue::new(CueId(self.id), self.number, self.kind, self.name)
            .file(self.file)
            .maybe_start(self.start)
            .maybe_end(self.end)
            .looping(self.looping)
            .maybe_loop_start(self.loop_start)
            .maybe_loop_end(self.loop_end)
            .depth(self.depth)
            .maybe_target(self.target.map(CueId))
            .maybe_pre_wait(self.pre_wait)
            .maybe_duration(self.duration)
            .maybe_post_wait(self.post_wait)
            .advance(self.advance)
            .maybe_level(self.level)
            .maybe_stop_target(self.stop_target)
            .notes(self.notes)
            .maybe_sends(sends)
            .lights(self.lights)
            .destination(self.destination)
            .message(self.message)
            .build())
    }
}

fn is_top(depth: &u8) -> bool {
    *depth == 0
}

fn is_manual(advance: &Advance) -> bool {
    *advance == Advance::Manual
}
