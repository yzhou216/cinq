// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the State column of the cue list and the columns that fit

use iced::widget::text;
use strum::VariantArray;

use super::{
    CELL_SPACE, Column, NAME_ROOM, columns::state, fitting, hidden_problem, layout::Layout, lines,
};
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
    let (state, _) = state(&workspace, &list.cues[1], None, false);
    assert_eq!(state, "NO TARGET");
}

#[test]
fn the_state_of_a_row_that_stands_for_the_standby_cue_reads_next() {
    let workspace = show(None);
    let cue = &workspace.lists()[0].cues[0];
    assert_eq!(standby_lines(&workspace), ["NEXT"]);
    assert!(lines(&workspace, cue, false, None, &[], false).is_empty());
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

/// Returns the number of columns that a list of one cue of `kind` shows and
/// the number that it keeps in a room that leaves its name just [`NAME_ROOM`]
/// columns when none gives way
fn columns_kept(kind: Kind) -> (usize, usize) {
    let cue = Cue::new(CueId(1), "1", kind, "Cue").build();
    let app = App {
        workspace: Workspace::new(vec![CueList::new("Main", vec![cue])]),
        ..App::default()
    };
    let layout = Layout::of(app.current());
    let shown: Vec<_> = Column::VARIANTS
        .iter()
        .filter(|column| column.shown(&app.workspace, &layout.groups))
        .collect();
    let taken: f32 = shown
        .iter()
        .map(|column| column.width(&app.workspace, &layout.groups).unwrap_or(0.0) + CELL_SPACE)
        .sum();
    let room = taken + NAME_ROOM as f32 * char_width();
    (shown.len(), fitting(&app, &layout, room).len())
}

#[test]
fn a_list_with_a_group_keeps_the_gutter_of_the_mark_out_of_the_room_of_a_name() {
    let (shown, kept) = columns_kept(Kind::Light);
    assert_eq!(kept, shown);
    let (shown, kept) = columns_kept(Kind::Group);
    assert!(kept < shown);
}

#[test]
fn a_column_that_holds_no_text_in_any_row_is_left_out() {
    let (shown, _) = columns_kept(Kind::Memo);
    assert!(shown < Column::VARIANTS.len());
}

/// Returns the width that the Target column takes for a file with a name of 40
/// characters in a list `room` wide, and the room that shows every column
/// whole and leaves a name just [`NAME_ROOM`] columns
fn target_width(room: impl Fn(f32) -> f32) -> Option<f32> {
    let cue = Cue::new(CueId(1), "1", Kind::Audio, "Rain")
        .file("x".repeat(36) + ".wav")
        .build();
    let app = App {
        workspace: Workspace::new(vec![CueList::new("Main", vec![cue])]),
        ..App::default()
    };
    let layout = Layout::of(app.current());
    let taken: f32 = Column::VARIANTS
        .iter()
        .filter(|column| column.shown(&app.workspace, &layout.groups))
        .map(|column| column.width(&app.workspace, &layout.groups).unwrap_or(0.0) + CELL_SPACE)
        .sum();
    fitting(&app, &layout, room(taken + NAME_ROOM as f32 * char_width()))
        .into_iter()
        .find_map(|(column, width)| (column == Column::Target).then_some(width).flatten())
}

#[test]
fn a_long_file_name_shows_whole_until_a_name_would_lack_room() {
    assert_eq!(target_width(|enough| enough), Some(40.0 * char_width()));
    assert_eq!(
        target_width(|enough| enough - char_width()),
        Some(24.0 * char_width())
    );
}

#[test]
fn a_collapsed_group_tells_its_count_in_the_target_column_and_leaves_its_name_bare() {
    let workspace = grouped(1).apply(Command::Collapse {
        cues: vec![CueId(1)],
        collapsed: true,
    });
    assert_eq!(texts(&workspace, Column::Name)[0], "Group");
    assert_eq!(texts(&workspace, Column::Target)[0], "1 cue");
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

#[test]
fn a_cue_whose_outputs_all_failed_reads_in_yellow_that_the_output_failed() {
    let read = |workspace: &Workspace, failing: &[OutputId]| {
        let cue = &workspace.lists()[0].cues[0];
        let theme = theme::modus_vivendi();
        lines(workspace, cue, false, None, failing, false)
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
        lines(workspace, cue, false, None, &[OutputId(1)], false)
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
    let next = lines(&workspace, cue, true, None, &[OutputId(1)], false);
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
    });
    let cue = &broken.lists()[0].cues[0];
    let broken = lines(&broken, cue, false, None, &[OutputId(1)], false);
    assert_eq!(
        broken
            .into_iter()
            .map(|(text_of, _)| text_of)
            .collect::<Vec<_>>(),
        ["BROKEN"]
    );
}
