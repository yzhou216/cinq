// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rounds of a clipboard pasted several times in a row, and the shape that
//! the cues of each keep

use std::collections::HashSet;

use itertools::Itertools;

use crate::back::{Cue, CueList};

/// Returns the round of each cue.  A clipboard pasted several times in a
/// row puts its cues again, so that a cue whose identity came already in
/// the round begins the next.
pub(super) fn rounds(cues: &[Cue]) -> Vec<usize> {
    cues.iter()
        .scan((0, HashSet::new()), |(round, seen), cue| {
            if !seen.insert(cue.id) {
                *round += 1;
                seen.clear();
                seen.insert(cue.id);
            }
            Some(*round)
        })
        .collect()
}

/// Returns how deep each cue lies within its own round, where the first of
/// a round is the top, which is the shape that a paste keeps
pub(super) fn shapes(cues: &[Cue], rounds: &[usize]) -> Vec<u8> {
    cues.iter()
        .zip(rounds)
        .chunk_by(|(_, round)| **round)
        .into_iter()
        .flat_map(|(_, round)| {
            let round: Vec<Cue> = round.map(|(cue, _)| cue.clone()).collect();
            CueList::depths_within(&round)
        })
        .collect()
}
