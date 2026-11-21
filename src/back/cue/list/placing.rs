// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The depth that a cue takes where it is added, pasted, or moved to
//!
//! Wherever a cue lands, it takes the depth that a cue added there would.
//! A cue below a row takes the deepest depth that the row allows, which is
//! one level deeper than the row where that is a group, so that a cue added
//! below a group is the first cue in it.  A cue above a row takes the depth
//! of that row.

use super::CueList;
use crate::back::{Cue, Kind, Place};

impl CueList {
    /// Returns the depth that a cue put at `at` takes
    pub(in crate::back) fn depth_at(&self, at: Place) -> u8 {
        let before = match at {
            Place::Before(cue) => self.position(cue).map(|row| self.depth_above(row)),
            Place::After(_) | Place::End => None,
        };
        before.unwrap_or_else(|| {
            self.index_at(at)
                .checked_sub(1)
                .map_or(0, |row| self.depth_below(row))
        })
    }

    /// Returns the depth of a cue put just above the row `row`
    fn depth_above(&self, row: usize) -> u8 {
        self.depths().get(row).copied().unwrap_or(0)
    }

    /// Returns the deepest depth of a cue put just below the row `row`
    fn depth_below(&self, row: usize) -> u8 {
        self.depths().get(row).map_or(0, |depth| {
            depth.saturating_add(u8::from(self.cues[row].kind == Kind::Group))
        })
    }

    /// Moves the cue at row `from`, together with the cues that it holds, so
    /// that it lands at row `to` of the list without them, leaving standby
    /// where it is.  The depths of the block shift together so that its first
    /// cue takes the depth of a cue added where the block lands: below the
    /// row above it when it moves down, and above the row below it when it
    /// moves up.  Moving the block onto a group therefore takes it into the
    /// group, and moving it off the edge of one takes it out.
    pub(in crate::back) fn with_block_moved(self, from: usize, to: usize) -> Self {
        let depths = self.depths();
        let extent = self.extent(from);
        let lead = depths.get(from).copied().unwrap_or(0);
        let mut cues = self.cues;
        let block: Vec<(Cue, u8)> = cues
            .drain(extent.clone())
            .zip(depths[extent].iter().copied())
            .collect();
        let rest = Self { cues, ..self };
        let to = to.min(rest.cues.len());
        let landing = if to > from {
            rest.depth_below(to - 1)
        } else {
            rest.depth_above(to)
        };
        let moved = block.into_iter().map(|(cue, depth)| Cue {
            depth: u8::try_from((i16::from(depth) - i16::from(lead) + i16::from(landing)).max(0))
                .unwrap_or(u8::MAX),
            ..cue
        });
        let mut cues = rest.cues;
        let tail = cues.split_off(to);
        cues.extend(moved);
        cues.extend(tail);
        Self { cues, ..rest }
    }
}
