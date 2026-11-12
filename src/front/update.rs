// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Update cycle, which applies each message to the interface state
//!
//! Only [`cinq`] reaches the parts with side effects.

mod cinq;
mod clipboard;
mod clock;
mod device;
mod dispatch;
mod editing;
mod focus_order;
mod history;
mod keyboard;
mod keys;
mod link;
mod loading;
mod navigation;
mod operators;
mod outputs;
mod paging;
mod reveal_focused;
mod scroll;
mod search;
mod select_focused;
mod settled;
mod show;
mod stage;
mod subscription;
#[cfg(test)]
mod tests;
mod viewing;

use iced::{Task, mouse};

use super::{App, Message};
use crate::back::Command;
pub(super) use clipboard::{Clipboard, Clipped};
use dispatch::on_own_thread;
pub(super) use history::{History, Spot};
use link::{edit_config, open_link, open_tutorial};
pub(super) use paging::kept::{Measured, Measures};
pub(super) use search::{Search, Searched};
pub(super) use show::{Keeper, Read, Reader, Saved, Show};
pub(super) use stage::Stage;

impl App {
    /// Applies a message that starts a task or changes what the rows show
    ///
    /// Any other message only changes state, which [`Self::settled_by`] does.
    fn handled(self, message: Message) -> (Self, Task<Message>) {
        match message {
            Message::Typed { press, repeat } => self.typed(press, repeat),
            // A key that moves the keyboard to another field ends the run of
            // edits to the field that it leaves, as a click there does
            Message::Focus(focus) => self.send(Command::EndRun).focused(focus),
            // While the outputs show, the keys that move through and change
            // cue rows act on outputs or on nothing, so that none of them
            // reaches the cue list out of sight
            message if self.outputs_shown && message.acts_on_rows() => self.in_outputs(message),
            // Selection scrolls into view as it or the rows around it move,
            // and as the main window changes size
            Message::Select(id) => self
                .without_search()
                .crossing(|app| app.showing_cues().with_selection(id))
                .revealed(),
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
            Message::SwitchList(list) => self
                .without_search()
                .jumping(|app| app.showing_cues().switched_to(list))
                .revealed(),
            Message::StepList(offset) => self.jumping(|app| app.stepped_list(offset)).revealed(),
            Message::Back(count) => self.walking(|app| app.went_back(count)),
            Message::Forward(count) => self.walking(|app| app.went_forward(count)),
            Message::Add(place, count) => self.with_new_cue(place, count).revealed(),
            Message::Move(by) => self.with_selected_moved(by).revealed(),
            Message::Cut(count) => self.cut(count).revealed(),
            Message::Paste(place, count) => self.with_pasted(place, count).revealed(),
            Message::Delete(count) => self.without_selected(count).revealed(),
            Message::Across(operator, motion, count) => {
                self.across(operator, motion, count).revealed()
            }
            Message::Search { backward } => self.searching(backward),
            Message::Searching(text) => self.search_typed(text),
            Message::SearchKey(key) => self.search_edited(key),
            Message::SearchAccepted => self.search_accepted(),
            Message::SearchStep(offset) => self.search_stepped(offset),
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
            Message::OpenTutorial => self.opening(open_tutorial),
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
            // Escape leaves a text field, so it gives up a search as well,
            // and the selection goes back to where the search began
            Message::Panic => {
                let searching = self.search.is_some();
                let app = Self {
                    help: false,
                    ..self.send(Command::Panic)
                }
                .search_cancelled();
                if searching {
                    app.revealed()
                } else {
                    (app, Task::none())
                }
            }
            // Keys typed so far went already, as for any message
            Message::Cancel => self.cancelled(),
            Message::ShowOutputs => self.showing_outputs().revealed(),
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
            message @ (Message::Nothing
            | Message::Drawn(_)
            | Message::SidePressed(..)
            | Message::Landed { .. }
            | Message::Measured(_)
            | Message::Reached { .. }
            | Message::MenuOpen(_)
            | Message::Held(_)
            | Message::FileHeld(_)
            | Message::Copy(_)
            | Message::PauseAll
            | Message::ResumeAll
            | Message::Stop(_)
            | Message::Tick(_)
            | Message::SearchSettled
            | Message::StandbySelected
            | Message::ShowMode(_)
            | Message::Loaded(..)
            | Message::Rechecked(..)
            | Message::Saved(_)
            | Message::Reconfigured(_)
            | Message::LinkOpened(_)
            | Message::AudioDevicesListed(_)
            | Message::AudioDeviceKept(_)
            | Message::SelectOutput(_)
            | Message::SettingTyped(..)
            | Message::OutputDevice(..)
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
}
