// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the State column of the cue list

use iced::widget::text;

use super::{Column, state};
use crate::{
    back::{Advance, Command, Cue, CueId, CueList, Edit, Kind, Media, OutputId, Sends, Workspace},
    front::view::theme,
};

fn show(sends: Option<Sends>) -> Workspace {
    let cue = Cue::new(CueId(1), "1", Kind::Audio, "Rain")
        .file("rain.ogg")
        .build();
    Workspace::new(vec![CueList::new("Main", vec![cue])])
        .apply(Command::Loaded {
            cue: CueId(1),
            file: "rain.ogg".to_owned(),
            media: Media::Loaded {
                duration: 10.0,
                sound: Some(10.0),
            },
        })
        .apply(Command::add_output(1, 1))
        .apply(Command::Edit(CueId(1), Edit::Sends(sends)))
        .apply(Command::DeleteOutputs(vec![OutputId(2)]))
}

fn shown(workspace: &Workspace) -> (String, bool) {
    let list = &workspace.lists()[0];
    let cue = &list.cues[0];
    let (state, style) = state(workspace, &list.groups(), cue, false);
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
    assert_eq!(shown(&show(Some(both))).0, "NEXT");
    assert_eq!(shown(&show(None)).0, "NEXT");
}

/// Returns a list of a looping audio cue, a group that holds an audio cue of
/// ten seconds after a pre-wait of one, and a group that holds nothing
fn grouped(depth: u8) -> Workspace {
    let ten = |id: u32| {
        Cue::new(CueId(id), id.to_string(), Kind::Audio, "Rain")
            .file("rain.ogg")
            .pre_wait(1.0)
            .depth(depth)
            .advance(Advance::Continue)
            .post_wait(2.0)
            .build()
    };
    let group = |id: u32| Cue::new(CueId(id), id.to_string(), Kind::Group, "Group").build();
    let workspace = Workspace::new(vec![CueList::new("Main", vec![group(1), ten(2), group(3)])]);
    workspace.apply(Command::Loaded {
        cue: CueId(2),
        file: "rain.ogg".to_owned(),
        media: Media::Loaded {
            duration: 10.0,
            sound: Some(10.0),
        },
    })
}

fn texts(workspace: &Workspace, column: Column) -> Vec<String> {
    let groups = workspace.lists()[0].groups();
    groups
        .list()
        .cues
        .iter()
        .map(|cue| column.text(workspace, &groups, cue))
        .collect()
}

#[test]
fn a_group_tells_how_many_cues_it_holds_and_how_long_its_action_lasts() {
    let workspace = grouped(1);
    assert_eq!(
        texts(&workspace, Column::Target),
        ["1 cue", "rain.ogg", "0 cues"]
    );
    assert_eq!(texts(&workspace, Column::Action), ["0:11.0", "0:10.0", "-"]);
}

#[test]
fn the_action_of_a_group_holding_a_cue_that_never_ends_is_a_hold() {
    let workspace = grouped(1).apply(Command::Edit(CueId(2), Edit::Loop(true)));
    assert_eq!(texts(&workspace, Column::Action)[0], "hold");
}

#[test]
fn a_cue_in_a_group_shows_no_post_wait_and_no_then_setting() {
    let workspace = grouped(1);
    assert_eq!(texts(&workspace, Column::Post), ["", "", ""]);
    assert_eq!(texts(&workspace, Column::Then), ["", "", ""]);
    let outside = grouped(0);
    assert_eq!(texts(&outside, Column::Then)[1], "continue");
    assert_eq!(texts(&outside, Column::Post)[1], "0:02.0");
}

#[test]
fn a_start_cue_in_the_group_that_it_starts_reads_no_target() {
    let group = Cue::new(CueId(1), "1", Kind::Group, "Group").build();
    let start = Cue::new(CueId(2), "2", Kind::Start, "Start")
        .target(CueId(1))
        .depth(1)
        .build();
    let workspace = Workspace::new(vec![CueList::new("Main", vec![group, start])]);
    let list = &workspace.lists()[0];
    let (state, _) = state(&workspace, &list.groups(), &list.cues[1], false);
    assert_eq!(state, "NO TARGET");
}
