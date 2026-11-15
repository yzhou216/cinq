// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Messages that change the state and start no task

use super::{App, Measured, Message};
use crate::{
    back::{Command, Edit, parse_level},
    front::Tab,
};

impl App {
    /// Applies a message that only changes state
    ///
    /// A message that [`Self::handled`] takes up before it reaches here, such
    /// as a press of a mouse button that it moves by, falls through to the
    /// arms that leave the state as it is.
    pub(super) fn settled_by(self, message: Message) -> Self {
        match message {
            Message::Landed { list, from, row } if self.list == list && self.selected == from => {
                self.at_row(row.saturating_add(1))
            }
            Message::Measured(Measured::Cues {
                serial,
                list,
                cues,
                shape,
            }) => self.with_measure(serial, list, cues, shape),
            Message::Measured(Measured::Help(view)) => self.with_help_measure(view),
            Message::Reached { list, from, row } if self.list == list && self.selected == from => {
                self.jumping(|app| app.at_row(row.saturating_add(1)))
            }
            Message::Panic => Self {
                help: false,
                ..self.send(Command::Panic)
            },
            Message::PauseAll => self.send(Command::Pause(true)),
            Message::ResumeAll => self.send(Command::Pause(false)),
            Message::Stop(id) => self.send(Command::Stop(id)),
            Message::Tick(now) => self.ticked(now),
            // Nothing without a selected cue, as the toolbar's button does
            Message::StandbySelected => {
                let standby = self.selected_here().map(|cue| Command::Standby {
                    list: self.list,
                    cue: Some(cue),
                });
                self.send_all(standby)
            }
            Message::ShowMode(show_mode) => self.send(Command::ShowMode(show_mode)),
            Message::Loaded(show, found) => self.loaded(show, found),
            Message::Rechecked(show, found) => self.rechecked(show, found),
            Message::Saved(saved) => self.saved(saved),
            // Keys typed halfway went by the bindings before, so the update
            // drops them, as it does for most messages
            Message::Reconfigured(config) => Self { config, ..self },
            Message::LinkOpened(result) => Self {
                link_problem: result.err(),
                ..self
            },
            Message::AudioDevicesListed(listed) => self.with_audio_devices(listed),
            Message::AudioDeviceKept(kept) => Self {
                device_problem: kept.err(),
                ..self
            },
            Message::Clicked => self,
            // Stage window holds no cue list to keep in view
            Message::WindowFocused(window) => Self {
                stage: self.stage.with_keyboard_in(window),
                ..self
            },
            Message::Edit(id, edit) => self.send(Command::Edit(id, edit)),
            Message::LevelTyped(cue, text) => {
                let edit = parse_level(&text).map(|level| Command::Edit(cue, Edit::Level(level)));
                Self {
                    typed_level: Some((cue, text)),
                    ..self
                }
                .send_all(edit)
            }
            Message::FileChosen(id, path) => {
                let file = self.workspace.file_name_of(&path);
                let playing = self.workspace.cue(id).map(|cue| cue.playing(file));
                self.send_all(playing.map(|edit| Command::Edit(id, edit)))
            }
            Message::ToggleHelp => Self {
                help: !self.help,
                ..self
            },
            Message::CloseHelp => Self {
                help: false,
                ..self
            },
            Message::Tab(tab) => Self { tab, ..self },
            Message::ShowTab(number) => Self {
                tab: Tab::numbered(number),
                ..self
            },
            Message::StepTab(offset) => Self {
                tab: self.tab.stepped(offset),
                ..self
            },
            Message::NewKind(new_kind) => Self { new_kind, ..self },
            // Guarded arms above that do not match, and messages that need
            // no state change, leave the state as it is
            _ => self,
        }
    }
}
