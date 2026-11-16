// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of how the hints of back and forward name the place they go to

use super::super::place;
use crate::{
    back::{Cue, CueId, CueList, Kind, Workspace},
    front::{App, update::Spot},
};

fn on_show() -> App {
    let memo = |id: u32| Cue::new(CueId(id), id.to_string(), Kind::Memo, "Note").build();
    App {
        workspace: Workspace::new(vec![
            CueList::new("Main", vec![memo(1), memo(2)]),
            CueList::new("A cue list with a very long name", vec![memo(3)]),
        ]),
        ..App::default()
    }
}

#[test]
fn a_cue_of_the_list_on_show_is_named_by_its_number_and_any_other_by_its_list() {
    let app = on_show();
    assert_eq!(place(&app, Spot::Cue(CueId(2))), "cue 2");
    assert_eq!(
        place(&app, Spot::Cue(CueId(3))),
        "A cue list with a very long name"
    );
    assert_eq!(place(&app, Spot::List(0)), "Main");
}
