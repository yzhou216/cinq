// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Outputs of the show, which the main window shows in place of a cue list,
//! and the settings that apply once the text typed into them reads as a
//! value
//!
//! The keys that move through and edit cues move through and edit the
//! outputs while they show, so that adding, deleting, moving, and renaming
//! an output takes the keys of a cue.

use iced::Task;

use crate::{
    back::{Channels, Command, CueId, Edit, OutputEdit, OutputId, Place, parse_level},
    front::{App, Message, Setting, step},
};

impl App {
    /// Shows the outputs in place of the cue list, closing a search line,
    /// which would otherwise search the list out of sight
    pub(super) fn showing_outputs(self) -> Self {
        let output = self
            .output
            .filter(|id| self.workspace.output(*id).is_some())
            .or_else(|| self.workspace.outputs().first().map(|output| output.id));
        Self {
            outputs_shown: true,
            output,
            ..self.without_search()
        }
    }

    /// Shows the cue list again in place of the outputs
    pub(super) fn showing_cues(self) -> Self {
        Self {
            outputs_shown: false,
            ..self
        }
    }

    /// Applies a message that acts on the rows of the cue list while the
    /// outputs show: the keys that select, add, move, and delete rows do the
    /// same to outputs, which scroll into view, and every other such
    /// message does nothing, since the cue list is out of sight
    pub(super) fn in_outputs(self, message: Message) -> (Self, Task<Message>) {
        let app = match message {
            Message::Step(offset) => self.stepped_output(offset),
            Message::Row(row) => self.output_at_row(row),
            Message::Add(place, _) => {
                self.with_new_output(matches!(place(CueId(0)), Place::Before(_)))
            }
            Message::Move(by) => self.with_output_moved(by),
            Message::Delete(_) => self.without_output(),
            _ => return (self, Task::none()),
        };
        app.revealed()
    }

    /// Returns the place of the output that the inspector shows
    pub(in crate::front) fn output_row(&self) -> Option<usize> {
        let outputs = self.workspace.outputs();
        self.output
            .and_then(|id| outputs.iter().position(|output| output.id == id))
    }

    fn with_output_at(self, row: Option<usize>) -> Self {
        let output = row
            .and_then(|row| self.workspace.outputs().get(row))
            .map(|output| output.id);
        Self { output, ..self }
    }

    pub(super) fn stepped_output(self, offset: isize) -> Self {
        let row = step(self.output_row(), offset, self.workspace.outputs().len());
        self.with_output_at(row)
    }

    /// Selects the output on a row counted from one, or the last one past
    /// the end, as the keys of the first and last cue do
    pub(super) fn output_at_row(self, row: usize) -> Self {
        let last = self.workspace.outputs().len().saturating_sub(1);
        self.with_output_at(Some(row.saturating_sub(1).min(last)))
    }

    /// Adds an output below the one shown, or above it, and shows the new
    /// one
    pub(super) fn with_new_output(self, above: bool) -> Self {
        // Show mode adds nothing, so the selection stays where it is
        if !self.editable() {
            return self;
        }
        let at = self
            .output_row()
            .map_or(usize::MAX, |row| if above { row } else { row + 1 });
        let app = self.send(Command::AddOutput { at });
        let row = at.min(app.workspace.outputs().len().saturating_sub(1));
        app.with_output_at(Some(row))
    }

    pub(super) fn with_output_moved(self, by: isize) -> Self {
        match self.output {
            Some(output) => self.send(Command::MoveOutput { output, by }),
            None => self,
        }
    }

    /// Deletes the output shown and shows the one that takes its place
    pub(super) fn without_output(self) -> Self {
        let (Some(output), row) = (self.output, self.output_row()) else {
            return self;
        };
        let app = self.send(Command::DeleteOutput(output));
        let last = app.workspace.outputs().len().saturating_sub(1);
        app.with_output_at(row.map(|row| row.min(last)))
    }

    /// Applies the channels typed into the field of an output once the
    /// operator has left it.  They wait until then, since a device would
    /// open for each of `1`, `12`, and `12-13` on the way to the last.  The
    /// other settings apply as they read, for they cost no device.
    pub(super) fn settled(self) -> Self {
        let command = match &self.typed {
            Some((Setting::Channels(id), text)) => Channels::parse(text)
                .filter(|channels| {
                    self.workspace
                        .output(*id)
                        .is_some_and(|output| output.channels != *channels)
                })
                .map(|channels| edit_output(*id, OutputEdit::Channels(channels))),
            _ => None,
        };
        Self {
            typed: self
                .typed
                .filter(|(setting, _)| !matches!(setting, Setting::Channels(_))),
            ..self
        }
        .send_all(command)
    }

    /// Settles the text of the field that the operator was in, unless it is
    /// the field of `setting`
    pub(super) fn settled_but(self, setting: Setting) -> Self {
        match &self.typed {
            Some((typed, _)) if *typed != setting => self.settled(),
            _ => self,
        }
    }

    /// Applies what has been typed into the field of `setting` once it reads
    /// as a value, and keeps the text for the field to show meanwhile
    pub(super) fn typed_setting(self, setting: Setting, text: String) -> Self {
        // Level that the text gives, none for an empty field, or no level
        // at all for text that is no level yet
        let level = || match text.trim() {
            "" => Some(None),
            typed => parse_level(typed).map(Some),
        };
        let command = match setting {
            Setting::Send(cue, output) => level().and_then(|level| {
                // Start from the sends as the cue holds them, which keep the
                // sends to outputs that have gone for an undo to bring back
                let cue = self.workspace.cue(cue)?;
                let sends = cue
                    .sends
                    .clone()
                    .unwrap_or_else(|| self.workspace.sends(cue))
                    .into_iter()
                    .filter(|(id, _)| *id != output)
                    .chain(level.map(|level| (output, level)))
                    .collect();
                Some(Command::Edit(cue.id, Edit::Sends(Some(sends))))
            }),
            Setting::OutputName(id) => Some(edit_output(id, OutputEdit::Name(text.clone()))),
            Setting::Channels(_) => None,
            Setting::NewCues(id) => {
                level().map(|level| edit_output(id, OutputEdit::NewCues(level)))
            }
        };
        Self {
            typed: Some((setting, text)),
            ..self
        }
        .send_all(command)
    }
}

fn edit_output(id: OutputId, edit: OutputEdit) -> Command {
    Command::EditOutput(id, edit)
}
