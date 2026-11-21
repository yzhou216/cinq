// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Changes in a row that make one change: edits of one setting of one cue,
//! and moves of one cue, whether or not they change its depth

use super::Change;
use crate::back::CueId;

impl Change {
    /// Returns whether `next` edits the setting that this change edited, or
    /// moves the cue that it moved, so that the two make one change
    pub(super) fn takes(&self, next: &Change) -> bool {
        match (self, next) {
            (last, next) if last.mover().is_some() => last.mover() == next.mover(),
            (
                Change::Edited { cue, redo, .. },
                Change::Edited {
                    cue: next,
                    redo: again,
                    ..
                },
            ) => cue == next && redo.same_setting(again),
            (
                Change::Outputs {
                    setting: Some(setting),
                    ..
                },
                Change::Outputs {
                    setting: Some(again),
                    ..
                },
            ) => setting == again,
            (Change::Patched { edit, .. }, Change::Patched { edit: next, .. }) => {
                edit.same_setting(next)
            }
            _ => false,
        }
    }

    /// Returns the cue that this change moved up or down alone, if it did
    fn mover(&self) -> Option<CueId> {
        match self {
            Self::Moved { cue, .. } => Some(*cue),
            Self::Regrouped {
                cues, moved: true, ..
            } => match cues.as_slice() {
                [cue] => Some(*cue),
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns this change followed by `next` as one change, which takes
    /// back to before this one and makes again what `next` made
    pub(super) fn then(self, next: Self) -> Self {
        match (self, next) {
            (Self::Edited { cue, undo, .. }, Self::Edited { redo, .. }) => {
                Self::Edited { cue, undo, redo }
            }
            (Self::Moved { cue, from, .. }, Self::Moved { to, .. }) => {
                Self::Moved { cue, from, to }
            }
            (
                Self::Moved { cue, from, .. },
                Self::Regrouped {
                    list,
                    deeper,
                    moved: true,
                    before,
                    after,
                    ..
                },
            ) => Self::Regrouped {
                list,
                cues: vec![cue],
                deeper,
                moved: true,
                before: placed_at(before, cue, from),
                after,
            },
            (
                Self::Regrouped {
                    list,
                    cues,
                    deeper,
                    moved: true,
                    before,
                    after,
                },
                Self::Moved { cue, to, .. },
            ) => Self::Regrouped {
                list,
                cues,
                deeper,
                moved: true,
                before,
                after: placed_at(after, cue, to),
            },
            (
                Self::Regrouped {
                    list,
                    cues,
                    moved: true,
                    before,
                    ..
                },
                Self::Regrouped {
                    after, moved: true, ..
                },
            ) => Self::Regrouped {
                deeper: deeper_than(&before, &after, &cues),
                list,
                cues,
                moved: true,
                before,
                after,
            },
            (
                Self::Outputs { before, .. },
                Self::Outputs {
                    after,
                    setting,
                    cut,
                    pasted,
                    anchor,
                    ..
                },
            ) => Self::Outputs {
                before,
                after,
                setting,
                cut,
                pasted,
                anchor,
            },
            (Self::Patched { before, .. }, Self::Patched { edit, after, .. }) => Self::Patched {
                edit,
                before,
                after,
            },
            (_, next) => next,
        }
    }

    /// Returns whether the change leaves the show as it found it, as edits
    /// or moves in a row that end where they began do
    pub(super) fn changes_nothing(&self) -> bool {
        match self {
            Self::Edited { undo, redo, .. } => undo == redo,
            Self::Moved { from, to, .. } => from == to,
            Self::Regrouped { before, after, .. } => before == after,
            Self::Outputs { before, after, .. } => before == after,
            Self::Patched { before, after, .. } => before == after,
            _ => false,
        }
    }
}

/// Returns whether the first of `cues` lies deeper in the arrangement `after`
/// than in `before`
fn deeper_than(before: &[(CueId, u8)], after: &[(CueId, u8)], cues: &[CueId]) -> bool {
    let depth = |arrangement: &[(CueId, u8)]| {
        cues.first().and_then(|&cue| {
            arrangement
                .iter()
                .find_map(|&(id, depth)| (id == cue).then_some(depth))
        })
    };
    depth(after) > depth(before)
}

/// Returns the arrangement with the entry of `cue` at the place `to`, as a
/// move of the cue within its list puts it
fn placed_at(arrangement: Vec<(CueId, u8)>, cue: CueId, to: usize) -> Vec<(CueId, u8)> {
    let (moving, rest): (Vec<_>, Vec<_>) = arrangement.into_iter().partition(|&(id, _)| id == cue);
    let (head, tail) = rest.split_at(to.min(rest.len()));
    head.iter().chain(&moving).chain(tail).copied().collect()
}
