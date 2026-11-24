// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Actions that keys can be bound to

mod description;
mod message;

use strum::{EnumString, IntoStaticStr, VariantArray};

use crate::front::message::Page;
pub use description::Rows;

/// [`Action::VARIANTS`] lists every action in the order of the help page,
/// in the groups that a user of Vim scans: GO, Panic, and standby, then
/// moving, then editing, then the files and the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, IntoStaticStr, VariantArray)]
#[strum(serialize_all = "snake_case")]
pub enum Action {
    Go,
    Panic,
    Standby,
    Cancel,
    Next,
    Previous,
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
    Collapse,
    Expand,
    ToggleGroup,
    CollapseAll,
    ExpandAll,
    NextList,
    PreviousList,
    NextTab,
    PreviousTab,
    Back,
    Forward,
    Search,
    SearchBackward,
    NextMatch,
    PreviousMatch,
    Edit,
    EditStart,
    ChangeName,
    NextField,
    PreviousField,
    Hint,
    Pick,
    CloseMenu,
    AddBelow,
    AddAbove,
    Cut,
    Copy,
    PasteBelow,
    PasteAbove,
    Delete,
    MoveDown,
    MoveUp,
    Indent,
    Outdent,
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

    /// Actions that keep the show in hand wherever the keyboard is: their
    /// keys act alone, with any modifiers held, and over text fields, menus,
    /// and the help page alike
    pub(super) const AT_ONCE: [Action; 2] = [Action::Go, Action::Panic];

    /// Actions that apply to the rows that a motion reaches, as `d` and `y`
    /// do in Vim.  Each takes its operator key from a binding that doubles
    /// a key, such as `d d`.
    pub(super) const OPERATORS: [Action; 5] = [
        Action::Cut,
        Action::Copy,
        Action::Delete,
        Action::Indent,
        Action::Outdent,
    ];

    /// Actions whose keys are the motions that follow an operator, each
    /// reaching the rows from the selection to where it would move it
    pub(super) const MOTIONS: [Action; 4] =
        [Action::Next, Action::Previous, Action::First, Action::Last];

    pub fn acts_at_once(self) -> bool {
        Self::AT_ONCE.contains(&self)
    }

    /// Returns whether a key of the action acts on the cue list, which the
    /// keys of an open menu do not
    pub(super) fn acts_in_cue_list(self) -> bool {
        !matches!(self, Action::Pick | Action::CloseMenu)
    }

    /// Returns whether a key of the action acts while a menu is open, where
    /// every other key is kept from acting
    pub(super) fn acts_in_menu(self) -> bool {
        self.acts_at_once()
            || matches!(
                self,
                Action::Cancel
                    | Action::Next
                    | Action::Previous
                    | Action::PageDown
                    | Action::PageUp
                    | Action::First
                    | Action::Last
                    | Action::Help
                    | Action::Pick
                    | Action::CloseMenu
            )
    }

    /// Returns whether keys of the two actions act in the same place, the
    /// cue list or an open menu.  Only keys that do can hide each other, so
    /// that a key may pick in a menu and scroll the list.
    pub(super) fn shares_place(self, other: Action) -> bool {
        (self.acts_in_cue_list() && other.acts_in_cue_list())
            || (self.acts_in_menu() && other.acts_in_menu())
    }

    /// Describes the operator `self` applied over the rows that `motion`
    /// reaches, which are `rows`.  A `count` typed so far moves the motion
    /// by that many rows, or to that row for `First` and `Last`, as the
    /// operator will take it.
    pub fn over(self, motion: Action, rows: Rows, count: Option<usize>) -> String {
        let verb = match self {
            Action::Copy => "Copy",
            Action::Cut => "Cut",
            Action::Indent => "Indent",
            Action::Outdent => "Outdent",
            _ => "Delete",
        };
        let (one, many) = rows.nouns();
        let reached = match (motion, count) {
            (Action::Next, Some(count)) => format!("the selected {one} and the next {count}"),
            (Action::Previous, Some(count)) => {
                format!("the selected {one} and the previous {count}")
            }
            (Action::First | Action::Last, Some(count)) => {
                format!("the {many} from the selection to row {count}")
            }
            (Action::Next, _) => format!("the selected {one} and the next"),
            (Action::Previous, _) => format!("the selected {one} and the previous"),
            (Action::First, _) => format!("the {many} from the selection up to the first"),
            _ => format!("the {many} from the selection down to the last"),
        };
        format!("{verb} {reached}")
    }

    /// Returns whether a key of the action that is held down acts only
    /// once: those of [`Action::AT_ONCE`], so that holding the key of Panic
    /// does not cut a panic short, and those of fullscreen and hint mode,
    /// which would otherwise toggle the stage window or the labels back and
    /// forth
    pub fn acts_once_a_press(self) -> bool {
        self.acts_at_once() || matches!(self, Action::Fullscreen | Action::Hint)
    }

    /// Returns whether a key of the action only moves the selection or the
    /// list in view, which leaves the key to paste what a copy or cut took
    /// where it was, since pasting into the other list takes such moves first
    pub fn only_moves(self) -> bool {
        matches!(
            self,
            Action::Next
                | Action::Previous
                | Action::First
                | Action::Last
                | Action::HalfPageDown
                | Action::HalfPageUp
                | Action::PageDown
                | Action::PageUp
                | Action::ScrollDown
                | Action::ScrollUp
                | Action::TopRow
                | Action::MiddleRow
                | Action::BottomRow
                | Action::NextList
                | Action::PreviousList
                | Action::NextMatch
                | Action::PreviousMatch
                | Action::Back
                | Action::Forward
        )
    }

    /// Returns whether a key of the action acts while a closed drop-down
    /// list has the keyboard, beside those that act in every field.  The
    /// list has no text of its own, so that the keys of undo and redo act
    /// there, and so do those of [`Action::leaves_lists`].
    pub fn acts_in_lists(self) -> bool {
        self.acts_in_fields() || matches!(self, Action::Undo | Action::Redo) || self.leaves_lists()
    }

    /// Returns whether the action takes the keyboard out of a closed list, a
    /// checkbox, or a button that has it, so that the keys of hint mode, whose
    /// labels the list would swallow, and of the cue list, which a new cue
    /// selects, act next
    pub fn leaves_lists(self) -> bool {
        matches!(self, Action::Hint | Action::AddBelow | Action::AddAbove)
    }

    pub fn acts_in_fields(self) -> bool {
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
    /// `typed` handles before it asks.  The links of the page take labels.
    pub fn acts_over_help(self) -> bool {
        self.acts_at_once() || matches!(self, Action::Help | Action::CloseHelp | Action::Hint)
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
}
