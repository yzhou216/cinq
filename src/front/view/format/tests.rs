// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the text that describes cues

use super::{change, clipped, target};
use crate::back::{Command, Cue, CueId, CueList, Kind, Place, Workspace};

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
