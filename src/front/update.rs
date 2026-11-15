// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Update cycle, which applies each message to the interface state
//!
//! Only [`cinq`] reaches the parts with side effects.

mod cinq;
mod clock;
mod device;
mod editing;
mod history;
mod keys;
mod link;
mod loading;
mod navigation;
mod paging;
mod scroll;
mod settled;
mod show;
mod stage;
mod subscription;
#[cfg(test)]
mod tests;
mod viewing;

use std::thread;

use iced::{Task, futures::channel::oneshot, mouse};

use super::{App, Message};
use crate::back::Command;
pub(super) use history::{History, Spot};
use link::{edit_config, open_link};
pub(super) use paging::kept::{Measured, Measures};
pub(super) use show::{Keeper, Read, Reader, Saved, Show};
pub(super) use stage::Stage;

impl App {
    fn with_task(self, task: impl FnOnce(&Self) -> Task<Message>) -> (Self, Task<Message>) {
        let task = task(&self);
        (self, task)
    }

    /// Applies a message that starts a task or changes what the rows show
    ///
    /// Any other message only changes state, which [`Self::settled_by`] does.
    fn handled(self, message: Message) -> (Self, Task<Message>) {
        match message {
            Message::Typed { press, repeat } => self.typed(press, repeat),
            Message::Focus(focus) => self.focused(focus),
            // Selection scrolls into view as it or the rows around it move,
            // and as the main window changes size
            Message::Select(id) => self.crossing(|app| app.with_selection(id)).revealed(),
            Message::Step(offset) => self.stepped(offset).revealed(),
            Message::Row(row) => self.jumping(|app| app.at_row(row)).revealed(),
            Message::Page(page, count) => self.paged(page, count),
            Message::Drawn(window) if self.main == Some(window) && self.unmeasured() => {
                let measure = self.measure_cues();
                (self, measure)
            }
            Message::Remeasure => {
                let app = self.remeasured();
                let measure = app.measure_cues();
                (app, measure)
            }
            Message::Scroll(rows) => self.scrolled(rows),
            Message::InView(height, count) => self.in_view(height, count),
            Message::Place(height, count) => self.placed(height, count),
            Message::SwitchList(list) => self.jumping(|app| app.switched_to(list)).revealed(),
            Message::StepList(offset) => self.jumping(|app| app.stepped_list(offset)).revealed(),
            Message::Back(count) => self.walking(|app| app.went_back(count)),
            Message::Forward(count) => self.walking(|app| app.went_forward(count)),
            Message::Add(place) => self.with_new_cue(place).revealed(),
            Message::Move(by) => self.with_selected_moved(by).revealed(),
            Message::Delete(count) => self.without_selected(count).revealed(),
            Message::Undo(count) => self.undone(count).revealed(),
            Message::Redo(count) => self.redone(count).revealed(),
            Message::Resized(window, size) => self.resized(window, size),
            Message::Recheck => self.recheck(),
            Message::Open => self.with_task(Self::choose_show),
            Message::Opened(read) => self.opened(*read).revealed(),
            Message::Save => self.with_task(Self::save),
            Message::SaveAs => self.with_task(Self::choose_file),
            Message::SaveTo(file) => self.with_task(|app| app.save_to(file)),
            Message::ChooseFile(id) => self.with_task(|app| app.choose_cue_file(id)),
            Message::OpenLink(target) => self.opening(move || open_link(&target)),
            Message::EditConfig(file) => self.opening(move || edit_config(&file)),
            Message::ToggleFullscreen => self.with_task(Self::toggle_fullscreen),
            Message::Closed(window) => self.closed(window),
            Message::ListAudioDevices => self.with_task(|_| device::listing()),
            Message::ChooseAudioDevice(device) => self.chose_audio_device(device),
            // GO and Panic close the help page, so that the show is in view
            // as it runs or stops, and a selection that GO moves scrolls
            // into view, while the list stays put otherwise
            Message::Go => {
                let selected = self.selected;
                let gone = self.gone();
                if gone.selected == selected {
                    (gone, Task::none())
                } else {
                    gone.revealed()
                }
            }
            // Keys typed so far went already, as for any message
            Message::Cancel => self.cancelled(),
            // Buttons of the mouse go back and forward from the main window, as
            // the keys do, apart from the help page, which the keys leave be
            Message::SidePressed(window, mouse::Button::Back)
                if self.main == Some(window) && !self.help =>
            {
                self.walking(|app| app.went_back(1))
            }
            Message::SidePressed(window, mouse::Button::Forward)
                if self.main == Some(window) && !self.help =>
            {
                self.walking(|app| app.went_forward(1))
            }
            // Naming each variant here, with no wildcard, keeps the compiler
            // checking that a new message is handled by one part or the other
            message @ (Message::Drawn(_)
            | Message::SidePressed(..)
            | Message::Landed { .. }
            | Message::Measured(_)
            | Message::Reached { .. }
            | Message::Panic
            | Message::PauseAll
            | Message::ResumeAll
            | Message::Stop(_)
            | Message::Tick(_)
            | Message::StandbySelected
            | Message::ShowMode(_)
            | Message::Loaded(..)
            | Message::Rechecked(..)
            | Message::Saved(_)
            | Message::Reconfigured(_)
            | Message::LinkOpened(_)
            | Message::AudioDevicesListed(_)
            | Message::AudioDeviceKept(_)
            | Message::Clicked
            | Message::WindowFocused(_)
            | Message::Edit(..)
            | Message::LevelTyped(..)
            | Message::FileChosen(..)
            | Message::ToggleHelp
            | Message::CloseHelp
            | Message::Tab(_)
            | Message::ShowTab(_)
            | Message::StepTab(_)
            | Message::NewKind(_)) => (self.settled_by(message), Task::none()),
        }
    }

    /// Hands a command to the back end, and brings the show as its file
    /// would hold it up to date after a command that can change the cues.
    /// The list on screen stays within the lists, whose number undo and
    /// bringing back unsaved changes can change.
    fn send(self, command: Command) -> Self {
        let changes = command.edits() || matches!(command, Command::Undo | Command::Redo);
        let workspace = self.workspace.apply(command);
        let app = Self {
            list: self.list.min(workspace.lists().len().saturating_sub(1)),
            workspace,
            ..self
        };
        if changes { app.with_text() } else { app }
    }

    fn send_all(self, commands: impl IntoIterator<Item = Command>) -> Self {
        commands.into_iter().fold(self, Self::send)
    }

    /// Anything the operator does other than typing drops a command typed
    /// halfway, so that a stray `d` cannot wait through a click and an edit
    /// to delete a cue.  A level typed so far gives way to the level of its
    /// cue as well, apart from a click, which can land in its own field.  A
    /// link that failed to open is forgotten once the help page that holds
    /// it closes.
    fn next(self, message: Message) -> (Self, Task<Message>) {
        let passive = matches!(
            message,
            Message::Tick(_)
                | Message::Loaded(..)
                | Message::Recheck
                | Message::Rechecked(..)
                | Message::LinkOpened(_)
                | Message::AudioDevicesListed(_)
                | Message::AudioDeviceKept(_)
                | Message::Saved(_)
                | Message::Resized(..)
                | Message::WindowFocused(_)
                | Message::Landed { .. }
                | Message::Measured(_)
                | Message::Remeasure
                | Message::Drawn(_)
                | Message::Reached { .. }
        );
        let typing = passive || matches!(message, Message::Typed { .. });
        let clicked = matches!(message, Message::Clicked);
        let app = Self {
            keys: if typing { self.keys } else { Vec::new() },
            typed_level: self.typed_level.filter(|_| passive || clicked),
            ..self
        };
        let (app, task) = app.handled(message);
        let app = Self {
            link_problem: app.link_problem.filter(|_| app.help),
            ..app
        };
        (app, task)
    }
}

/// Runs blocking `work` on a thread of its own once iced runs the task: off
/// the interface thread, where GO and Panic must never wait, and off the
/// workers of iced's executor, which keep the clock ticking.  The task gives
/// what the work returns, or nothing when the program closes first.
fn on_own_thread<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Task<T> {
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
