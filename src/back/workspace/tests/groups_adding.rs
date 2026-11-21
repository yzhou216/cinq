// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the depth that an added cue takes in a list with groups

use super::*;

fn depths(workspace: &Workspace) -> Vec<u8> {
    workspace.lists()[0].depths()
}

fn grouped() -> Workspace {
    workspace(vec![group(1), within(1, audio(2)), audio(3)])
}

fn added(workspace: Workspace, kind: Kind, at: Place) -> Workspace {
    workspace.apply(add(0, kind, at))
}

#[test]
fn cue_added_below_a_group_is_the_first_cue_in_it() {
    let added = added(grouped(), Kind::Memo, Place::After(CueId(1)));
    assert_eq!(ids(&added), [1, 4, 2, 3]);
    assert_eq!(depths(&added), [0, 1, 1, 0]);
}

#[test]
fn cue_added_below_a_cue_in_a_group_joins_the_group() {
    let added = added(grouped(), Kind::Memo, Place::After(CueId(2)));
    assert_eq!(ids(&added), [1, 2, 4, 3]);
    assert_eq!(depths(&added), [0, 1, 1, 0]);
    let nested = workspace(vec![group(1), within(1, group(2)), within(2, audio(3))]);
    let deeper = self::added(nested.clone(), Kind::Memo, Place::After(CueId(3)));
    assert_eq!(depths(&deeper), [0, 1, 2, 2]);
    let inner = self::added(nested, Kind::Memo, Place::After(CueId(2)));
    assert_eq!(depths(&inner), [0, 1, 2, 2]);
}

#[test]
fn cue_added_above_a_row_takes_the_depth_of_that_row() {
    let above_member = added(grouped(), Kind::Memo, Place::Before(CueId(2)));
    assert_eq!(ids(&above_member), [1, 4, 2, 3]);
    assert_eq!(depths(&above_member), [0, 1, 1, 0]);
    let above_top = added(grouped(), Kind::Memo, Place::Before(CueId(3)));
    assert_eq!(ids(&above_top), [1, 2, 4, 3]);
    assert_eq!(depths(&above_top), [0, 1, 0, 0]);
    let above_group = added(grouped(), Kind::Memo, Place::Before(CueId(1)));
    assert_eq!(depths(&above_group), [0, 0, 1, 0]);
}

#[test]
fn cue_added_at_the_end_follows_the_last_row() {
    let in_group = workspace(vec![group(1), within(1, audio(2))]);
    assert_eq!(depths(&added(in_group, Kind::Memo, Place::End)), [0, 1, 1]);
    let after_top = added(grouped(), Kind::Memo, Place::End);
    assert_eq!(depths(&after_top), [0, 1, 0, 0]);
    let after_group = workspace(vec![audio(1), group(2)]);
    assert_eq!(
        depths(&added(after_group, Kind::Memo, Place::End)),
        [0, 0, 1]
    );
}

#[test]
fn groups_added_in_a_row_lie_side_by_side() {
    let added = grouped().apply(Command::Add {
        list: 0,
        kind: Kind::Group,
        at: Place::After(CueId(1)),
        count: 3,
    });
    assert_eq!(ids(&added), [1, 4, 5, 6, 2, 3]);
    assert_eq!(depths(&added), [0, 1, 1, 1, 1, 0]);
}

#[test]
fn control_cue_added_below_a_group_does_not_target_the_group_that_holds_it() {
    let target = |added: &Workspace| added.cue(CueId(4)).and_then(|cue| cue.target);
    let below_group = added(grouped(), Kind::Stop, Place::After(CueId(1)));
    assert_eq!(depths(&below_group), [0, 1, 1, 0]);
    assert_eq!(target(&below_group), None);
    let above_member = self::added(grouped(), Kind::Start, Place::Before(CueId(2)));
    assert_eq!(target(&above_member), None);
    let below_member = self::added(grouped(), Kind::Stop, Place::After(CueId(2)));
    assert_eq!(target(&below_member), Some(CueId(2)));
    let outside = workspace(vec![audio(1), group(2), within(1, audio(3))]);
    let above_group = self::added(outside, Kind::Start, Place::Before(CueId(2)));
    assert_eq!(target(&above_group), Some(CueId(1)));
}

#[test]
fn start_cue_added_below_a_group_does_not_fire_it_forever() {
    let added = added(grouped(), Kind::Start, Place::After(CueId(1)));
    let mut fired = added
        .apply(Command::Standby {
            list: 0,
            cue: Some(CueId(1)),
        })
        .apply(GO);
    for _ in 0..200 {
        fired = fired.apply(Command::Tick(1.0));
    }
    assert!(
        running(&fired).is_empty(),
        "still running: {:?}",
        running(&fired)
    );
}

#[test]
fn undo_and_redo_of_an_addition_keep_the_depth() {
    let added = added(grouped(), Kind::Memo, Place::After(CueId(1)));
    let undone = added.apply(Command::Undo);
    assert_eq!(ids(&undone), [1, 2, 3]);
    assert_eq!(depths(&undone), [0, 1, 0]);
    let redone = undone.apply(Command::Redo);
    assert_eq!(ids(&redone), [1, 4, 2, 3]);
    assert_eq!(depths(&redone), [0, 1, 1, 0]);
}
