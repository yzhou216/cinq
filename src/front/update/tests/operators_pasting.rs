// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of pasting what the operators cut and copied over a motion

use super::{
    clipboard::{fading, targets},
    operators::{across, rows},
    *,
};
use crate::front::keymap::Action::{Cut, Last};

#[test]
fn cutting_a_fade_with_its_target_over_a_motion_remaps_the_target_where_it_is_pasted() {
    let app = after(
        typing(fading(), "d j"),
        [Message::Select(CueId(3)), Message::Paste(Place::After, 1)],
    );
    assert_eq!(targets(&app), [(3, None), (4, None), (5, Some(4))]);
    let app = typing(app, "u");
    assert_eq!(targets(&app), [(3, None)]);
}

#[test]
fn copying_a_fade_with_its_target_over_a_motion_remaps_the_target_and_keeps_the_old_one() {
    let app = after(
        typing(fading(), "y j"),
        [Message::Select(CueId(3)), Message::Paste(Place::After, 1)],
    );
    let expected = [(1, None), (2, Some(1)), (3, None), (4, None), (5, Some(4))];
    assert_eq!(targets(&app), expected);
}

#[test]
fn cutting_over_a_motion_pastes_at_most_a_thousand_cues_like_any_cut() {
    let cues = (1..=1500).map(audio).collect();
    let app = App {
        workspace: Workspace::new(vec![CueList::new("Main", cues)]),
        selected: Some(CueId(1)),
        ..App::default()
    };
    let app = across(app, Cut, Last, None)
        .handled(Message::Paste(Place::After, 2))
        .0;
    assert_eq!(rows(&app).len(), 1000);
}
