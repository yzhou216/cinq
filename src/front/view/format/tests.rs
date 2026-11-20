// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the text that describes cues

use super::{activity, chain, change, clipped, target};
use crate::back::{
    Advance, Command, Cue, CueId, CueList, Edit, Kind, Place, Workspace, loaded, sound,
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
    assert_eq!(target(&workspace, &fade(1)), "1");
    assert_eq!(target(&workspace, &fade(2)), "\"Thunder\"");
    assert_eq!(target(&workspace, &fade(3)), "");
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
        chain(&workspace, list, standby)
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
