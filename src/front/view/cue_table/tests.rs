// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the State column of the cue list and the columns that fit

use iced::widget::text;
use strum::VariantArray;

use super::{CELL_SPACE, Column, NAME_ROOM, fitting, hidden_problem, layout::Layout, state};
use crate::{
    back::{Advance, Command, Cue, CueId, CueList, Edit, Kind, Media, OutputId, Sends, Workspace},
    front::{
        App,
        view::{char_width, theme},
    },
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
    let (state, style) = state(workspace, cue, list.standby == Some(cue.id), None, false);
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
    let (state, _) = state(&workspace, &list.cues[1], false, None, false);
    assert_eq!(state, "NO TARGET");
}

#[test]
fn the_state_of_a_row_that_stands_for_the_standby_cue_reads_next() {
    let workspace = show(None);
    let cue = &workspace.lists()[0].cues[0];
    assert_eq!(state(&workspace, cue, true, None, false).0, "NEXT");
    assert_eq!(state(&workspace, cue, false, None, false).0, "");
}

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
    let (state, style) = state(workspace, cue, false, hidden, false);
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
fn the_problem_of_a_hidden_cue_comes_before_next() {
    let workspace = troubled(true);
    let list = &workspace.lists()[0];
    let cue = &list.cues[0];
    let hidden = hidden_problem(&workspace, &list.groups(), cue);
    assert_eq!(state(&workspace, cue, true, hidden, false).0, "NO FILE");
    assert_eq!(state(&workspace, cue, true, None, false).0, "NEXT");
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
    assert_eq!(texts(&workspace, Column::State)[0], "NO FILE");
    assert_eq!(texts(&troubled(false), Column::State)[0], "NEXT");
}

/// Returns the number of columns that a list of one cue of `kind` keeps in a
/// room that leaves its name just [`NAME_ROOM`] columns when none gives way
fn columns_kept(kind: Kind) -> usize {
    let cue = Cue::new(CueId(1), "1", kind, "Cue").build();
    let app = App {
        workspace: Workspace::new(vec![CueList::new("Main", vec![cue])]),
        ..App::default()
    };
    let layout = Layout::of(app.current());
    let taken: f32 = Column::VARIANTS
        .iter()
        .map(|column| column.width(&app.workspace, &layout.groups).unwrap_or(0.0) + CELL_SPACE)
        .sum();
    let room = taken + NAME_ROOM as f32 * char_width();
    fitting(&app, &layout, room).len()
}

#[test]
fn a_list_with_a_group_keeps_the_gutter_of_the_mark_out_of_the_room_of_a_name() {
    assert_eq!(columns_kept(Kind::Light), Column::VARIANTS.len());
    assert!(columns_kept(Kind::Group) < Column::VARIANTS.len());
}
