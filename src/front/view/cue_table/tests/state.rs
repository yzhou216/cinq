// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the State column for a cue that plays, waits, or fails to load

use iced::widget::text;

use super::{
    super::{columns::state, lines},
    show,
};
use crate::{
    back::{Command, Cue, CueId, CueList, Kind, OutputId, Sends, Workspace},
    front::view::theme,
};

fn shown(workspace: &Workspace) -> (String, bool) {
    let list = &workspace.lists()[0];
    let cue = &list.cues[0];
    let (state, style) = state(workspace, cue, None, false);
    let theme = theme::modus_vivendi();
    let yellow = style(&theme).color == text::warning(&theme).color;
    (state, yellow)
}

#[test]
fn a_cue_that_sends_to_no_output_left_reads_in_yellow_that_it_does() {
    let sends = [(OutputId(2), 0.0)].into();
    assert_eq!(shown(&show(Some(sends))), ("NO OUTPUT".to_owned(), true));
}

#[test]
fn a_cue_that_sends_somewhere_or_names_no_sends_reads_as_any_other() {
    let both: Sends = [(OutputId(1), 0.0), (OutputId(2), 0.0)].into();
    assert_eq!(shown(&show(Some(both))).0, "");
    assert_eq!(shown(&show(None)).0, "");
}

/// Returns the lines of the State column of the only cue, which is standby
fn standby_lines(workspace: &Workspace) -> Vec<String> {
    let cue = &workspace.lists()[0].cues[0];
    lines(workspace, cue, true, None, &[], false)
        .into_iter()
        .map(|(line, _)| line)
        .collect()
}

#[test]
fn a_standby_cue_with_a_problem_reads_next_and_then_the_problem() {
    let stranded = show(Some([(OutputId(2), 0.0)].into()));
    assert_eq!(standby_lines(&stranded), ["NEXT", "NO OUTPUT"]);
    let unloaded = Workspace::new(vec![CueList::new(
        "Main",
        vec![Cue::new(CueId(1), "1", Kind::Audio, "Rain").build()],
    )]);
    assert_eq!(standby_lines(&unloaded), ["NEXT", "NO FILE"]);
}

#[test]
fn the_state_of_a_row_that_stands_for_the_standby_cue_reads_next() {
    let workspace = show(None);
    let cue = &workspace.lists()[0].cues[0];
    assert_eq!(standby_lines(&workspace), ["NEXT"]);
    assert!(lines(&workspace, cue, false, None, &[], false).is_empty());
}

#[test]
fn a_cue_that_a_panic_is_fading_reads_fading_in_the_color_of_a_running_cue() {
    let read = |workspace: &Workspace| state(workspace, &workspace.lists()[0].cues[0], None, false);
    let running = show(None)
        .apply(Command::Go { list: 0 })
        .apply(Command::Tick(1.0));
    assert!(read(&running).0.ends_with("left"));
    let panicked = running.apply(Command::Panic).apply(Command::Tick(0.1));
    let (text_of, style) = read(&panicked);
    let theme = theme::modus_vivendi();
    assert_eq!(text_of, "fading");
    assert_eq!(style(&theme).color, text::success(&theme).color);
}
