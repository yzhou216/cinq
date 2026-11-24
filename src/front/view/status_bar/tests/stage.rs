// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hints about the stage window

use strum::VariantArray;

use super::{
    super::hints::{acts_here, notices},
    hinted, opened,
};
use crate::{
    back::{Command, Cue, CueId, CueList, Kind, Place, Workspace, loaded, silent},
    front::{
        App,
        keymap::{Action, Preset},
        update::Stage,
    },
};

/// Returns the state with a video cue that shows once `go` fires it, and a
/// stage window that the operator closed
fn closed(preset: Preset, go: bool) -> App {
    let cue = Cue::new(CueId(1), "1", Kind::Video, "Film")
        .file("film.mp4")
        .build();
    let workspace = Workspace::new(vec![CueList::new("Main", vec![cue])]).apply(loaded(
        1,
        "film.mp4",
        silent(10.0),
    ));
    let workspace = if go {
        workspace.apply(Command::Go { list: 0 })
    } else {
        workspace
    };
    App {
        workspace,
        stage: Stage::closed(),
        ..opened(preset, false, false)
    }
}

fn told(app: &App) -> Vec<String> {
    hinted(app).into_iter().map(|(_, hint)| hint).collect()
}

#[test]
fn a_closed_stage_window_that_a_cue_shows_on_hints_the_key_that_opens_it() {
    assert!(told(&closed(Preset::Vi, true)).contains(&"F11: open stage".to_owned()));
    assert!(told(&closed(Preset::Cua, true)).contains(&"F11: open stage".to_owned()));
}

#[test]
fn a_closed_stage_window_with_nothing_to_show_hints_nothing() {
    assert!(!told(&closed(Preset::Vi, false)).contains(&"F11: open stage".to_owned()));
    let open = App {
        stage: Stage::default(),
        ..closed(Preset::Vi, true)
    };
    assert!(!told(&open).contains(&"F11: open stage".to_owned()));
}

#[test]
fn the_hint_of_opening_the_stage_window_stays_off_the_help_page() {
    let app = App {
        help: true,
        ..closed(Preset::Vi, true)
    };
    assert!(!told(&app).contains(&"F11: open stage".to_owned()));
}

#[test]
fn a_closed_stage_window_that_a_cue_shows_on_counts_as_a_warning() {
    let counted = |app: &App| crate::front::view::warnings(app);
    let nothing = counted(&closed(Preset::Vi, false));
    assert_eq!(counted(&closed(Preset::Vi, true)), nothing + 1);
    let open = App {
        stage: Stage::default(),
        ..closed(Preset::Vi, true)
    };
    assert_eq!(counted(&open), nothing);
}

fn keyboard(preset: Preset) -> App {
    App {
        stage: Stage::holding_keyboard(),
        ..opened(preset, false, false)
    }
}

#[test]
fn the_stage_window_with_the_keyboard_gets_only_the_hints_of_keys_that_act_there() {
    [Preset::Vi, Preset::Cua].into_iter().for_each(|preset| {
        let hints = hinted(&App {
            workspace: Workspace::default().apply(Command::Add {
                list: 0,
                kind: Kind::Audio,
                at: Place::End,
                count: 1,
            }),
            ..keyboard(preset)
        });
        let words = hints
            .iter()
            .map(|(_, hint)| hint.as_str())
            .collect::<Vec<_>>();
        assert!(words.contains(&"F11: fullscreen"), "{words:?}");
        assert!(words.contains(&"Space: GO"), "{words:?}");
        assert!(
            hints
                .iter()
                .filter_map(|(action, _)| *action)
                .all(|action| !action.stays_in_main_window()),
            "{words:?}"
        );
        assert!(words.iter().all(|hint| !hint.contains("undo")), "{words:?}");
    });
}

#[test]
fn the_notice_of_a_change_names_no_undo_while_the_stage_window_has_the_keyboard() {
    let app = |stage| App {
        workspace: Workspace::default().apply(Command::Add {
            list: 0,
            kind: Kind::Audio,
            at: Place::End,
            count: 1,
        }),
        stage,
        ..opened(Preset::Vi, false, false)
    };
    let notice = |app: &App| notices::notice(app).expect("a change was made");
    assert!(notice(&app(Stage::default())).contains("u: undo"));
    assert!(!notice(&app(Stage::holding_keyboard())).contains("undo"));
}

#[test]
fn the_hints_and_the_refusals_agree_on_the_keys_that_act_in_the_stage_window() {
    let app = keyboard(Preset::Vi);
    Action::VARIANTS.iter().for_each(|action| {
        assert_eq!(
            acts_here(&app, *action),
            app.refusal(*action).is_none(),
            "{action:?}"
        );
    });
}
