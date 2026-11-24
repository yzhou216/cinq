// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the State column of a collapsed group that hides a problem or a
//! cue that runs

use iced::widget::text;

use std::collections::BTreeSet;

use super::{
    super::{
        Column,
        columns::{counted, hidden_problem, lines, shown_lines, state},
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
        lines(&workspace, cue, true, hidden, &[], &BTreeSet::new(), false)
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

/// Returns a collapsed group 1 that holds the waits 2 and 3, on which each of
/// `commands` acts, and the lines of the State column of the group
fn counted_lines(commands: &[Command]) -> Vec<(String, bool)> {
    let wait = |id: u32| {
        Cue::new(CueId(id), id.to_string(), Kind::Wait, "Hold")
            .duration(10.0)
            .depth(1)
            .build()
    };
    let group = Cue::new(CueId(1), "1", Kind::Group, "Group")
        .collapsed(true)
        .build();
    let workspace = commands.iter().cloned().fold(
        Workspace::new(vec![CueList::new("Main", vec![group, wait(2), wait(3)])]),
        Workspace::apply,
    );
    let list = &workspace.lists()[0];
    let theme = theme::modus_vivendi();
    shown_lines(
        &workspace,
        &list.groups(),
        &list.cues[0],
        false,
        &[],
        &BTreeSet::new(),
        false,
    )
    .into_iter()
    .map(|(line, style)| (line, style(&theme).color == text::warning(&theme).color))
    .collect()
}

#[test]
fn a_collapsed_group_counts_the_cues_it_hides_that_run_and_the_paused_ones_first() {
    let start = |id| Command::Start(CueId(id));
    assert!(counted_lines(&[]).is_empty());
    assert_eq!(
        counted_lines(&[start(2), start(3)]),
        [("2 running".to_owned(), false)]
    );
    assert_eq!(
        counted_lines(&[start(2), start(3), Command::Hold(CueId(2))]),
        [
            ("1 paused".to_owned(), true),
            ("1 running".to_owned(), false)
        ]
    );
}

#[test]
fn a_collapsed_group_tells_each_warning_of_the_cues_it_hides_once() {
    let light = |id: u32| Cue::new(CueId(id), id.to_string(), Kind::Light, "Look").depth(1);
    let group = |collapsed| {
        Cue::new(CueId(1), "1", Kind::Group, "Group")
            .collapsed(collapsed)
            .build()
    };
    let told = |collapsed| {
        let workspace = Workspace::new(vec![CueList::new(
            "Main",
            vec![group(collapsed), light(2).build(), light(3).build()],
        )]);
        let list = &workspace.lists()[0];
        shown_lines(
            &workspace,
            &list.groups(),
            &list.cues[0],
            false,
            &[],
            &BTreeSet::new(),
            false,
        )
        .into_iter()
        .map(|(line, _)| line)
        .collect::<Vec<_>>()
    };
    assert_eq!(told(true), ["NO LEVELS"]);
    assert!(told(false).is_empty());
}
