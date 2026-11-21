// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of operators that take a motion: the keys that cut and copy, the
//! motions after them, and the counts before either

use super::*;

/// Returns the operator, the motion, and the counts that the keys complete
fn across(text: &str) -> Option<(Action, Action, Option<usize>)> {
    typed(text).across()
}

fn assert_across(wanted: &[(&str, Action, Action, Option<usize>)]) {
    wanted.iter().for_each(|&(text, operator, motion, count)| {
        assert_eq!(across(text), Some((operator, motion, count)), "{text}");
    });
}

#[test]
fn operator_and_motion_complete_a_command_through_every_key_of_the_motion() {
    let motions = [
        (Action::Next, ["j", "ArrowDown", "Control+n"].as_slice()),
        (Action::Previous, &["k", "ArrowUp", "Control+p"]),
        (Action::First, &["g g", "Control+Home"]),
        (Action::Last, &["G", "Control+End"]),
    ];
    for (operator, key) in [(Action::Cut, "d"), (Action::Copy, "y")] {
        for (motion, bound) in motions {
            for keys in bound {
                let text = format!("{key} {keys}");
                assert_eq!(across(&text), Some((operator, motion, None)), "{text}");
            }
        }
    }
}

#[test]
fn counts_before_the_operator_or_the_motion_or_both_multiply() {
    let most = format!("d {}j", "9 ".repeat(20));
    assert_across(&[
        ("3 d j", Action::Cut, Action::Next, Some(3)),
        ("d 3 j", Action::Cut, Action::Next, Some(3)),
        ("2 d 3 j", Action::Cut, Action::Next, Some(6)),
        ("1 2 y 2 k", Action::Copy, Action::Previous, Some(24)),
        ("d 3 G", Action::Cut, Action::Last, Some(3)),
        ("d 4 g g", Action::Cut, Action::First, Some(4)),
        (&most, Action::Cut, Action::Next, Some(usize::MAX)),
    ]);
    // Parts stay apart in the outcome
    let outcome = typed("2 d 3 j");
    assert_eq!(outcome.action, Some((Action::Cut, Some(2))));
    assert_eq!(outcome.motion, Some((Action::Next, Some(3))));
}

#[test]
fn doubled_operator_multiplies_the_counts_too() {
    assert_actions(&[
        ("d d", Action::Cut, None),
        ("3 d d", Action::Cut, Some(3)),
        ("d 3 d", Action::Cut, Some(3)),
        ("2 d 3 d", Action::Cut, Some(6)),
        ("2 y 3 y", Action::Copy, Some(6)),
    ]);
    assert_eq!(typed("2 d 3 d").motion, None);
}

#[test]
fn operator_waits_for_a_count_and_for_the_rest_of_a_motion() {
    ["d", "d 3", "2 d 3", "d g", "2 d 3 g"]
        .into_iter()
        .for_each(|text| assert_eq!(typed(text).pending, keys(text), "{text}"));
    assert!(typed("d g").action.is_none() && typed("d 3").action.is_none());
    assert!(typed("d j").pending.is_empty());
}

#[test]
fn key_that_is_no_motion_drops_the_operator_and_the_key_as_vim_does() {
    [
        "d i", "d z", "2 d 3 z", "d g z", "d u", "d x", "d p", "d o", "d l", "d g t", "y x", "y p",
    ]
    .into_iter()
    .for_each(|text| assert_eq!(typed(text), Outcome::default(), "{text}"));
    // Next key starts afresh
    assert_eq!(action("d x j"), Some((Action::Next, None)));
}

#[test]
fn go_and_panic_act_at_once_in_the_middle_of_an_operator() {
    let outcome = typed("d 3 Space");
    assert_eq!(outcome.action, Some((Action::Go, None)));
    assert!(outcome.pending.is_empty() && outcome.motion.is_none());
    assert_eq!(action("2 y Escape"), Some((Action::Panic, None)));
}

#[test]
fn operator_key_follows_a_rebinding_that_doubles_a_key() {
    let keymap = Keymap::default().with(Action::Cut, vec![keys("D D")]);
    let across = |text| typed_on(&keymap, text).across();
    assert_eq!(across("D j"), Some((Action::Cut, Action::Next, None)));
    assert_eq!(across("2 D G"), Some((Action::Cut, Action::Last, Some(2))));
    assert_eq!(across("d j"), None);
    // Key of Copy stays an operator
    assert_eq!(across("y k"), Some((Action::Copy, Action::Previous, None)));
}

#[test]
fn rebound_motion_carries_over_to_the_operators() {
    let keymap = Keymap::default().with(Action::Next, vec![keys("e")]);
    assert_eq!(
        typed_on(&keymap, "d e").across(),
        Some((Action::Cut, Action::Next, None))
    );
}

#[test]
fn binding_that_is_not_a_doubled_key_makes_no_operator() {
    let keymap = Keymap::default()
        .with(Action::Cut, vec![keys("Control+x"), keys("c v")])
        .with(Action::Copy, vec![keys("Control+c")]);
    let outcome = |text| typed_on(&keymap, text);
    assert_eq!(outcome("c j").across(), None);
    assert_eq!(outcome("y j").across(), None);
    assert_eq!(outcome("Control+x j").across(), None);
    assert_eq!(outcome("c v").action, Some((Action::Cut, None)));
}

#[test]
fn own_binding_of_the_keys_wins_over_the_operator() {
    let keymap = Keymap::default().with(Action::Standby, vec![keys("d j")]);
    let outcome = keymap.parse(&keys("d"), press("j"));
    assert_eq!(outcome.action, Some((Action::Standby, None)));
    assert_eq!(outcome.motion, None);
}

#[test]
fn cua_has_no_operator() {
    let outcome = typed_on(&Keymap::from(Preset::Cua), "Control+x ArrowDown");
    assert_eq!(outcome.across(), None);
    assert_eq!(outcome.action, Some((Action::Next, None)));
}

#[test]
fn box_lists_the_motions_after_an_operator_with_what_they_do() {
    let keymap = Keymap::default();
    let next = |text: &str| -> Vec<(String, Action, Option<Action>)> {
        keymap
            .continuations(&keys(text))
            .map(|(rest, action, operator)| (written(rest), action, operator))
            .collect()
    };
    let after_d = next("d");
    assert_eq!(after_d[0], ("d".to_owned(), Action::Cut, None));
    let motions: Vec<_> = after_d
        .iter()
        .filter(|(_, _, operator)| operator.is_some())
        .map(|(rest, action, operator)| (rest.as_str(), *action, *operator))
        .collect();
    let cut = Some(Action::Cut);
    assert_eq!(
        motions,
        [
            ("j", Action::Next, cut),
            ("g j", Action::Next, cut),
            ("ArrowDown", Action::Next, cut),
            ("Control+n", Action::Next, cut),
            ("+", Action::Next, cut),
            ("k", Action::Previous, cut),
            ("g k", Action::Previous, cut),
            ("ArrowUp", Action::Previous, cut),
            ("Control+p", Action::Previous, cut),
            ("-", Action::Previous, cut),
            ("g g", Action::First, cut),
            ("Control+Home", Action::First, cut),
            ("G", Action::Last, cut),
            ("Control+End", Action::Last, cut),
        ]
    );
    // Count in the middle changes nothing, and half a motion narrows it
    assert_eq!(next("2 d 3"), after_d);
    assert_eq!(
        next("d g"),
        [
            ("j".to_owned(), Action::Next, cut),
            ("k".to_owned(), Action::Previous, cut),
            ("g".to_owned(), Action::First, cut),
        ]
    );
    assert_eq!(
        next("y g")[2],
        ("g".to_owned(), Action::First, Some(Action::Copy))
    );
}

#[test]
fn descriptions_name_the_operator_and_the_rows_it_reaches() {
    assert_eq!(
        Action::Cut.over(Action::Next, Rows::Cues),
        "Cut the selected cue and the next"
    );
    assert_eq!(
        Action::Copy.over(Action::Last, Rows::Cues),
        "Copy the cues from the selection down to the last"
    );
    assert_eq!(
        Action::Delete.over(Action::First, Rows::Outputs),
        "Delete the outputs from the selection up to the first"
    );
    assert_eq!(
        Action::Cut.over(Action::Previous, Rows::Outputs),
        "Cut the selected output and the previous"
    );
}

#[test]
fn descriptions_name_the_rows_that_show_and_never_both() {
    let (cues, outputs) = (Rows::Cues, Rows::Outputs);
    assert_eq!(
        Action::AddBelow.description_in(cues),
        "Add a cue below the selection"
    );
    assert_eq!(
        Action::AddBelow.description_in(outputs),
        "Add an output below the selection"
    );
    assert_eq!(
        Action::PasteAbove.description_in(outputs),
        "Paste the copied outputs above the selection"
    );
    assert_eq!(
        Action::ChangeName.description_in(outputs),
        "Change the name of the selected output, selecting all of it"
    );
    assert_eq!(
        Action::Next.description_in(cues),
        "Select the next cue or menu option"
    );
    assert_eq!(
        Action::Search.description_in(outputs),
        "Search the outputs by the text of their columns"
    );
    Action::VARIANTS.iter().for_each(|action| {
        [cues, outputs].into_iter().for_each(|rows| {
            let text = action.description_in(rows);
            assert!(
                !text.contains("or output") && !text.contains("or the output"),
                "{text}"
            );
        });
    });
}

#[test]
fn help_page_names_the_keys_that_begin_an_operator() {
    assert_eq!(Keymap::default().operator_keys(), ["d", "y", ">", "<"]);
    assert!(Keymap::from(Preset::Cua).operator_keys().is_empty());
}

#[test]
fn indent_and_outdent_take_counts_and_motions_as_cut_and_copy_do() {
    assert_actions(&[
        ("> >", Action::Indent, None),
        ("3 > >", Action::Indent, Some(3)),
        ("> 3 >", Action::Indent, Some(3)),
        ("< <", Action::Outdent, None),
        ("2 < 3 <", Action::Outdent, Some(6)),
    ]);
    assert_across(&[
        ("> j", Action::Indent, Action::Next, None),
        ("3 > k", Action::Indent, Action::Previous, Some(3)),
        ("< G", Action::Outdent, Action::Last, None),
        ("< 4 g g", Action::Outdent, Action::First, Some(4)),
    ]);
    assert!(typed("> x").pending.is_empty() && typed("> x").action.is_none());
}

#[test]
fn descriptions_name_the_move_of_indent_and_outdent() {
    assert_eq!(
        Action::Indent.description(),
        "Move the selected cue into the group above it"
    );
    assert_eq!(
        Action::Outdent.description(),
        "Move the selected cue out of its group"
    );
    assert_eq!(
        Action::Indent.over(Action::Next, Rows::Cues),
        "Indent the selected cue and the next"
    );
    assert_eq!(
        Action::Outdent.over(Action::Last, Rows::Cues),
        "Outdent the cues from the selection down to the last"
    );
}

#[test]
fn cua_binds_indent_and_outdent_to_control_with_the_brackets() {
    let keymap = Keymap::from(Preset::Cua);
    assert_eq!(
        typed_on(&keymap, "Control+]").action,
        Some((Action::Indent, None))
    );
    assert_eq!(
        typed_on(&keymap, "3 Control+[").action,
        Some((Action::Outdent, Some(3)))
    );
    assert!(keymap.operator_keys().is_empty());
}
