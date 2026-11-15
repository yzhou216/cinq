// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Actions that keys can be bound to

use strum::{EnumString, IntoStaticStr, VariantArray};

use crate::{
    back::Place,
    front::{
        Focus, Message,
        message::{Height, Page},
    },
};

/// [`Action::VARIANTS`] lists every action in the order of the help page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, IntoStaticStr, VariantArray)]
#[strum(serialize_all = "snake_case")]
pub enum Action {
    Go,
    Panic,
    Next,
    Previous,
    NextList,
    PreviousList,
    First,
    Last,
    HalfPageDown,
    HalfPageUp,
    PageDown,
    PageUp,
    ScrollDown,
    ScrollUp,
    TopRow,
    MiddleRow,
    BottomRow,
    RowToMiddle,
    RowToTop,
    RowToBottom,
    NextTab,
    PreviousTab,
    Back,
    Forward,
    Standby,
    Cancel,
    Edit,
    NextField,
    PreviousField,
    AddBelow,
    AddAbove,
    MoveDown,
    MoveUp,
    Delete,
    Copy,
    Cut,
    PasteBelow,
    PasteAbove,
    Undo,
    Redo,
    Open,
    Save,
    SaveAs,
    Fullscreen,
    Help,
    CloseHelp,
}

impl Action {
    pub fn name(self) -> &'static str {
        self.into()
    }

    pub fn description(self) -> &'static str {
        match self {
            Action::Go => "GO: fire the standby cue and its chain",
            Action::Panic => "Panic: fade everything out, or stop it at once",
            Action::Next => "Select the next cue or menu option",
            Action::Previous => "Select the previous cue or menu option",
            Action::NextList => "Show the next cue list",
            Action::PreviousList => "Show the previous cue list",
            Action::First => "Select the first cue, or the row counted",
            Action::Last => "Select the last cue, or the row counted",
            Action::HalfPageDown => {
                "Scroll half a page down with the selection, or the rows counted"
            }
            Action::HalfPageUp => "Scroll half a page up with the selection, or the rows counted",
            Action::PageDown => "Scroll a page down with the selection, or the pages counted",
            Action::PageUp => "Scroll a page up with the selection, or the pages counted",
            Action::ScrollDown => "Scroll the cue list a row down, or the rows counted",
            Action::ScrollUp => "Scroll the cue list a row up, or the rows counted",
            Action::TopRow => "Select the first row in view, or the row counted from it",
            Action::MiddleRow => "Select the middle row in view",
            Action::BottomRow => "Select the last row in view, or the row counted up from it",
            Action::RowToMiddle => {
                "Scroll the cue list to put the selected row, or the row counted, in the middle"
            }
            Action::RowToTop => {
                "Scroll the cue list to put the selected row, or the row counted, at the top"
            }
            Action::RowToBottom => {
                "Scroll the cue list to put the selected row, or the row counted, at the bottom"
            }
            Action::NextTab => "Show the next inspector tab, or the tab counted",
            Action::PreviousTab => "Show the previous inspector tab, or go back as many as counted",
            Action::Back => "Go back to the place that the last jump left",
            Action::Forward => "Go forward again after going back",
            Action::Standby => "Put the selected cue on standby",
            Action::Cancel => "Drop the keys typed so far and leave a field",
            Action::Edit => "Edit the name of the selected cue",
            Action::NextField => "Move to the next field",
            Action::PreviousField => "Move to the previous field",
            Action::AddBelow => "Add a cue below the selection",
            Action::AddAbove => "Add a cue above the selection",
            Action::MoveDown => "Move the selected cue down",
            Action::MoveUp => "Move the selected cue up",
            Action::Delete => "Delete the selected cue without copying it",
            Action::Copy => "Copy the selected cue to the clipboard",
            Action::Cut => "Cut the selected cue to the clipboard",
            Action::PasteBelow => "Paste the copied cues below the selection",
            Action::PasteAbove => "Paste the copied cues above the selection",
            Action::Undo => "Undo the latest change to the show, or as many as counted",
            Action::Redo => "Redo the change undone last, or as many as counted",
            Action::Open => "Open a show",
            Action::Save => "Save the show",
            Action::SaveAs => "Save the show in another file",
            Action::Fullscreen => "Toggle fullscreen on the stage window",
            Action::Help => "Show or hide this help",
            Action::CloseHelp => "Hide this help",
        }
    }

    /// Actions that keep the show in hand wherever the keyboard is: their
    /// keys act alone, with any modifiers held, and over text fields, menus,
    /// and the help page alike
    pub(super) const AT_ONCE: [Action; 2] = [Action::Go, Action::Panic];

    /// Actions that apply to the rows that a motion reaches, as `d` and `y`
    /// do in Vim.  Each takes its operator key from a binding that doubles
    /// a key, such as `d d`.
    pub(super) const OPERATORS: [Action; 3] = [Action::Cut, Action::Copy, Action::Delete];

    /// Actions whose keys are the motions that follow an operator, each
    /// reaching the rows from the selection to where it would move it
    pub(super) const MOTIONS: [Action; 4] =
        [Action::Next, Action::Previous, Action::First, Action::Last];

    pub fn acts_at_once(self) -> bool {
        Self::AT_ONCE.contains(&self)
    }

    /// Describes the operator `self` applied over the rows that `motion`
    /// reaches
    pub fn over(self, motion: Action) -> String {
        let verb = match self {
            Action::Copy => "Copy",
            Action::Cut => "Cut",
            _ => "Delete",
        };
        let rows = match motion {
            Action::Next => "the selected cue and the next",
            Action::Previous => "the selected cue and the previous",
            Action::First => "the cues from the selection up to the first",
            _ => "the cues from the selection down to the last",
        };
        format!("{verb} {rows}")
    }

    /// Returns whether a key of the action that is held down acts only
    /// once: those of [`Action::AT_ONCE`], so that holding the key of Panic
    /// does not cut a panic short, and that of fullscreen, which would
    /// otherwise toggle the stage window back and forth
    pub fn acts_once_a_press(self) -> bool {
        self.acts_at_once() || self == Action::Fullscreen
    }

    /// Returns whether a key of the action acts while a closed drop-down
    /// list has the keyboard, beside those that act in every field
    pub(super) fn acts_in_lists(self) -> bool {
        self.acts_in_fields() || matches!(self, Action::Undo | Action::Redo)
    }

    pub(super) fn acts_in_fields(self) -> bool {
        self.acts_at_once()
            || matches!(
                self,
                Action::Cancel
                    | Action::NextField
                    | Action::PreviousField
                    | Action::Back
                    | Action::Forward
                    | Action::Open
                    | Action::Save
                    | Action::SaveAs
            )
    }

    /// Returns whether a key of the action acts while the help page is
    /// open.  The keys that scroll the page are the page's own, which
    /// `typed` handles before it asks.
    pub fn acts_over_help(self) -> bool {
        self.acts_at_once() || matches!(self, Action::Help | Action::CloseHelp)
    }

    /// Returns the page that the action moves by, if it is one of the four
    /// that do
    pub fn page(self) -> Option<Page> {
        match self {
            Action::HalfPageDown => Some(Page::HalfDown),
            Action::HalfPageUp => Some(Page::HalfUp),
            Action::PageDown => Some(Page::WholeDown),
            Action::PageUp => Some(Page::WholeUp),
            _ => None,
        }
    }

    pub fn message(self, count: Option<usize>) -> Message {
        let times = count.unwrap_or(1);
        let step = isize::try_from(times).unwrap_or(isize::MAX);
        match self {
            Action::Go => Message::Go,
            Action::Panic => Message::Panic,
            Action::Next => Message::Step(step),
            Action::Previous => Message::Step(-step),
            Action::NextList => Message::StepList(step),
            Action::PreviousList => Message::StepList(-step),
            Action::First => Message::Row(times),
            Action::Last => Message::Row(count.unwrap_or(usize::MAX)),
            Action::HalfPageDown => Message::Page(Page::HalfDown, count),
            Action::HalfPageUp => Message::Page(Page::HalfUp, count),
            Action::PageDown => Message::Page(Page::WholeDown, count),
            Action::PageUp => Message::Page(Page::WholeUp, count),
            Action::ScrollDown => Message::Scroll(step),
            Action::ScrollUp => Message::Scroll(-step),
            Action::TopRow => Message::InView(Height::Top, count),
            Action::MiddleRow => Message::InView(Height::Middle, count),
            Action::BottomRow => Message::InView(Height::Bottom, count),
            Action::RowToMiddle => Message::Place(Height::Middle, count),
            Action::RowToTop => Message::Place(Height::Top, count),
            Action::RowToBottom => Message::Place(Height::Bottom, count),
            Action::NextTab => count.map_or(Message::StepTab(1), Message::ShowTab),
            Action::PreviousTab => Message::StepTab(-step),
            Action::Back => Message::Back(times),
            Action::Forward => Message::Forward(times),
            Action::Standby => Message::StandbySelected,
            Action::Cancel => Message::Cancel,
            Action::Edit => Message::Focus(Focus::Name),
            Action::NextField => Message::Focus(Focus::Next),
            Action::PreviousField => Message::Focus(Focus::Previous),
            Action::AddBelow => Message::Add(Place::After, times),
            Action::AddAbove => Message::Add(Place::Before, times),
            Action::MoveDown => Message::Move(step),
            Action::MoveUp => Message::Move(-step),
            Action::Delete => Message::Delete(times),
            Action::Copy => Message::Copy(times),
            Action::Cut => Message::Cut(times),
            Action::PasteBelow => Message::Paste(Place::After, times),
            Action::PasteAbove => Message::Paste(Place::Before, times),
            Action::Undo => Message::Undo(times),
            Action::Redo => Message::Redo(times),
            Action::Open => Message::Open,
            Action::Save => Message::Save,
            Action::SaveAs => Message::SaveAs,
            Action::Fullscreen => Message::ToggleFullscreen,
            Action::Help => Message::ToggleHelp,
            Action::CloseHelp => Message::CloseHelp,
        }
    }

    /// Returns whether the action edits the show or fills the clipboard, which
    /// the stage window leaves to the main window: the operator cannot see the
    /// cue list from there, so a key held down by mistake would change cues
    /// out of sight
    pub fn edits_the_show(self) -> bool {
        matches!(
            self,
            Action::AddBelow
                | Action::AddAbove
                | Action::Cut
                | Action::Copy
                | Action::PasteBelow
                | Action::PasteAbove
                | Action::Delete
                | Action::MoveDown
                | Action::MoveUp
                | Action::Undo
                | Action::Redo
        )
    }
}
