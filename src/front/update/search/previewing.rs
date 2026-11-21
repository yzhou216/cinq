// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The groups that the preview of a search expands, which close again when
//! the match moves elsewhere or the search is cancelled, as Vim's
//! incremental search closes the folds that it opened

use super::Search;
use crate::{
    back::{Command, CueId},
    front::App,
};

impl App {
    /// Closes the groups that the preview of the search expanded, so that
    /// only those that hide the match of the moment stay open
    pub(super) fn search_reclosed(self) -> Self {
        let opened = self
            .search
            .as_ref()
            .map(|search| search.opened.clone())
            .unwrap_or_default();
        let app = if opened.is_empty() {
            self
        } else {
            self.send(Command::Collapse {
                cues: opened,
                collapsed: true,
            })
        };
        Self {
            search: app.search.map(|search| Search {
                opened: Vec::new(),
                ..search
            }),
            ..app
        }
    }

    /// Expands the collapsed groups that hide the selected match, and
    /// remembers them for the preview to close
    pub(super) fn search_previewed(self) -> Self {
        let hiding: Vec<CueId> = self
            .selected
            .filter(|_| !self.outputs_shown)
            .map(|id| self.current().groups().hiding(id))
            .unwrap_or_default();
        let app = self.with_selection_unhidden();
        Self {
            search: app.search.map(|search| Search {
                opened: hiding,
                ..search
            }),
            ..app
        }
    }
}
