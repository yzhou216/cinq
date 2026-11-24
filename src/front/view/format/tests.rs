// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the text that describes cues

use super::{activity, chain, change, clipped, moving, target};
use crate::back::{
    Advance, Command, Cue, CueId, CueList, Edit, InstrumentId, Kind, OutputId, Place, Sends,
    Workspace, loaded, sound,
};

fn memo(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Memo, "Note").build()
}

/// Returns what the latest change to `workspace` says of itself
fn told(workspace: &Workspace) -> String {
    let (_, latest) = workspace.latest_change().expect("a command is a change");
    change(workspace, latest)
}

#[test]
fn target_goes_by_its_number_or_else_by_its_quoted_name() {
    let fade = |target: u32| {
        Cue::new(CueId(10 + target), "10", Kind::Fade, "Fade")
            .target(CueId(target))
            .build()
    };
    let numbered = Cue::new(CueId(1), "1", Kind::Audio, "Rain").build();
    let unnumbered = Cue::new(CueId(2), "", Kind::Audio, "Thunder").build();
    let workspace = Workspace::new(vec![CueList::new("Main", vec![numbered, unnumbered])]);
    let groups = &workspace.lists()[0].groups();
    assert_eq!(target(&workspace, groups, &fade(1)), "1");
    assert_eq!(target(&workspace, groups, &fade(2)), "\"Thunder\"");
    assert_eq!(target(&workspace, groups, &fade(3)), "");
}

#[test]
fn copied_and_cut_cues_are_told_by_the_number_of_the_cue_or_else_by_how_many() {
    assert_eq!(clipped("Copied", &[memo(4)]), "Copied cue 4");
    assert_eq!(clipped("Cut", &[memo(1), memo(2), memo(3)]), "Cut 3 cues");
}

#[test]
fn changed_cues_are_told_by_the_number_of_the_one_or_else_by_how_many() {
    let shown = Workspace::new(vec![CueList::new("Main", vec![memo(1), memo(2), memo(3)])]);
    let after = |command| told(&shown.clone().apply(command));
    let add = |count| Command::Add {
        list: 0,
        kind: Kind::Memo,
        at: Place::End,
        count,
    };
    let paste = |count| Command::Paste {
        list: 0,
        cues: vec![memo(1); count],
        at: Place::End,
    };
    let ids = |ids: [u32; 2]| ids.map(CueId).to_vec();
    assert_eq!(after(add(1)), "added cue 4");
    assert_eq!(after(add(3)), "added 3 cues");
    assert_eq!(after(paste(1)), "pasted cue 4");
    assert_eq!(after(paste(3)), "pasted 3 cues");
    assert_eq!(after(Command::Cut(vec![CueId(2)])), "cut cue 2");
    assert_eq!(after(Command::Cut(ids([1, 2]))), "cut 2 cues");
    assert_eq!(after(Command::Delete(vec![CueId(2)])), "deleted cue 2");
    assert_eq!(after(Command::Delete(ids([1, 2]))), "deleted 2 cues");
}

#[test]
fn a_chain_with_nothing_to_fire_says_so_and_names_each_cue_with_its_reason() {
    let audio = |id: u32| {
        Cue::new(CueId(id), id.to_string(), Kind::Audio, "Rain")
            .advance(Advance::Continue)
            .build()
    };

    let words = |cues: Vec<Cue>| {
        let workspace = Workspace::new(vec![CueList::new("Main", cues)]);
        let list = &workspace.lists()[0];
        let standby = workspace
            .cue(list.standby.expect("a standby cue"))
            .expect("it exists");
        chain(&workspace, list, standby, &[])
    };
    assert_eq!(
        words(vec![audio(1)]),
        "GO fires nothing: cue 1 (no file set)"
    );
    assert_eq!(
        words(vec![audio(1), audio(2)]),
        "GO fires nothing: cue 1 (no file set), cue 2 (no file set)"
    );
    assert_eq!(words(vec![memo(3)]), "GO fires 1 cue: 3");
}

#[test]
fn a_run_moved_below_silence_is_told_at_silence() {
    let cue = |id: u32, kind, level: f32| {
        Cue::new(CueId(id), id.to_string(), kind, "Rain")
            .advance(Advance::Continue)
            .file("sound.ogg")
            .target(CueId(1))
            .level(level)
            .duration(1.0)
            .stop_target(false)
            .build()
    };
    let workspace = Workspace::new(vec![CueList::new(
        "Main",
        vec![cue(1, Kind::Audio, 0.0), cue(2, Kind::Fade, -60.0)],
    )]);
    let loaded = loaded(1, "sound.ogg", sound(10.0));
    let ducked = [loaded, Command::Go { list: 0 }, Command::Tick(2.0)]
        .into_iter()
        .fold(workspace, Workspace::apply)
        .apply(Command::Edit(CueId(1), Edit::Level(-30.0)));
    let active = ducked.active_of(CueId(1)).expect("the cue plays");
    assert_eq!(active.level, -90.0);
    let cue = ducked.cue(CueId(1)).expect("the cue is there");
    assert_eq!(activity(active, cue), "0:08.0 left, -60.0 dB");
}

#[test]
fn loop_edits_are_told_by_the_part_of_the_loop_they_change() {
    let rain = Cue::new(CueId(1), "1", Kind::Audio, "Rain")
        .looping(true)
        .build();
    let workspace = Workspace::new(vec![CueList::new("Main", vec![rain])]);
    let after = |edit| told(&workspace.clone().apply(Command::Edit(CueId(1), edit)));
    assert_eq!(
        after(Edit::LoopStart(2.0.into())),
        "changed the loop start of cue 1"
    );
    assert_eq!(
        after(Edit::LoopEnd(6.0.into())),
        "changed the loop end of cue 1"
    );
}

#[test]
fn a_devamped_run_is_told_by_the_time_it_has_left_instead_of_looping() {
    let rain = Cue::new(CueId(1), "1", Kind::Audio, "Rain")
        .file("sound.ogg")
        .looping(true)
        .loop_start(2.0)
        .loop_end(6.0)
        .build();
    let devamp = Cue::new(CueId(2), "2", Kind::Devamp, "Leave rain")
        .target(CueId(1))
        .build();
    let workspace = [
        loaded(1, "sound.ogg", sound(10.0)),
        Command::Go { list: 0 },
        Command::Tick(1.0),
    ]
    .into_iter()
    .fold(
        Workspace::new(vec![CueList::new("Main", vec![rain, devamp])]),
        Workspace::apply,
    );
    let told = |workspace: &Workspace| {
        let cue = workspace.cue(CueId(1)).expect("the cue is there");
        activity(workspace.active_of(CueId(1)).expect("the cue plays"), cue)
    };
    assert_eq!(told(&workspace), "looping, +0.0 dB");
    let devamped = [Command::Go { list: 0 }, Command::Tick(1.0)]
        .into_iter()
        .fold(workspace, Workspace::apply);
    assert_eq!(told(&devamped), "0:08.0 left, +0.0 dB");
}

#[test]
fn a_chain_tells_a_cue_that_plays_in_silence_and_still_fires_it() {
    let audio = |id: u32, sends: Sends| Cue {
        sends: Some(sends),
        ..Cue::new(CueId(id), id.to_string(), Kind::Audio, "Rain")
            .file("sound.ogg")
            .advance(Advance::Continue)
            .build()
    };
    let workspace = Workspace::new(vec![CueList::new(
        "Main",
        vec![
            audio(1, [(OutputId(1), 0.0)].into()),
            audio(2, [(OutputId(9), 0.0)].into()),
        ],
    )]);
    let workspace = [1, 2]
        .into_iter()
        .map(|id| loaded(id, "sound.ogg", sound(10.0)))
        .fold(workspace, Workspace::apply);
    let list = &workspace.lists()[0];
    let standby = workspace.cue(CueId(1)).expect("it exists");
    assert_eq!(
        chain(&workspace, list, standby, &[]),
        "GO fires 2 cues: 1, 2 (+0.0s, silent: no output)"
    );
    assert_eq!(
        chain(&workspace, list, standby, &[OutputId(1)]),
        "GO fires 2 cues: 1 (silent: output failed), 2 (+0.0s, silent: no output)"
    );
}

#[test]
fn a_file_typed_into_an_unnamed_cue_is_told_by_a_short_notice() {
    let workspace = Workspace::new(vec![CueList::new("Main", vec![memo(1)])]).apply(Command::Edit(
        CueId(1),
        Edit::FileAndName {
            file: "knock.ogg".into(),
            name: "knock".into(),
        },
    ));
    assert_eq!(told(&workspace), "changed the file of cue 1");
}

#[test]
fn a_move_is_told_to_enter_or_leave_a_group_only_where_the_depth_changes() {
    let within = |cue: Cue| Cue { depth: 1, ..cue };
    let group = Cue::new(CueId(2), "2", Kind::Group, "Group").build();
    let shown = Workspace::new(vec![CueList::new(
        "Main",
        vec![memo(1), group, within(memo(3)), within(memo(4)), memo(5)],
    )]);
    let moved = |workspace: &Workspace, cue: u32, by: isize| {
        workspace.clone().apply(Command::Move {
            cue: CueId(cue),
            by,
        })
    };
    let into = moved(&shown, 1, 1);
    assert_eq!(told(&into), "moved cue 1 into a group");
    assert_eq!(told(&moved(&shown, 4, 1)), "moved cue 4 out of a group");
    assert_eq!(told(&moved(&shown, 3, 1)), "moved cue 3");
    let undone = into.apply(Command::Undo);
    let latest = undone.redoable().expect("the move can be redone");
    assert_eq!(change(&undone, latest), "moved cue 1 into a group");
}

#[test]
fn clearing_the_last_send_of_a_cue_is_told_as_playing_nowhere() {
    let rain = Cue::new(CueId(1), "1", Kind::Audio, "Rain").build();
    let workspace = Workspace::new(vec![CueList::new("Main", vec![rain])]);
    let after = |sends| {
        told(
            &workspace
                .clone()
                .apply(Command::Edit(CueId(1), Edit::Sends(sends))),
        )
    };
    assert_eq!(after(Some(Sends::new())), "changed cue 1 to play nowhere");
    assert_eq!(
        after(Some([(OutputId(1), -3.0)].into())),
        "changed where cue 1 plays"
    );
}

#[test]
fn a_chain_tells_what_each_control_cue_does_and_names_a_list_only_when_it_is_another() {
    let control = |id: u32, kind, target: u32| {
        Cue::new(CueId(id), id.to_string(), kind, "Control")
            .advance(Advance::Continue)
            .target(CueId(target))
            .build()
    };
    let effect = Cue::new(CueId(30), "E1", Kind::Memo, "Rain").build();
    let workspace = Workspace::new(vec![
        CueList::new(
            "Scene",
            vec![
                memo(1),
                control(12, Kind::Pause, 1),
                control(13, Kind::Stop, 1),
                control(15, Kind::Start, 30),
                control(16, Kind::GoTo, 1),
            ],
        ),
        CueList::new("Effects", vec![effect]),
    ])
    .apply(Command::Standby {
        list: 0,
        cue: Some(CueId(12)),
    });
    let list = &workspace.lists()[0];
    let standby = workspace.cue(CueId(12)).expect("it exists");
    assert_eq!(
        chain(&workspace, list, standby, &[]),
        "GO fires 4 cues: 12 (pause 1), 13 (+0.0s, stop 1), \
         15 (+0.0s, start E1 in Effects), 16 (+0.0s, GoTo 1)"
    );
}

#[test]
fn standby_about_to_move_is_told_by_the_number_of_its_target_and_the_seconds() {
    assert_eq!(moving(&memo(5), 2.94), "Standby moves to 5 in 2.9s");
}

#[test]
fn a_chain_tells_a_delay_of_a_minute_or_more_as_a_clock_as_the_moving_standby_does() {
    let held = Cue {
        post_wait: 70.0.into(),
        ..Cue::new(CueId(1), "1", Kind::Memo, "Note")
            .advance(Advance::Continue)
            .build()
    };
    let workspace = Workspace::new(vec![CueList::new("Main", vec![held, memo(2)])]);
    let list = &workspace.lists()[0];
    let standby = workspace.cue(CueId(1)).expect("it exists");
    assert_eq!(
        chain(&workspace, list, standby, &[]),
        "GO fires 2 cues: 1, 2 (+1:10.0)"
    );
}

#[test]
fn a_cue_added_below_a_group_or_one_of_its_cues_is_told_to_be_in_a_group() {
    let group = Cue::new(CueId(1), "1", Kind::Group, "Group").build();
    let held = Cue::new(CueId(2), "2", Kind::Memo, "Note").depth(1).build();
    let after = |cues: Vec<Cue>, at: CueId, count| {
        told(
            &Workspace::new(vec![CueList::new("Main", cues)]).apply(Command::Add {
                list: 0,
                kind: Kind::Memo,
                at: Place::After(at),
                count,
            }),
        )
    };
    assert_eq!(
        after(vec![group.clone()], CueId(1), 1),
        "added cue 2 in a group"
    );
    assert_eq!(
        after(vec![group.clone(), held.clone()], CueId(2), 2),
        "added 2 cues in a group"
    );
    assert_eq!(
        after(vec![group, held, memo(3)], CueId(3), 1),
        "added cue 4"
    );
}

#[test]
fn a_time_ahead_turns_to_a_clock_from_a_minute_on() {
    assert_eq!(moving(&memo(5), 75.0), "Standby moves to 5 in 1:15.0");
    assert_eq!(moving(&memo(5), 59.97), "Standby moves to 5 in 1:00.0");
}

#[test]
fn target_in_another_list_is_told_by_the_name_of_that_list() {
    let start = |number: &str, target: u32| {
        Cue::new(CueId(40 + target), number, Kind::Start, "Start")
            .target(CueId(target))
            .build()
    };
    let workspace = Workspace::new(vec![
        CueList::new(
            "Main",
            vec![memo(1), start("2", 1), start("3", 9), start("4", 99)],
        ),
        CueList::new("Effects", vec![memo(9)]),
    ]);
    let groups = &workspace.lists()[0].groups();
    let told = |id: u32| {
        let cue = workspace.cue(CueId(id)).expect("it exists");
        target(&workspace, groups, cue)
    };
    assert_eq!(told(41), "1");
    assert_eq!(told(49), "9 in Effects");
    assert_eq!(told(139), "");
}

#[test]
fn a_light_cue_counts_every_instrument_it_names_even_one_the_patch_lacks() {
    let light = |ids: &[u32]| {
        Cue::new(CueId(1), "1", Kind::Light, "Light")
            .lights(ids.iter().map(|&id| (InstrumentId(id), 50.0)).collect())
            .build()
    };
    let workspace = Workspace::new(vec![CueList::new("Main", vec![])]);
    let groups = &workspace.lists()[0].groups();
    assert_eq!(target(&workspace, groups, &light(&[99])), "1 instrument");
    assert_eq!(
        target(&workspace, groups, &light(&[1, 99])),
        "2 instruments"
    );
    assert_eq!(target(&workspace, groups, &light(&[])), "0 instruments");
}
