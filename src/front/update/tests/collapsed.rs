// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the keys and counts that move through the rows of a list with a
//! collapsed group, which count the rows that the table shows, and of the
//! selection, which a collapsed group never hides

use super::{operators::rows, *};
use crate::back::{Command, Cue, Kind};

/// Returns a show of one list with the group 2 collapsed: audio cues 1, 7,
/// and 8, and the group 2, which holds 3, the group 4 with 5 in it, and 6
pub(super) fn collapsed(selected: u32) -> App {
    let cue = |id: u32, depth: u8, kind: Kind, name: &str| Cue {
        depth,
        ..Cue::new(CueId(id), id.to_string(), kind, name).build()
    };
    let cues = vec![
        cue(1, 0, Kind::Audio, "Rain"),
        Cue {
            collapsed: true,
            ..cue(2, 0, Kind::Group, "Storm")
        },
        cue(3, 1, Kind::Audio, "Thunder"),
        cue(4, 1, Kind::Group, "Wind"),
        cue(5, 2, Kind::Audio, "Gust"),
        cue(6, 1, Kind::Audio, "Lightning"),
        cue(7, 0, Kind::Audio, "Birds"),
        cue(8, 0, Kind::Audio, "Dawn"),
    ];
    App {
        workspace: Workspace::new(vec![CueList::new("Main", cues)]),
        selected: Some(CueId(selected)),
        ..App::default()
    }
}

pub(super) fn selected(app: &App) -> Option<u32> {
    app.selected.map(|id| id.0)
}

pub(super) fn closed(app: &App, id: u32) -> bool {
    app.workspace
        .cue(CueId(id))
        .is_some_and(|cue| cue.collapsed)
}

#[test]
fn the_table_shows_one_row_for_a_collapsed_group() {
    let app = collapsed(1);
    assert_eq!(app.row_count(), 4);
    assert_eq!(app.row(), Some(0));
    assert_eq!(app.selecting(Some(2)).selected, Some(CueId(7)));
    assert_eq!(collapsed(2).row(), Some(1));
    assert_eq!(collapsed(7).row(), Some(2));
}

#[test]
fn a_cue_that_the_group_hides_is_on_the_row_of_the_group() {
    assert_eq!(collapsed(5).row(), Some(1));
}

#[test]
fn the_keys_that_step_and_jump_count_rows_that_are_shown() {
    assert_eq!(selected(&typing(collapsed(1), "j")), Some(2));
    assert_eq!(selected(&typing(collapsed(1), "2 j")), Some(7));
    assert_eq!(selected(&typing(collapsed(8), "2 k")), Some(2));
    assert_eq!(selected(&typing(collapsed(1), "G")), Some(8));
    assert_eq!(selected(&typing(collapsed(1), "3 G")), Some(7));
    assert_eq!(selected(&typing(collapsed(1), "2 G")), Some(2));
    assert_eq!(selected(&typing(collapsed(8), "g g")), Some(1));
    assert_eq!(selected(&typing(collapsed(1), "9 G")), Some(8));
}

#[test]
fn deleting_a_collapsed_group_takes_every_cue_that_it_holds() {
    let app = typing(collapsed(2), "d d");
    assert_eq!(rows(&app), [1, 7, 8]);
    assert_eq!(selected(&app), Some(7));
}

#[test]
fn a_count_or_a_motion_takes_the_rows_that_are_shown() {
    assert_eq!(rows(&typing(collapsed(1), "2 d d")), [7, 8]);
    assert_eq!(rows(&typing(collapsed(1), "d 2 j")), [8]);
    assert_eq!(rows(&typing(collapsed(2), "d G")), [1]);
    assert_eq!(rows(&typing(collapsed(8), "d 2 k")), [1]);
    assert_eq!(rows(&typing(collapsed(1), "d 3 G")), [8]);
}

#[test]
fn copying_and_cutting_a_collapsed_group_take_every_cue_that_it_holds() {
    let copied = typing(collapsed(1), "2 y y");
    let held: Vec<_> = copied.clipboard.cues.iter().map(|cue| cue.id.0).collect();
    assert_eq!(held, [1, 2, 3, 4, 5, 6]);
    let cut = typing(collapsed(2), "x");
    assert_eq!(rows(&cut), [1, 7, 8]);
    assert_eq!(cut.clipboard.cues.len(), 5);
}

#[test]
fn indenting_the_rows_counts_the_group_as_one() {
    // The cue below the group goes in as its last cue, and the group opens
    let app = typing(collapsed(7), "> >");
    assert_eq!(app.workspace.lists()[0].depths(), [0, 0, 1, 1, 2, 1, 1, 0]);
    assert_eq!(selected(&app), Some(7));
    assert!(!closed(&app, 2));
    // A count takes the rows below as well
    let nested = typing(collapsed(7), "2 > >");
    assert_eq!(
        nested.workspace.lists()[0].depths(),
        [0, 0, 1, 1, 2, 1, 1, 1]
    );
}

#[test]
fn a_move_that_takes_the_selection_into_a_collapsed_group_expands_it() {
    let app = typing(collapsed(7), "> >");
    assert!(!closed(&app, 2));
    let outer = typing(collapsed(1), "> >");
    assert!(closed(&outer, 2));
}

#[test]
fn collapsing_and_expanding_keep_the_selection_on_a_shown_row() {
    let hidden = after(
        collapsed(5),
        [Message::Fold(crate::front::message::Fold::OpenAll)],
    );
    assert_eq!(selected(&hidden), Some(5));
    let again = after(
        hidden,
        [Message::Fold(crate::front::message::Fold::CloseAll)],
    );
    assert_eq!(selected(&again), Some(2));
    assert_eq!(again.row(), Some(1));
}

#[test]
fn selecting_a_cue_that_a_group_hides_expands_the_groups_that_hide_it() {
    let nested = after(
        collapsed(1),
        [Message::Fold(crate::front::message::Fold::CloseAll)],
    );
    assert!(closed(&nested, 2) && closed(&nested, 4));
    let app = after(nested, [Message::Select(CueId(5))]);
    assert_eq!(selected(&app), Some(5));
    assert!(!closed(&app, 2) && !closed(&app, 4));
    assert_eq!(app.row(), Some(4));
}

#[test]
fn undo_that_selects_a_hidden_cue_expands_its_group() {
    let deleted = typing(collapsed(1), "j z o j d d");
    assert_eq!(rows(&deleted), [1, 2, 4, 5, 6, 7, 8]);
    let folded = after(
        deleted,
        [Message::Fold(crate::front::message::Fold::CloseAll)],
    );
    assert!(closed(&folded, 2));
    let undone = after(folded, [Message::Undo(1)]);
    assert_eq!(selected(&undone), Some(3));
    assert!(!closed(&undone, 2));
}

#[test]
fn the_jump_list_expands_the_group_that_hides_where_it_goes() {
    let open = after(
        collapsed(1),
        [Message::Fold(crate::front::message::Fold::OpenAll)],
    );
    let jumped = typing(typing(open, "5 G"), "g g");
    assert_eq!(selected(&jumped), Some(1));
    let folded = after(
        jumped,
        [Message::Fold(crate::front::message::Fold::CloseAll)],
    );
    let back = typing(folded, "Control+o");
    assert_eq!(selected(&back), Some(5));
    assert!(!closed(&back, 2) && !closed(&back, 4));
}

#[test]
fn go_that_moves_the_selection_with_standby_into_a_group_expands_it() {
    let target = |app: App| {
        let goto = Cue::new(CueId(9), "9", Kind::GoTo, "Back")
            .target(CueId(5))
            .build();
        let list = CueList::new(
            "Main",
            app.workspace.lists()[0]
                .cues
                .iter()
                .cloned()
                .chain([goto])
                .collect(),
        );
        App {
            workspace: Workspace::new(vec![list]),
            selected: Some(CueId(9)),
            ..app
        }
        .send(Command::Standby {
            list: 0,
            cue: Some(CueId(9)),
        })
    };
    let app = after(target(collapsed(9)), [Message::Go]);
    assert_eq!(selected(&app), Some(5));
    assert!(!closed(&app, 2));
}

#[test]
fn standby_may_stay_in_a_collapsed_group_with_no_selection_in_it() {
    let app = collapsed(1).send(Command::Standby {
        list: 0,
        cue: Some(CueId(5)),
    });
    assert_eq!(app.current().standby, Some(CueId(5)));
    assert!(closed(&app, 2));
}

#[test]
fn a_search_finds_a_cue_that_a_group_hides_and_expands_the_group() {
    let app = collapsed(1).handled(Message::Search { backward: false }).0;
    let app = app.handled(Message::Searching("gust".to_owned())).0;
    assert_eq!(selected(&app), Some(5));
    assert!(!closed(&app, 2));
    let accepted = app.handled(Message::SearchAccepted).0;
    assert_eq!(selected(&accepted), Some(5));
}

#[test]
fn the_next_match_goes_through_the_cues_that_a_group_hides() {
    let searched = |text: &str| {
        let app = collapsed(1).handled(Message::Search { backward: false }).0;
        app.handled(Message::Searching(text.to_owned()))
            .0
            .handled(Message::SearchAccepted)
            .0
    };
    // Cues 3, 6, and 7 hold an r, as do the group 2 and the cue 1
    let first = searched("r");
    assert_eq!(selected(&first), Some(2));
    let next = first.handled(Message::SearchStep(1)).0;
    assert_eq!(selected(&next), Some(3));
    assert!(!closed(&next, 2));
}

#[test]
fn cancelling_a_search_gives_the_selection_back_to_where_it_began() {
    let app = collapsed(7).handled(Message::Search { backward: true }).0;
    let found = app.handled(Message::Searching("gust".to_owned())).0;
    assert_eq!(selected(&found), Some(5));
    let cancelled = found.search_cancelled();
    assert_eq!(selected(&cancelled), Some(7));
}

#[test]
fn a_page_or_a_scroll_lands_on_a_row_that_is_shown() {
    let from = collapsed(1).spot();
    let landed = after(collapsed(1), [Message::Landed { from, row: 1 }]);
    assert_eq!(selected(&landed), Some(2));
    let reached = after(collapsed(1), [Message::Reached { from, row: 3 }]);
    assert_eq!(selected(&reached), Some(8));
}

#[test]
fn a_row_number_counts_the_rows_that_are_shown() {
    assert_eq!(selected(&after(collapsed(1), [Message::Row(3)])), Some(7));
    assert_eq!(selected(&after(collapsed(1), [Message::Row(99)])), Some(8));
}
