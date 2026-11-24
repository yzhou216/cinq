// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hint of the key that opens a show, which names the first step
//! while no show is open

use super::{
    super::{order::yielding, status},
    cancel::COLUMNS,
    fitted, hinted, opened,
};
use crate::{
    back::{Cue, CueId, CueList, Kind, Workspace},
    front::{App, keymap::Preset, update::Show},
};
use unicode_width::UnicodeWidthStr;

fn told(app: &App, hint: &str) -> bool {
    hinted(app).iter().any(|(_, told)| told == hint)
}

#[test]
fn the_key_of_opening_a_show_is_hinted_while_the_show_has_no_file_and_no_cues() {
    assert!(told(&opened(Preset::Cua, false, false), "Control+o: open"));
    assert!(told(&opened(Preset::Vi, false, false), "Control+l: open"));
}

#[test]
fn the_key_of_opening_a_show_is_not_hinted_once_a_file_or_a_cue_exists() {
    let with_file = App {
        show: Show::in_file("/shows/storm.cinq.toml"),
        ..opened(Preset::Cua, false, false)
    };
    let with_cue = App {
        workspace: Workspace::new(vec![CueList::new(
            "Main",
            vec![Cue::new(CueId(1), "1", Kind::Memo, "Note").build()],
        )]),
        ..opened(Preset::Cua, false, false)
    };
    assert!(!told(&with_file, "Control+o: open"));
    assert!(!told(&with_cue, "Control+o: open"));
}

#[test]
fn the_key_of_opening_a_show_is_not_hinted_where_a_field_or_the_outputs_have_the_keys() {
    let app = |held, outputs_shown| App {
        held,
        outputs_shown,
        ..opened(Preset::Cua, false, false)
    };
    assert!(!told(&app(true, false), "Control+o: open"));
    assert!(!told(&app(false, true), "Control+o: open"));
}

#[test]
fn the_key_of_opening_a_show_fits_a_bar_of_1024_pixels_in_both_keymaps() {
    [Preset::Cua, Preset::Vi].into_iter().for_each(|preset| {
        let app = opened(preset, false, false);
        let taken: usize = status(&app, app.show.name().into_owned())
            .iter()
            .map(|part| part.text.width())
            .sum();
        let shown = fitted(&hinted(&app), COLUMNS - taken - 2, yielding(&app));
        assert!(shown.contains(": open"), "{shown}");
    });
}
