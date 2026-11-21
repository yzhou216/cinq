// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What surrounds the handling of a message: handing commands to the back
//! end, deciding what a message of any kind does to a command typed halfway,
//! and running blocking work off the interface thread

use std::thread;

use iced::{Task, futures::channel::oneshot};

use super::reveal_list::reveal_current_list;
use crate::{
    back::Command,
    front::{App, Message},
};

/// Returns whether `message` reports something that happened without the
/// operator, which changes nothing that the operator was in the middle of,
/// and moves no keyboard
pub(super) fn passive(message: &Message) -> bool {
    matches!(
        message,
        Message::Tick(_)
            | Message::Loaded(..)
            | Message::Recheck
            | Message::Rechecked(..)
            | Message::NetworkChecked
            | Message::LinkOpened(_)
            | Message::AudioDevicesListed(_)
            | Message::AudioDeviceKept(_)
            | Message::Saved(_)
            | Message::SearchSettled
            | Message::Resized(..)
            | Message::WindowFocused(_)
            | Message::Held(_)
            | Message::FileHeld(_)
            | Message::Landed { .. }
            | Message::Measured(_)
            | Message::Remeasure
            | Message::Drawn(_)
            | Message::Reached { .. }
    )
}

impl App {
    pub(super) fn with_task(
        self,
        task: impl FnOnce(&Self) -> Task<Message>,
    ) -> (Self, Task<Message>) {
        let task = task(&self);
        (self, task)
    }

    /// Hands a command to the back end, and brings the show as its file
    /// would hold it up to date after a command that can change the cues.
    /// The list on screen stays within the lists, whose number undo and
    /// bringing back unsaved changes can change.
    pub(super) fn send(self, command: Command) -> Self {
        let changes = command.edits()
            || matches!(
                command,
                Command::Undo | Command::Redo | Command::Collapse { .. }
            );
        let workspace = self.workspace.apply(command);
        let app = Self {
            list: self.list.min(workspace.lists().len().saturating_sub(1)),
            workspace,
            ..self
        };
        if changes { app.with_text() } else { app }
    }

    pub(super) fn send_all(self, commands: impl IntoIterator<Item = Command>) -> Self {
        commands.into_iter().fold(self, Self::send)
    }

    /// Applies a message, after anything other than typing drops what the
    /// operator left half done.  A command typed halfway goes, so that a stray
    /// `d` cannot wait through a click and an edit to delete a cue.  A level
    /// or the channels typed into a field apply once it is left or at any
    /// click, before the message acts, so that GO fires a cue at the level
    /// typed and no key after a click opens a device.  A link that failed to
    /// open is forgotten once its help page closes.
    pub(super) fn next(self, message: Message) -> (Self, Task<Message>) {
        let passive = passive(&message);
        let typing = passive || matches!(message, Message::Typed { .. });
        let clicked = matches!(message, Message::Clicked);
        // Typing into a field keeps its text, which the operator leaves with
        // anything else, Enter included
        let in_field = matches!(message, Message::SettingTyped(..));
        let kept = passive || clicked || in_field;
        // Field that the keyboard enters or leaves ends the run of edits to its
        // text, so that each visit makes a change of its own to undo, as each
        // stay in insert mode does in Vim
        let ends = matches!(message, Message::Focus(_) | Message::Clicked)
            || (matches!(message, Message::Held(false)) && self.held);
        let this = if ends {
            self.send(Command::EndRun)
        } else {
            self
        };
        // Field that the keyboard leaves takes its rejected text with it, so
        // that the status bar tells the reason, which the field no longer does
        let unapplied = (!kept).then(|| this.unapplied()).flatten();
        let app = if passive || in_field {
            this
        } else {
            this.settled()
        };
        let app = Self {
            keys: if typing { app.keys } else { Vec::new() },
            clipped: app.clipped.filter(|_| typing),
            remark: app.remark.filter(|_| typing),
            unapplied: app.unapplied.filter(|_| typing),
            typed: app.typed.filter(|_| kept),
            ..app
        };
        let shown = app.shown_list();
        let (app, task) = app.handled(message);
        let task = if app.shown_list() == shown {
            task
        } else {
            task.chain(reveal_current_list())
        };
        let app = Self {
            link_problem: app.link_problem.filter(|_| app.help),
            unapplied: unapplied.or(app.unapplied),
            ..app
        };
        (app, task)
    }
}

/// Runs blocking `work` on a thread of its own once iced runs the task: off
/// the interface thread, where GO and Panic must never wait, and off the
/// workers of iced's executor, which keep the clock ticking.  The task gives
/// what the work returns, or nothing when the program closes first.
pub(super) fn on_own_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Task<T> {
    Task::future(async move {
        let (sender, receiver) = oneshot::channel();
        thread::spawn(move || {
            // Receiver has gone only when the program is closing
            let _ = sender.send(work());
        });
        receiver.await.ok()
    })
    .and_then(Task::done)
}
