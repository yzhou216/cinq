// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of a search matching the text of every column that the list shows
//! of a cue, as Vim matches the whole line, but for the state

use super::search::searched;
use crate::{
    back::{Advance, Command, Cue, CueId, CueList, Kind, Media, Workspace},
    front::{App, Message},
};

/// Returns the app with the cue list on screen, the first of five cues
/// selected, and a search line open: a sound with a file, a fade of it that
/// continues, a pause that waits before it starts, a memo, and a stop
fn listed() -> App {
    let sound = Cue::new(CueId(1), "A1", Kind::Audio, "Thunder")
        .file("storm/rumble.ogg")
        .build();
    let fade = Cue {
        advance: Advance::Continue,
        ..Cue::new(CueId(2), "A2", Kind::Fade, "Out")
            .target(CueId(1))
            .build()
    };
    let waiting = Cue {
        pre_wait: 3.0.into(),
        post_wait: 4.5.into(),
        ..Cue::new(CueId(3), "A3", Kind::Memo, "Note").build()
    };
    let stop = Cue::new(CueId(4), "A4", Kind::Stop, "End")
        .target(CueId(1))
        .build();
    let workspace = Workspace::new(vec![CueList::new("Main", vec![sound, fade, waiting, stop])])
        .apply(Command::Loaded {
            cue: CueId(1),
            file: "storm/rumble.ogg".to_owned(),
            media: Media::Loaded {
                duration: 12.0,
                sound: Some(12.0),
            },
        });
    App {
        workspace,
        selected: Some(CueId(4)),
        ..App::default()
    }
    .handled(Message::Search { backward: false })
    .0
}

fn found(text: &str) -> Option<u32> {
    searched(listed(), text).selected.map(|id| id.0)
}

#[test]
fn a_search_matches_the_number_type_name_target_action_and_then_of_a_cue() {
    // Each text lies in one column of one cue, and the search wraps to it
    assert_eq!(found("a2"), Some(2), "number");
    assert_eq!(found("audio"), Some(1), "type");
    assert_eq!(found("thunder"), Some(1), "name");
    assert_eq!(found("rumble"), Some(1), "target of a file");
    assert_eq!(found("ogg"), Some(1), "target of a file");
    assert_eq!(found("0:12"), Some(1), "action");
    assert_eq!(found("continue"), Some(2), "what follows");
    assert_eq!(found("fade"), Some(2), "type");
}

#[test]
fn a_search_leaves_out_the_waits_and_the_state() {
    // Selection stays on the last cue, which a miss leaves alone
    assert_eq!(found("0:03"), Some(4), "pre-wait");
    assert_eq!(found("0:04"), Some(4), "post-wait");
    // Cue 1 is the standby of its list, which the state column tells
    assert_eq!(found("next"), Some(4), "state");
}
