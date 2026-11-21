// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of adding, pasting, moving, and indenting cues next to collapsed
//! groups, which count as one row, as the cue list shows them

use super::*;
use crate::back::Change;

fn collapse(id: u32) -> Command {
    Command::Collapse {
        cues: vec![CueId(id)],
        collapsed: true,
    }
}

fn depths(workspace: &Workspace) -> Vec<u8> {
    workspace.lists()[0].depths()
}

fn raw_depths(workspace: &Workspace) -> Vec<u8> {
    workspace.lists()[0]
        .cues
        .iter()
        .map(|cue| cue.depth)
        .collect()
}

/// Returns a collapsed group of two levels, with a cue above and below it
fn closed() -> Workspace {
    workspace(vec![
        audio(1),
        group(2),
        within(1, audio(3)),
        within(1, group(4)),
        within(2, audio(5)),
        audio(6),
    ])
    .apply(collapse(2))
}

fn target(workspace: &Workspace, id: u32) -> Option<CueId> {
    workspace.cue(CueId(id)).and_then(|cue| cue.target)
}

fn paste(cues: Vec<Cue>, at: Place) -> Command {
    Command::Paste { list: 0, cues, at }
}

/// Asserts that undo and redo bring back the lists of `before`
/// and `after`, with their depths
fn assert_undoes(before: &Workspace, after: &Workspace) {
    let undone = after.clone().apply(Command::Undo);
    assert_eq!(ids(&undone), ids(before));
    assert_eq!(raw_depths(&undone), raw_depths(before));
    let redone = undone.apply(Command::Redo);
    assert_eq!(ids(&redone), ids(after));
    assert_eq!(raw_depths(&redone), raw_depths(after));
}

#[test]
fn cue_added_below_a_collapsed_group_lands_below_its_cues() {
    let before = closed();
    let added = before
        .clone()
        .apply(add(0, Kind::Memo, Place::After(CueId(2))));
    assert_eq!(ids(&added), [1, 2, 3, 4, 5, 7, 6]);
    assert_eq!(depths(&added), [0, 0, 1, 1, 2, 0, 0]);
    assert_undoes(&before, &added);
}

#[test]
fn cue_added_below_a_collapsed_group_in_a_group_stays_in_that_group() {
    let before = workspace(vec![
        group(1),
        within(1, group(2)),
        within(2, audio(3)),
        within(1, audio(4)),
    ])
    .apply(collapse(2));
    let added = before
        .clone()
        .apply(add(0, Kind::Memo, Place::After(CueId(2))));
    assert_eq!(ids(&added), [1, 2, 3, 5, 4]);
    assert_eq!(depths(&added), [0, 1, 2, 1, 1]);
    assert_undoes(&before, &added);
}

#[test]
fn cues_added_in_a_row_below_a_collapsed_group_stay_side_by_side() {
    let added = closed().apply(Command::Add {
        list: 0,
        kind: Kind::Memo,
        at: Place::After(CueId(2)),
        count: 3,
    });
    assert_eq!(ids(&added), [1, 2, 3, 4, 5, 7, 8, 9, 6]);
    assert_eq!(depths(&added), [0, 0, 1, 1, 2, 0, 0, 0, 0]);
}

#[test]
fn group_added_below_a_collapsed_group_lies_beside_it() {
    let added = closed().apply(add(0, Kind::Group, Place::After(CueId(2))));
    assert_eq!(depths(&added), [0, 0, 1, 1, 2, 0, 0]);
    assert!(!added.cue(CueId(7)).is_some_and(|cue| cue.collapsed));
}

#[test]
fn cue_added_below_an_expanded_group_still_goes_into_it() {
    let open = closed().apply(Command::Collapse {
        cues: vec![CueId(2)],
        collapsed: false,
    });
    let added = open.apply(add(0, Kind::Memo, Place::After(CueId(2))));
    assert_eq!(ids(&added), [1, 2, 7, 3, 4, 5, 6]);
    assert_eq!(depths(&added), [0, 0, 1, 1, 1, 2, 0]);
}

#[test]
fn cue_added_above_a_collapsed_group_takes_its_depth() {
    let added = closed().apply(add(0, Kind::Memo, Place::Before(CueId(2))));
    assert_eq!(ids(&added), [1, 7, 2, 3, 4, 5, 6]);
    assert_eq!(depths(&added), [0, 0, 0, 1, 1, 2, 0]);
}

#[test]
fn cue_added_at_the_end_below_a_collapsed_group_lies_outside_it() {
    let before =
        workspace(vec![group(1), within(1, group(2)), within(2, audio(3))]).apply(collapse(1));
    let added = before.clone().apply(add(0, Kind::Stop, Place::End));
    assert_eq!(depths(&added), [0, 1, 2, 0]);
    assert_eq!(target(&added, 4), Some(CueId(1)));
}

#[test]
fn control_cue_added_below_a_collapsed_group_targets_the_group() {
    [Kind::Stop, Kind::Pause, Kind::Start, Kind::GoTo]
        .into_iter()
        .for_each(|kind| {
            let added = closed().apply(add(0, kind, Place::After(CueId(2))));
            assert_eq!(target(&added, 7), Some(CueId(2)), "{kind:?}");
            assert_eq!(depths(&added), [0, 0, 1, 1, 2, 0, 0]);
        });
}

#[test]
fn control_cue_added_next_to_a_collapsed_group_targets_what_the_table_shows_above() {
    let below = closed().apply(add(0, Kind::Stop, Place::Before(CueId(6))));
    assert_eq!(target(&below, 7), Some(CueId(2)));
    let above = closed().apply(add(0, Kind::Stop, Place::Before(CueId(2))));
    assert_eq!(target(&above, 7), Some(CueId(1)));
}

#[test]
fn control_cue_added_below_a_collapsed_group_that_it_would_lie_in_has_no_target() {
    let inside = closed().apply(add(0, Kind::Stop, Place::After(CueId(3))));
    assert_eq!(depths(&inside), [0, 0, 1, 1, 1, 2, 0]);
    assert_eq!(target(&inside, 7), None);
}

#[test]
fn cues_pasted_below_a_collapsed_group_land_below_its_cues() {
    let before = closed();
    let clipboard = vec![group(10), within(1, audio(11))];
    let pasted = before
        .clone()
        .apply(paste(clipboard, Place::After(CueId(2))));
    assert_eq!(ids(&pasted), [1, 2, 3, 4, 5, 7, 8, 6]);
    assert_eq!(depths(&pasted), [0, 0, 1, 1, 2, 0, 1, 0]);
    assert_undoes(&before, &pasted);
    let above = before.apply(paste(vec![audio(10)], Place::Before(CueId(2))));
    assert_eq!(ids(&above), [1, 7, 2, 3, 4, 5, 6]);
}

#[test]
fn block_moved_down_past_a_collapsed_group_lands_below_its_cues() {
    let before = closed();
    let moved = before.clone().apply(moving(1, 1));
    assert_eq!(ids(&moved), [2, 3, 4, 5, 1, 6]);
    assert_eq!(depths(&moved), [0, 1, 1, 2, 0, 0]);
    assert!(matches!(moved.undoable(), Some(Change::Moved { .. })));
    assert_undoes(&before, &moved);
}

#[test]
fn block_moved_up_past_a_collapsed_group_lands_above_it() {
    let before = closed();
    let moved = before.clone().apply(moving(6, -1));
    assert_eq!(ids(&moved), [1, 6, 2, 3, 4, 5]);
    assert_eq!(depths(&moved), [0, 0, 0, 1, 1, 2]);
    assert_undoes(&before, &moved);
}

#[test]
fn collapsed_group_moves_as_one_row_with_its_cues() {
    let before = closed();
    let down = before.clone().apply(moving(2, 1));
    assert_eq!(ids(&down), [1, 6, 2, 3, 4, 5]);
    assert_eq!(depths(&down), [0, 0, 0, 1, 1, 2]);
    assert!(matches!(down.undoable(), Some(Change::Regrouped { .. })));
    assert_undoes(&before, &down);
    let up = before.clone().apply(moving(2, -1));
    assert_eq!(ids(&up), [2, 3, 4, 5, 1, 6]);
    assert_eq!(depths(&up), [0, 1, 1, 2, 0, 0]);
    assert_undoes(&before, &up);
}

#[test]
fn move_counts_each_collapsed_group_as_one_row() {
    let before = workspace(vec![
        audio(1),
        group(2),
        within(1, audio(3)),
        group(4),
        within(1, audio(5)),
        audio(6),
    ])
    .apply(collapse(2))
    .apply(collapse(4));
    let moved = before.clone().apply(moving(1, 2));
    assert_eq!(ids(&moved), [2, 3, 4, 5, 1, 6]);
    assert_eq!(depths(&moved), [0, 1, 0, 1, 0, 0]);
    assert_undoes(&before, &moved);
    let clamped = before.clone().apply(moving(1, 10));
    assert_eq!(ids(&clamped), [2, 3, 4, 5, 6, 1]);
    let past_top = before.apply(moving(6, -10));
    assert_eq!(ids(&past_top), [6, 1, 2, 3, 4, 5]);
}

#[test]
fn move_past_a_collapsed_group_in_a_group_stays_in_that_group() {
    let before = workspace(vec![
        group(1),
        within(1, audio(2)),
        within(1, group(3)),
        within(2, audio(4)),
        within(1, audio(5)),
        audio(6),
    ])
    .apply(collapse(3));
    let moved = before.clone().apply(moving(2, 1));
    assert_eq!(ids(&moved), [1, 3, 4, 2, 5, 6]);
    assert_eq!(depths(&moved), [0, 1, 2, 1, 1, 0]);
    assert!(matches!(moved.undoable(), Some(Change::Moved { .. })));
    assert_undoes(&before, &moved);
}

#[test]
fn move_next_to_an_expanded_group_works_as_before() {
    let open = closed().apply(Command::Collapse {
        cues: vec![CueId(2)],
        collapsed: false,
    });
    let moved = open.clone().apply(moving(1, 1));
    assert_eq!(ids(&moved), [2, 1, 3, 4, 5, 6]);
    assert_eq!(depths(&moved), [0, 1, 1, 1, 2, 0]);
    assert_undoes(&open, &moved);
}

#[test]
fn row_below_a_collapsed_group_indents_into_it_as_its_last_cue() {
    let before = closed();
    let indented = before.clone().apply(Command::Indent(vec![CueId(6)]));
    assert_eq!(ids(&indented), [1, 2, 3, 4, 5, 6]);
    assert_eq!(depths(&indented), [0, 0, 1, 1, 2, 1]);
    assert_eq!(
        indented.lists()[0]
            .groups()
            .group_of(CueId(6))
            .map(|g| g.id),
        Some(CueId(2))
    );
    assert!(matches!(
        indented.undoable(),
        Some(Change::Regrouped { .. })
    ));
    assert_undoes(&before, &indented);
}

#[test]
fn collapsed_group_below_a_collapsed_group_indents_into_it_with_its_cues() {
    let before = workspace(vec![
        group(1),
        within(1, audio(2)),
        group(3),
        within(1, audio(4)),
    ])
    .apply(collapse(1))
    .apply(collapse(3));
    let indented = before.clone().apply(Command::Indent(vec![CueId(3)]));
    assert_eq!(depths(&indented), [0, 1, 1, 2]);
    assert_undoes(&before, &indented);
}

#[test]
fn fade_and_devamp_cues_added_below_a_collapsed_group_have_no_target() {
    [Kind::Fade, Kind::Devamp].into_iter().for_each(|kind| {
        let added = closed().apply(add(0, kind, Place::After(CueId(2))));
        assert_eq!(target(&added, 7), None, "{kind:?}");
    });
}
