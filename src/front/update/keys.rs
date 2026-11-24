// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keys and the keyboard focus
//!
//! Keys typed outside any text field go to the key bindings in
//! [`crate::front::keymap`], and the keys of a command typed halfway wait
//! in the interface state until the command is complete.  The inspector and
//! the toolbar keep the keys typed into their fields, apart from the keys
//! that act in fields.  iced moves the keyboard focus only on clicks, so the
//! keys that edit fields move it themselves.

use iced::{
    Task,
    advanced::widget::{operate, operation::focusable},
    keyboard,
    widget::operation::{self, AbsoluteOffset, RelativeOffset},
};

use super::{
    focus_order::move_focus, reveal_focused::reveal_focused, select_focused::select_focused,
};
use crate::front::{
    App, Caret, Focus, Message, Tab,
    keymap::{Action, KeyPress, Press},
    view,
};

/// What the status bar tells when show mode drops a key that changes the show
const LOCKED: &str = "Show mode does not allow edits";

/// What the status bar tells when the stage window has the keyboard, which
/// leaves the keys that change the show to the main window
const STAGE: &str = "Edits act in the main window";

impl App {
    /// Returns why a key of `action` does nothing now, when it is one that
    /// changes the show or names a cue.  Copying and undoing carry on in show
    /// mode, so that they stay unremarked there, and the help page covers the
    /// cues, so that no key is refused for them.
    fn refusal(&self, action: Action) -> Option<&'static str> {
        let changes = action.edits_the_show()
            || matches!(
                action,
                Action::Edit | Action::EditStart | Action::ChangeName
            );
        let locked = changes && !matches!(action, Action::Copy | Action::Undo | Action::Redo);
        match (self.help, self.stage.has_keyboard()) {
            (true, _) => None,
            (false, true) => changes.then_some(STAGE),
            (false, false) => (!self.editable() && locked).then_some(LOCKED),
        }
    }

    pub(super) fn typed(self, press: Press, repeat: bool) -> (Self, Task<Message>) {
        if self.hints.is_some() {
            return self.hint_typed(press, repeat);
        }
        let outcome = self.config.keymap.parse(&self.keys, press);
        let across = outcome.across();
        let app = Self {
            keys: outcome.pending,
            clipped: self.clipped.filter(|_| outcome.action.is_none()),
            remark: self.remark.filter(|_| outcome.action.is_none()),
            unapplied: self.unapplied.filter(|_| outcome.action.is_none()),
            ..self
        };
        if let Some((operator, motion, count)) = across {
            // Help page has no cues to operate on, and the stage window leaves
            // the cues to the main window
            let remark = app.refusal(operator);
            return if app.help || app.stage.has_keyboard() {
                (Self { remark, ..app }, Task::none())
            } else {
                let (app, task) = app.handled(Message::Across(operator, motion, count));
                (
                    Self {
                        remark: remark.or(app.remark),
                        ..app
                    },
                    task,
                )
            };
        }
        match outcome.action {
            Some((action, _)) if repeat && action.acts_once_a_press() => (app, Task::none()),
            // Cue list is out of sight from the stage window, so that the keys
            // that change the show or the clipboard are left to the main window,
            // and show mode locks the show, so that the status bar says why
            Some((action, _)) if app.refusal(action).is_some() => (
                Self {
                    remark: app.refusal(action),
                    ..app
                },
                Task::none(),
            ),
            // Page opens in the main window, which the fullscreen stage
            // window may cover, so that the page would open out of sight
            Some((Action::Help | Action::Hint, _)) if app.stage.has_keyboard() => {
                (app, Task::none())
            }
            Some((
                action @ (Action::Next | Action::Previous | Action::ScrollDown | Action::ScrollUp),
                count,
            )) if app.help => {
                let down = if matches!(action, Action::Next | Action::ScrollDown) {
                    1.0
                } else {
                    -1.0
                };
                let lines = count.map_or(1.0, |count| count as f32);
                let offset = AbsoluteOffset {
                    x: 0.0,
                    y: down * lines * view::line_height(),
                };
                (app, operation::scroll_by(view::HELP, offset))
            }
            Some((Action::First, _)) if app.help => {
                (app, operation::snap_to(view::HELP, RelativeOffset::START))
            }
            Some((Action::Last, _)) if app.help => {
                (app, operation::snap_to(view::HELP, RelativeOffset::END))
            }
            // The page has no focus for links, so that its keys reach the
            // tutorial, which a held key would open again on every repeat
            Some((Action::Standby, _)) if app.help && repeat => (app, Task::none()),
            Some((Action::Standby, _)) if app.help => app.handled(Message::OpenTutorial),
            Some((action, count)) if app.help && !action.acts_over_help() => match action.page() {
                Some(page) => app.help_paged(page, count),
                None => (app, Task::none()),
            },
            Some((action, count)) if !app.help || action.acts_over_help() => {
                app.handled(action.message(count))
            }
            _ => (app, Task::none()),
        }
    }

    /// Gives up the search line, with the selection going back to where the
    /// search began, and takes the keyboard out of the field that has it,
    /// as Control+c does in Vim.  Playback goes on, since it is no Panic.
    pub(super) fn cancelled(self) -> (Self, Task<Message>) {
        let (app, reveal) = if self.search.is_some() {
            self.search_cancelled().revealed()
        } else {
            (self, Task::none())
        };
        (app, Task::batch([reveal, operate(focusable::unfocus())]))
    }

    /// Editing the name shows the Basics tab, which holds the Name field, or
    /// the Name field of the output while the outputs show.  No field takes
    /// the keyboard while the stage window has it, since the keys typed there
    /// would never reach the field.
    pub(super) fn focused(self, focus: Focus) -> (Self, Task<Message>) {
        match focus {
            Focus::Name(_) | Focus::Next | Focus::Previous if self.stage.has_keyboard() => {
                (self, Task::none())
            }
            Focus::Name(caret)
                if self.outputs_shown && self.editable() && self.output.is_some() =>
            {
                (self, in_field(view::OUTPUT_NAME, caret))
            }
            Focus::Name(_) if self.outputs_shown => (self, Task::none()),
            Focus::Name(caret) if self.editable() && self.selected.is_some() => (
                Self {
                    tab: Tab::Basics,
                    ..self
                },
                in_field(view::NAME, caret),
            ),
            Focus::Name(_) => (self, Task::none()),
            Focus::Next => (self.without_search(), reached(move_focus(false))),
            Focus::Previous => (self.without_search(), reached(move_focus(true))),
            Focus::Cues => (self, operate(focusable::unfocus())),
        }
    }
}

/// Reads a key that no widget captured, with Shift applied, so that Shift
/// and `g` make `G`
pub(super) fn key_press(event: keyboard::Event) -> Option<Message> {
    let KeyPress { press, repeat, .. } = KeyPress::of(&event)?;
    press.map(|press| Message::Typed { press, repeat })
}

/// Selects the text of the field that a move by key reached, so that typing
/// replaces it as in desktop programs, while a click keeps the caret where
/// it fell, and scrolls the sends or the sidebar to show the field if it is
/// in one of them
pub(super) fn reached(moved: Task<Message>) -> Task<Message> {
    moved.chain(select_focused()).chain(reveal_focused())
}

/// Focuses the text field `id`, with the caret where the operator asked for
/// it
fn in_field(id: &'static str, caret: Caret) -> Task<Message> {
    match caret {
        // Focusing puts the caret at the end, as Vim's `A` does
        Caret::End => operation::focus(id),
        Caret::Start => operation::focus(id).chain(operation::move_cursor_to_front(id)),
        Caret::All => operation::focus(id).chain(operation::select_all(id)),
    }
}
