// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The messages that act on the rows of a cue list

use super::Message;

impl Message {
    /// Returns whether the message moves the selection through the rows of
    /// the cue list, scrolls it, searches it, or edits it.  While the outputs
    /// show in place of the cue list, these must not reach it.
    pub fn acts_on_rows(&self) -> bool {
        matches!(
            self,
            Message::Step(_)
                | Message::Row(_)
                | Message::Page(..)
                | Message::Landed { .. }
                | Message::Scroll(_)
                | Message::InView(..)
                | Message::Reached { .. }
                | Message::Place(..)
                | Message::StandbySelected
                | Message::StepTab(_)
                | Message::ShowTab(_)
                | Message::Add(..)
                | Message::Move(_)
                | Message::Copy(_)
                | Message::Cut(_)
                | Message::Paste(..)
                | Message::Delete(_)
                | Message::Across(..)
                | Message::Search { .. }
                | Message::Searching(_)
                | Message::SearchKey(_)
                | Message::SearchAccepted
                | Message::SearchStep(_)
        )
    }
}
