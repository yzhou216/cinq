// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the State column of a collapsed group that hides a problem

use super::{
    super::{
        Column,
        columns::{counted, state},
        hidden_problem, lines,
    },
    texts,
};
use crate::{
    back::{Command, Cue, CueId, CueList, Edit, Kind, Workspace},
    front::view::theme,
};

/// Returns a list of the group 1, collapsed or not, that holds a memo, a
/// sound with no file, and a fade with no target
fn troubled(collapsed: bool) -> Workspace {
    let group = Cue::new(CueId(1), "1", Kind::Group, "Group")
        .collapsed(collapsed)
        .build();
    let within = |cue: Cue| Cue { depth: 1, ..cue };
    let cues = vec![
        group,
        within(Cue::new(CueId(2), "2", Kind::Memo, "Note").build()),
        within(Cue::new(CueId(3), "3", Kind::Audio, "Rain").build()),
        within(Cue::new(CueId(4), "4", Kind::Fade, "Fade").build()),
    ];
    Workspace::new(vec![CueList::new("Main", cues)])
}

fn group_state(workspace: &Workspace) -> (String, bool) {
    let list = &workspace.lists()[0];
    let groups = list.groups();
    let cue = &list.cues[0];
    let hidden = hidden_problem(workspace, &groups, cue);
    let (state, style) = state(workspace, cue, hidden, false);
    let theme = theme::modus_vivendi();
    (
        state,
        style(&theme).color == theme::danger(false)(&theme).color,
    )
}

#[test]
fn a_collapsed_group_reads_the_first_problem_of_the_cues_it_hides_as_they_do() {
    assert_eq!(group_state(&troubled(true)), ("NO FILE".to_owned(), true));
    let target = Command::Edit(CueId(3), Edit::File("rain.ogg".to_owned()));
    let loaded = troubled(true).apply(target);
    assert_eq!(group_state(&loaded).0, "loading");
}

#[test]
fn an_expanded_group_reads_no_problem_of_the_cues_it_holds() {
    assert_eq!(group_state(&troubled(false)), (String::new(), false));
}

#[test]
fn a_collapsed_group_on_standby_reads_next_and_then_the_hidden_problem() {
    let workspace = troubled(true);
    let list = &workspace.lists()[0];
    let cue = &list.cues[0];
    let hidden = hidden_problem(&workspace, &list.groups(), cue);
    let lines_of = |hidden| -> Vec<String> {
        lines(&workspace, cue, true, hidden, &[], false)
            .into_iter()
            .map(|(line, _)| line)
            .collect()
    };
    assert_eq!(lines_of(hidden), ["NEXT", "NO FILE"]);
    assert_eq!(lines_of(None), ["NEXT"]);
}

#[test]
fn a_running_group_gives_its_own_state_before_the_problem_of_a_hidden_cue() {
    let hold = Cue::new(CueId(5), "5", Kind::Wait, "Hold")
        .duration(10.0)
        .depth(1)
        .build();
    let workspace = troubled(true);
    let cues = workspace.lists()[0].cues.iter().cloned().chain([hold]);
    let running = Workspace::new(vec![CueList::new("Main", cues.collect())])
        .apply(Command::Go { list: 0 })
        .apply(Command::Tick(1.0));
    let (state, _) = group_state(&running);
    assert!(state.contains("left"), "{state}");
}

#[test]
fn the_text_of_the_state_column_of_a_collapsed_group_reads_the_hidden_problem() {
    let workspace = troubled(true);
    assert_eq!(texts(&workspace, Column::State)[0], "NEXT\nNO FILE");
    assert_eq!(texts(&troubled(false), Column::State)[0], "NEXT");
}

#[test]
fn only_a_collapsed_group_counts_the_cues_it_hides_after_its_name() {
    let count = |workspace: &Workspace, row: usize| {
        let list = &workspace.lists()[0];
        counted(workspace, &list.groups(), &list.cues[row])
    };
    assert_eq!(count(&troubled(true), 0).as_deref(), Some(" (3 cues)"));
    assert_eq!(count(&troubled(true), 1), None);
    assert_eq!(count(&troubled(false), 0), None);
}
