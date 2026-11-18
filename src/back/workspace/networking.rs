// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rules of network cues, which send their message as they start
//!
//! A network cue that starts puts its message in the outbox of the
//! workspace, which the network output empties after each change, so that
//! the message goes out once for each run of the cue.

use std::collections::BTreeSet;

use super::Workspace;
use crate::back::{CueId, Destination, Kind, Outgoing, osc};

impl Workspace {
    /// Puts the message of network cue `id` in the outbox, unless the cue
    /// has no destination or message that could be read
    pub(super) fn sent_by(self, id: CueId) -> Self {
        let outgoing = self.cue(id).and_then(|cue| {
            Some(Outgoing {
                destination: Destination::parse(&cue.destination)?,
                packet: osc::packet(&cue.message)?,
            })
        });
        Self {
            outbox: self.outbox.into_iter().chain(outgoing).collect(),
            ..self
        }
    }

    /// Returns the destinations that the network cues name, as the network
    /// output keeps its problems by them
    pub fn destinations(&self) -> BTreeSet<String> {
        self.cues()
            .filter(|cue| cue.kind == Kind::Network)
            .filter_map(|cue| Destination::parse(&cue.destination))
            .map(|destination| destination.to_string())
            .collect()
    }

    /// Returns the workspace with an empty outbox, and the messages that the
    /// outbox held
    pub fn drained(self) -> (Self, Vec<Outgoing>) {
        let outbox = self.outbox;
        (
            Self {
                outbox: Vec::new(),
                ..self
            },
            outbox,
        )
    }
}
