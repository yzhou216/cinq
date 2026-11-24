// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hint of the key that opens a show, which names the first step
//! while no show is open

use super::{
    super::{order::yielding, status},
    cancel::COLUMNS,
    fitted, hinted, opened, opened_with,
};
use crate::{
    back::{Cue, CueId, CueList, Kind, Workspace},
    front::{
        App,
        keymap::{Keymap, desk},
        update::Show,
    },
};
use unicode_width::UnicodeWidthStr;

fn told(app: &App, hint: &str) -> bool {
    hinted(app).iter().any(|(_, told)| told == hint)
}

#[test]
fn the_key_of_opening_a_show_is_hinted_while_the_show_has_no_file_and_no_cues() {
    assert!(told(&opened(false, false), "Control+l: open"));
}

#[test]
fn the_key_of_opening_a_show_is_not_hinted_once_a_file_or_a_cue_exists() {
    let with_file = App {
        show: Show::in_file("/shows/storm.cinq.toml"),
        ..opened(false, false)
    };
    let with_cue = App {
        workspace: Workspace::new(vec![CueList::new(
            "Main",
            vec![Cue::new(CueId(1), "1", Kind::Memo, "Note").build()],
        )]),
        ..opened(false, false)
    };
    assert!(!told(&with_file, "Control+l: open"));
    assert!(!told(&with_cue, "Control+l: open"));
}

#[test]
fn the_key_of_opening_a_show_is_not_hinted_where_a_field_or_the_outputs_have_the_keys() {
    let app = |held, outputs_shown| App {
        held,
        outputs_shown,
        ..opened(false, false)
    };
    assert!(!told(&app(true, false), "Control+l: open"));
    assert!(!told(&app(false, true), "Control+l: open"));
}

#[test]
fn the_key_of_opening_a_show_fits_a_bar_of_1024_pixels_with_and_without_a_key_to_cancel() {
    [desk(""), Keymap::default()]
        .into_iter()
        .for_each(|keymap| {
            let app = opened_with(keymap, false, false);
            let taken: usize = status(&app, app.show.name().into_owned())
                .iter()
                .map(|part| part.text.width())
                .sum();
            let shown = fitted(&hinted(&app), COLUMNS - taken - 2, yielding(&app));
            assert!(shown.contains(": open"), "{shown}");
        });
}
