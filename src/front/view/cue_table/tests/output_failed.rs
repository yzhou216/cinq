// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the State column for a cue whose outputs failed

use std::collections::BTreeSet;

use iced::widget::text;

use super::{super::columns::lines, show};
use crate::{
    back::{Command, CueId, Edit, Media, OutputId, Workspace},
    front::view::theme,
};

#[test]
fn a_cue_whose_outputs_all_failed_reads_in_yellow_that_the_output_failed() {
    let read = |workspace: &Workspace, failing: &[OutputId]| {
        let cue = &workspace.lists()[0].cues[0];
        let theme = theme::modus_vivendi();
        lines(
            workspace,
            cue,
            false,
            None,
            failing,
            &BTreeSet::new(),
            false,
        )
        .into_iter()
        .map(|(text_of, style)| (text_of, style(&theme).color == text::warning(&theme).color))
        .collect::<Vec<_>>()
    };
    let failed = [OutputId(1)];
    let failure = ("OUTPUT FAILED".to_owned(), true);
    let elsewhere = show(Some([(OutputId(1), 0.0)].into()));
    assert_eq!(read(&elsewhere, &failed), [failure]);
    assert_eq!(read(&elsewhere, &[OutputId(7)]), []);
    let both = elsewhere
        .apply(Command::add_output(1, 1))
        .apply(Command::Edit(
            CueId(1),
            Edit::Sends(Some([(OutputId(1), 0.0), (OutputId(3), 0.0)].into())),
        ));
    assert_eq!(read(&both, &failed), []);
}

#[test]
fn a_cue_that_runs_keeps_its_state_above_the_line_that_the_output_failed() {
    let lines_of = |workspace: &Workspace| {
        let cue = &workspace.lists()[0].cues[0];
        lines(
            workspace,
            cue,
            false,
            None,
            &[OutputId(1)],
            &BTreeSet::new(),
            false,
        )
        .into_iter()
        .map(|(text_of, _)| text_of)
        .collect::<Vec<_>>()
    };
    let playing = show(Some([(OutputId(1), 0.0)].into()))
        .apply(Command::Go { list: 0 })
        .apply(Command::Tick(1.0));
    let running = lines_of(&playing);
    assert_eq!((running.len(), running[0].ends_with("left")), (2, true));
    assert_eq!(running[1], "OUTPUT FAILED");
    let paused = lines_of(&playing.apply(Command::Pause(true)));
    assert_eq!(paused, ["paused", "OUTPUT FAILED"]);
}

#[test]
fn a_cue_that_is_next_and_a_cue_with_a_problem_share_the_cell_with_the_failure_as_told() {
    let workspace = show(Some([(OutputId(1), 0.0)].into()));
    let cue = &workspace.lists()[0].cues[0];
    let next = lines(
        &workspace,
        cue,
        true,
        None,
        &[OutputId(1)],
        &BTreeSet::new(),
        false,
    );
    assert_eq!(
        next.into_iter()
            .map(|(text_of, _)| text_of)
            .collect::<Vec<_>>(),
        ["NEXT", "OUTPUT FAILED"]
    );
    // A cue that cannot play tells why alone, and GO skips it
    let broken = workspace.apply(Command::Loaded {
        cue: CueId(1),
        file: "rain.ogg".to_owned(),
        media: Media::Failed("no".to_owned()),
        seen: None,
    });
    let cue = &broken.lists()[0].cues[0];
    let broken = lines(
        &broken,
        cue,
        false,
        None,
        &[OutputId(1)],
        &BTreeSet::new(),
        false,
    );
    assert_eq!(
        broken
            .into_iter()
            .map(|(text_of, _)| text_of)
            .collect::<Vec<_>>(),
        ["BROKEN"]
    );
}
