// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Commands that arrive over OSC, which do what the keys of GO and Panic and
//! the buttons of the transport do

use std::net::IpAddr;

use iced::{
    Task,
    advanced::widget::{operate, operation::focusable},
};

use super::App;
use crate::{
    back::{Command, CueId, Remote},
    front::Message,
    warn,
};

/// Most characters of a cue number that a warning holds, since anyone on the
/// network can send it
const SHOWN: usize = 64;

impl App {
    /// Carries out a command from the network.  It leaves hint mode as a
    /// click does, and acts in show mode, while a field has the keyboard,
    /// and over the help page, as GO and Panic do, since none of the
    /// commands edits the show.  A command that moves the selection or the
    /// list in view takes the keyboard out of a text field, as going back does,
    /// so that what is typed next cannot edit another cue.  A cue number that
    /// no cue has does nothing to the show, and to the keys typed halfway,
    /// and tells where it came from and what it named, on standard error once
    /// and in the sidebar until a later command acts.
    pub(super) fn remote(self, from: IpAddr, remote: Remote) -> (Self, Task<Message>) {
        if let Some(number) = self.unnumbered(&remote) {
            let shown: String = number.chars().take(SHOWN).collect();
            let line = format!("OSC from {from}: no cue is numbered {shown:?}");
            warn::once(&line);
            return (self.with_ignored(line), Task::none());
        }
        let here = (self.selected, self.list);
        let (app, task) = Self {
            osc_ignored: None,
            ..self
        }
        .carried_out(remote);
        if (app.selected, app.list) == here {
            (app, task)
        } else {
            (app, Task::batch([task, operate(focusable::unfocus())]))
        }
    }

    /// Returns the state with `line` as the command that did nothing.  A first
    /// line moves what lies below it in the sidebar, so that it leaves hint mode.
    pub(super) fn with_ignored(self, line: String) -> Self {
        let app = if self.osc_ignored.is_none() {
            self.without_hints()
        } else {
            self
        };
        Self {
            osc_ignored: Some(line),
            ..app
        }
    }

    /// Returns the number of the cue that `remote` aims at, if no cue has it,
    /// which makes the command change nothing at all, the interface included
    pub(super) fn unnumbered<'a>(&self, remote: &'a Remote) -> Option<&'a str> {
        remote
            .number()
            .filter(|number| self.workspace.numbered(number).is_none())
    }

    fn carried_out(self, remote: Remote) -> (Self, Task<Message>) {
        let app = self.without_hints();
        match remote {
            Remote::Go => app.handled(Message::Go),
            Remote::Panic => app.handled(Message::Panic),
            Remote::Pause => (app.send(Command::Pause(true)), Task::none()),
            Remote::Resume => (app.send(Command::Pause(false)), Task::none()),
            Remote::Start(number) => app.with_numbered(&number, Self::started),
            Remote::Stop(number) => app.with_numbered(&number, |app, id| {
                (app.send(Command::Stop(id)), Task::none())
            }),
            Remote::PauseCue(number) => app.with_numbered(&number, |app, id| {
                (app.send(Command::Hold(id)), Task::none())
            }),
            Remote::ResumeCue(number) => app.with_numbered(&number, |app, id| {
                (app.send(Command::Resume(id)), Task::none())
            }),
            Remote::Playhead(number) => {
                app.with_numbered(&number, |app, id| app.send(Command::GoTo(id)).picked(id))
            }
        }
    }

    fn with_numbered(
        self,
        number: &str,
        act: impl FnOnce(Self, CueId) -> (Self, Task<Message>),
    ) -> (Self, Task<Message>) {
        match self.workspace.numbered(number).map(|cue| cue.id) {
            Some(id) => act(self, id),
            // Unreachable, since [`Self::remote`] looked the number up
            None => (self, Task::none()),
        }
    }

    /// Fires a cue as a start cue does, and cues the stage window as GO does
    /// when a cue that it fires shows on stage
    fn started(self, id: CueId) -> (Self, Task<Message>) {
        let staged = self
            .workspace
            .firing(id)
            .iter()
            .any(|(cue, _)| cue.kind.on_stage());
        let app = self.send(Command::Start(id));
        let stage = if staged { app.stage.cued() } else { app.stage };
        (Self { stage, ..app }, Task::none())
    }
}
