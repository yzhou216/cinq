// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of group cues: which cues a group holds, what GO on one fires, and
//! moving cues into groups and out of them

use super::*;

fn depths(workspace: &Workspace) -> Vec<u8> {
    workspace.lists()[0].depths()
}

/// Returns each cue that GO on `start` fires with its delay
fn chain(workspace: &Workspace, start: u32) -> Vec<(u32, f32)> {
    workspace.lists()[0]
        .chain(CueId(start))
        .map(|(cue, delay)| (cue.id.0, delay))
        .collect()
}

fn indent(ids: &[u32]) -> Command {
    Command::Indent(ids.iter().map(|&id| CueId(id)).collect())
}

fn outdent(ids: &[u32]) -> Command {
    Command::Outdent(ids.iter().map(|&id| CueId(id)).collect())
}

#[test]
fn cue_lies_only_as_deep_as_the_rows_above_it_allow() {
    let workspace = workspace(vec![
        within(2, audio(1)),
        group(2),
        within(1, group(3)),
        within(3, audio(4)),
        within(2, audio(5)),
        within(1, audio(6)),
        audio(7),
        within(1, audio(8)),
    ]);
    assert_eq!(depths(&workspace), [0, 0, 1, 2, 2, 1, 0, 0]);
    let list = &workspace.lists()[0];
    assert_eq!(list.extent(1), 1..6);
    assert_eq!(list.extent(2), 2..5);
    assert_eq!(list.extent(6), 6..7);
    let members: Vec<u32> = list.members(CueId(2)).iter().map(|cue| cue.id.0).collect();
    assert_eq!(members, [3, 4, 5, 6]);
    assert_eq!(list.group_of(CueId(5)).map(|cue| cue.id.0), Some(3));
    assert_eq!(list.group_of(CueId(6)).map(|cue| cue.id.0), Some(2));
    assert_eq!(list.group_of(CueId(8)), None);
}

#[test]
fn go_on_a_group_fires_every_cue_in_it_and_moves_standby_past_it() {
    let workspace = workspace(vec![
        group(1),
        within(1, continuing(2)),
        within(1, audio(3)),
        audio(4),
    ]);
    assert_eq!(chain(&workspace, 1), [(1, 0.0), (2, 0.0), (3, 0.0)]);
    let fired = workspace.apply(GO);
    assert!(running(&fired).ends_with(&[2, 3]));
    assert_eq!(standby(&fired), Some(4));
}

#[test]
fn pre_waits_of_the_groups_around_a_cue_hold_it_back() {
    let held = |id, wait: f32| Cue {
        pre_wait: wait.into(),
        ..group(id)
    };
    let workspace = workspace(vec![
        held(1, 1.0),
        within(1, held(2, 2.0)),
        within(2, audio(3)),
        within(1, audio(4)),
    ]);
    assert_eq!(
        chain(&workspace, 1),
        [(1, 0.0), (2, 1.0), (3, 3.0), (4, 1.0)]
    );
}

#[test]
fn group_goes_on_by_its_own_setting_once_its_cues_have_played() {
    let following = Cue {
        advance: Advance::Follow,
        pre_wait: 1.0.into(),
        ..group(1)
    };
    let late = Cue {
        pre_wait: 2.0.into(),
        ..within(1, audio(3))
    };
    let workspace = workspace(vec![following, within(1, audio(2)), late, audio(4)]);
    // Ten seconds of sound, the later one two seconds into the group
    assert_eq!(
        chain(&workspace, 1),
        [(1, 0.0), (2, 1.0), (3, 1.0), (4, 13.0)]
    );
}

#[test]
fn cue_in_a_group_fires_alone() {
    let workspace = workspace(vec![
        group(1),
        within(1, continuing(2)),
        within(1, audio(3)),
    ]);
    assert_eq!(chain(&workspace, 2), [(2, 0.0)]);
}

#[test]
fn cue_goes_into_the_group_above_it_with_the_cues_that_it_holds() {
    let workspace = workspace(vec![
        group(1),
        audio(2),
        group(3),
        within(1, audio(4)),
        audio(5),
    ]);
    let indented = workspace.clone().apply(indent(&[2]));
    assert_eq!(depths(&indented), [0, 1, 0, 1, 0]);
    let nested = indented.apply(indent(&[3]));
    assert_eq!(depths(&nested), [0, 1, 1, 2, 0]);
    // No group above the first cue, nor directly above the last
    let unchanged = workspace.clone().apply(indent(&[1]));
    assert_eq!(depths(&unchanged), depths(&workspace));
    assert!(unchanged.undoable().is_none());
}

#[test]
fn cue_leaves_its_group_for_the_row_below_it() {
    let workspace = workspace(vec![
        group(1),
        within(1, audio(2)),
        within(1, group(3)),
        within(2, audio(4)),
        within(1, audio(5)),
        audio(6),
    ]);
    let out = workspace.clone().apply(outdent(&[3]));
    assert_eq!(ids(&out), [1, 2, 5, 3, 4, 6]);
    assert_eq!(depths(&out), [0, 1, 1, 0, 1, 0]);
    let undone = out.apply(Command::Undo);
    assert_eq!(ids(&undone), ids(&workspace));
    assert_eq!(depths(&undone), depths(&workspace));
    let redone = undone.apply(Command::Redo);
    assert_eq!(ids(&redone), [1, 2, 5, 3, 4, 6]);
    let both = workspace.apply(outdent(&[2, 5]));
    assert_eq!(ids(&both), [1, 3, 4, 2, 5, 6]);
    assert_eq!(depths(&both), [0, 1, 2, 0, 0, 0]);
}

#[test]
fn show_file_keeps_the_depth_of_each_cue() {
    let workspace = workspace(vec![group(1), within(1, audio(2))]);
    let text = shown(&workspace);
    assert!(text.contains("depth = 1"));
    let read = Workspace::from_show(&text, "/show").expect("the show reads back");
    assert_eq!(read.lists()[0].depths(), [0, 1]);
}

#[test]
fn rows_nested_as_deep_as_a_depth_can_count_do_not_overflow() {
    let rows: Vec<Cue> = (0..300_u32)
        .map(|row| within(u8::try_from(row).unwrap_or(u8::MAX), group(row + 1)))
        .collect();
    let workspace = workspace(rows);
    assert_eq!(depths(&workspace).last(), Some(&u8::MAX));
    assert_eq!(workspace.lists()[0].extent(0), 0..300);
    assert_eq!(chain(&workspace, 1).len(), 300);
}

#[test]
fn block_that_already_lies_as_deep_as_one_can_does_not_indent() {
    let nested =
        (1..=255_u32).map(|row| within(u8::try_from(row).unwrap_or(u8::MAX), group(row + 2)));
    let workspace = workspace([group(1), group(2)].into_iter().chain(nested).collect());
    assert_eq!(depths(&workspace).last(), Some(&u8::MAX));
    let indented = workspace.clone().apply(indent(&[2]));
    assert_eq!(depths(&indented), depths(&workspace));
    assert!(indented.undoable().is_none());
}

#[test]
fn show_file_keeps_depths_that_the_rows_above_do_not_allow() {
    let workspace = workspace(vec![
        within(2, audio(1)),
        group(2),
        within(1, group(3)),
        within(7, audio(4)),
        audio(5),
    ]);
    let text = shown(&workspace);
    assert!(text.contains("kind = \"group\""));
    let read = read_back(&text);
    assert_eq!(
        read.lists()[0].arrangement(),
        workspace.lists()[0].arrangement()
    );
    assert_eq!(depths(&read), [0, 0, 1, 2, 0]);
}

#[test]
fn indent_below_a_group_at_the_deepest_depth_leaves_the_list_alone() {
    let mut cues: Vec<Cue> = (0..=u8::MAX)
        .map(|depth| within(depth, group(u32::from(depth) + 1)))
        .collect();
    cues.push(within(u8::MAX, audio(1000)));
    let before = depths(&workspace(cues.clone()));
    let indented = workspace(cues).apply(indent(&[1000]));
    assert_eq!(depths(&indented), before);
}
