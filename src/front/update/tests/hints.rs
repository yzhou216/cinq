// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the keys typed in hint mode: the letters that choose a place,
//! those that take a letter back or leave the mode, and the keys and clicks
//! that act as everywhere

use iced::{Point, Rectangle, Size, window};

use super::*;
use crate::front::view::{Act, Site, hint};

fn place(act: Act, x: f32) -> Site {
    let bounds = Rectangle::new(Point::new(x, 10.0), Size::new(20.0, 20.0));
    Site {
        shown: bounds,
        at: bounds,
        act,
    }
}

/// Returns the state with hint mode on, as the widgets told that these places
/// lie in the window, a list switch, the outputs, and a field, which the
/// labels `s`, `a`, and `d` stand for
fn labelled() -> App {
    let places = vec![
        place(Act::Send(Message::SwitchList(1)), 0.0),
        place(Act::Send(Message::ShowOutputs), 30.0),
        place(Act::Reach, 60.0),
    ];
    let (app, _) = show(Some(CueId(2))).hinting();
    app.hinted(places)
}

/// Returns the state with hint mode on over more places than letters, so that
/// every label has two letters, the first of them leading to the places
/// `Message::Row(n)` in the order of the letters
fn many_labelled() -> App {
    let places = (1..=20).map(|row| place(Act::Send(Message::Row(row)), 0.0));
    let (app, _) = show(Some(CueId(2))).hinting();
    app.hinted(places.collect())
}

/// Returns the letters of the labels that the layer shows
fn shown(app: &App) -> Vec<&str> {
    app.hints
        .as_ref()
        .map(|hints| hints.labels().map(|(letters, _)| letters).collect())
        .unwrap_or_default()
}

#[test]
fn the_action_turns_hint_mode_on_and_a_second_press_turns_it_off() {
    let (on, task) = show(None).hinting();
    assert!(on.hints.is_some());
    assert!(
        task.units() > 0,
        "the widgets are asked where the places lie"
    );
    let (off, task) = on.hinting();
    assert!(off.hints.is_none());
    assert_eq!(task.units(), 0);
}

#[test]
fn places_that_arrive_after_the_mode_went_off_get_no_labels() {
    let (on, _) = show(None).hinting();
    let off = on.without_hints();
    assert!(off.hinted(vec![place(Act::Reach, 0.0)]).hints.is_none());
    let none = show(None).hinting().0.hinted(Vec::new());
    assert!(none.hints.is_none(), "no place in view leaves the mode");
}

#[test]
fn each_label_shows_left_of_its_place_at_its_height() {
    let app = labelled();
    let corners: Vec<_> = app.hints.iter().flat_map(|hints| hints.labels()).collect();
    let left_of = |x| {
        hint::corner(
            Rectangle::new(Point::new(x, 10.0), Size::new(20.0, 20.0)),
            1,
        )
    };
    assert_eq!(
        corners,
        [
            ("s", left_of(0.0)),
            ("a", left_of(30.0)),
            ("d", left_of(60.0)),
        ]
    );
    assert_eq!(
        corners[0].1,
        Point::new(20.0, 10.0),
        "right of the place that lies at the window edge"
    );
    assert!(corners[1].1.x < 30.0 && corners[2].1.x < 60.0);
}

#[test]
fn a_label_typed_whole_sends_the_message_of_its_place_and_leaves_the_mode() {
    let chosen = typing(labelled(), "a");
    assert!(chosen.hints.is_none());
    assert!(chosen.outputs_shown);
    let switched = typing(labelled(), "s");
    assert!(switched.hints.is_none());
    assert_eq!(switched.list, 1);
}

#[test]
fn a_label_that_reaches_a_field_starts_a_task_and_leaves_the_mode() {
    let (reached, task) = typing_task(labelled(), "d");
    assert!(reached.hints.is_none());
    assert!(task.units() > 0);
}

#[test]
fn letters_narrow_the_labels_until_one_is_whole_and_backspace_takes_one_back() {
    let begun = typing(many_labelled(), "s");
    assert_eq!(begun.hints.as_ref().map(|hints| hints.typed()), Some("s"));
    assert!(!begun.outputs_shown);
    assert_eq!(
        shown(&begun),
        [
            "s", "a", "d", "f", "j", "k", "l", "e", "w", "c", "m", "p", "g", "h"
        ],
        "the labels that begin with `s` show the letter that follows"
    );
    assert_eq!(shown(&many_labelled()).len(), 20);
    let taken_back = typing(many_labelled(), "s Backspace");
    assert_eq!(
        taken_back.hints.as_ref().map(|hints| hints.typed()),
        Some("")
    );
    // Second letter makes the label whole: the third place, `s d`
    let whole = typing(many_labelled(), "s d");
    assert!(whole.hints.is_none());
    assert_eq!(
        whole.selected,
        Some(CueId(3)),
        "the third place selects row 3"
    );
}

#[test]
fn backspace_with_nothing_typed_leaves_the_mode() {
    assert!(typing(labelled(), "Backspace").hints.is_none());
    assert!(
        typing(many_labelled(), "s Backspace Backspace")
            .hints
            .is_none()
    );
}

#[test]
fn a_character_that_no_label_continues_is_ignored_and_the_mode_stays() {
    let after = typing(labelled(), "k");
    assert!(after.hints.is_some());
    assert_eq!(after.selected, Some(CueId(2)));
    assert_eq!(after.list, 0);
    assert!(!after.outputs_shown);
    let after = typing(many_labelled(), "a h");
    assert_eq!(
        shown(&after),
        shown(&typing(many_labelled(), "a")),
        "`a` stays typed, and `ah` begins none"
    );
    assert_eq!(after.list, 0);
    let after = typing(many_labelled(), "d d 5");
    assert!(after.hints.is_some(), "a slip never runs the next keys");
}

#[test]
fn a_key_that_is_no_letter_leaves_the_mode_and_does_nothing_else() {
    ["ArrowDown", "Control+c", "Tab", "Enter", "Control+d"]
        .into_iter()
        .for_each(|keys| {
            let after = typing(labelled(), keys);
            assert!(after.hints.is_none(), "{keys}");
            assert_eq!(after.selected, Some(CueId(2)), "{keys}");
            assert!(!after.outputs_shown, "{keys}");
        });
}

#[test]
fn go_and_panic_act_as_everywhere_and_leave_the_mode() {
    let panicked = typing(
        App {
            help: true,
            ..labelled()
        },
        "Escape",
    );
    assert!(panicked.hints.is_none());
    assert!(!panicked.help, "Panic closes the help page");
    assert!(typing(labelled(), "Space").hints.is_none());
}

#[test]
fn a_held_letter_does_not_choose_the_labels_that_it_begins() {
    let (held, task) = labelled().typed(press(Key::Character("a".into())), true);
    assert!(held.hints.is_some());
    assert!(!held.outputs_shown);
    assert_eq!(task.units(), 0);
}

#[test]
fn a_click_leaves_the_mode() {
    let (clicked, _) = labelled().next(Message::Clicked);
    assert!(clicked.hints.is_none());
}

#[test]
fn a_turn_of_the_wheel_leaves_the_mode() {
    let (scrolled, _) = labelled().next(Message::Remeasure);
    assert!(scrolled.hints.is_none());
}

#[test]
fn a_resize_of_the_main_window_leaves_the_mode_and_that_of_another_does_not() {
    let size = Size::new(800.0, 600.0);
    let (other, _) = labelled().next(Message::Resized(window::Id::unique(), size));
    assert!(other.hints.is_some());
    let main = window::Id::unique();
    let (resized, _) = App {
        main: Some(main),
        ..labelled()
    }
    .next(Message::Resized(main, size));
    assert!(resized.hints.is_none());
}

#[test]
fn the_key_of_the_action_is_a_label_letter_in_both_keymaps() {
    let mut cua = labelled();
    cua.config.keymap = crate::front::keymap::Keymap::from(crate::front::keymap::Preset::Cua);
    [labelled(), cua].into_iter().for_each(|app| {
        let after = typing(app, "f");
        assert!(after.hints.is_some(), "no label of three begins with `f`");
    });
}

#[test]
fn keys_of_the_action_act_once_a_press_and_over_the_help_page() {
    use crate::front::keymap::Action;
    assert!(Action::Hint.acts_once_a_press());
    assert!(Action::Hint.acts_over_help());
    assert!(!Action::Hint.acts_in_fields());
}

#[test]
fn letters_match_the_labels_whatever_their_case() {
    let capital = |text: &str| chord(Key::Character(text.into()), Modifiers::empty());
    let (chosen, _) = labelled().typed(capital("A"), false);
    assert!(chosen.hints.is_none());
    assert!(chosen.outputs_shown);
    let (begun, _) = many_labelled().typed(capital("S"), false);
    assert_eq!(begun.hints.as_ref().map(|hints| hints.typed()), Some("s"));
}

#[test]
fn a_change_of_the_active_list_leaves_the_mode_and_the_same_list_keeps_it() {
    let before = labelled().active_cues();
    assert!(labelled().hints_after(&before).hints.is_some());
    assert!(labelled().hints_after(&[CueId(1)]).hints.is_none());
}
