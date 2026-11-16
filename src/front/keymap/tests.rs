// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the key bindings, with the helpers that read the keys they
//! type

mod cancel;
mod dropping;
mod file;
mod manual;
mod notation;
mod operators;
mod typing;
mod viewing;

use itertools::Itertools;
use strum::VariantArray;

use super::{
    Action, Keymap, Preset, Press, binding, config,
    press::{sequence, shown, written},
    typing::Outcome,
};

fn keys(text: &str) -> Vec<Press> {
    sequence(text).expect("keys in the tests are valid")
}

fn one(text: &str) -> Option<Press> {
    keys(text).first().copied()
}

fn press(text: &str) -> Press {
    one(text).expect("a key")
}

fn listed(preset: Preset, action: Action) -> String {
    Keymap::from(preset).listed(action)
}

/// Returns what the last key amounts to on a keymap
fn typed_on(keymap: &Keymap, text: &str) -> Outcome {
    keys(text)
        .into_iter()
        .fold(Outcome::default(), |outcome, press| {
            keymap.parse(&outcome.pending, press)
        })
}

fn typed(text: &str) -> Outcome {
    typed_on(&Keymap::default(), text)
}

fn action(text: &str) -> Option<(Action, Option<usize>)> {
    typed(text).action
}

fn assert_actions(wanted: &[(&str, Action, Option<usize>)]) {
    wanted.iter().for_each(|&(text, action, count)| {
        assert_eq!(typed(text).action, Some((action, count)), "{text}");
    });
}

#[test]
fn shipped_keymaps_are_valid_and_hide_no_binding() {
    let invalid: Vec<_> = Preset::VARIANTS
        .iter()
        .flat_map(|preset| Action::VARIANTS.iter().map(|&action| preset.keys(action)))
        .flatten()
        .copied()
        .filter(|keys| binding(keys).is_err())
        .collect();
    assert!(invalid.is_empty(), "{invalid:?}");
    let conflicts: Vec<_> = Preset::VARIANTS
        .iter()
        .map(|&preset| (preset.name(), Keymap::from(preset).conflicts()))
        .filter(|(_, conflicts)| !conflicts.is_empty())
        .collect();
    assert!(conflicts.is_empty(), "{conflicts:?}");
}

#[test]
fn open_is_control_l_in_vi_and_control_o_in_cua() {
    let opens = |preset: Preset| {
        Keymap::from(preset)
            .key(Action::Open)
            .expect("both keymaps bind open")
    };
    assert_eq!(opens(Preset::Vi), "Control+l");
    assert_eq!(opens(Preset::Cua), "Control+o");
}

#[test]
fn keys_of_go_and_panic_hide_every_other_sequence_that_holds_them() {
    let keymap = Keymap::default()
        .with(Action::Panic, vec![keys("Escape"), keys("Shift+Escape")])
        .with(Action::NextList, vec![keys("Shift+Space")])
        .with(Action::First, vec![keys("g Space")])
        .with(Action::Delete, vec![keys("Space x")]);
    assert_eq!(
        keymap.conflicts(),
        [
            "`Space` comes first, so `Space x` can never be typed",
            "`Space` acts at once as go, so `Shift+Space` can never be typed",
            "`Space` acts at once as go, so `g Space` can never be typed",
        ]
    );
}

#[test]
fn back_and_forward_keys_follow_vim_and_the_browsers_and_act_in_fields() {
    assert_eq!(listed(Preset::Vi, Action::Back), "Control+o");
    assert_eq!(listed(Preset::Vi, Action::Forward), "Control+i");
    assert_eq!(listed(Preset::Cua, Action::Back), "Alt+ArrowLeft");
    assert_eq!(listed(Preset::Cua, Action::Forward), "Alt+ArrowRight");
    let acting = |preset: Preset, text: &str| Keymap::from(preset).acts_in_fields(press(text));
    assert!(acting(Preset::Vi, "Control+o") && acting(Preset::Vi, "Control+i"));
    assert!(acting(Preset::Cua, "Alt+ArrowLeft") && acting(Preset::Cua, "Alt+ArrowRight"));
    assert!(!acting(Preset::Cua, "Control+i"));
}

#[test]
fn no_key_is_bound_twice_in_a_shipped_keymap() {
    let twice: Vec<_> = Preset::VARIANTS
        .iter()
        .flat_map(|preset| {
            Action::VARIANTS
                .iter()
                .flat_map(|&action| preset.keys(action))
                .duplicates()
                .map(|keys| (preset.name(), *keys))
        })
        .collect();
    assert!(twice.is_empty(), "{twice:?}");
}

#[test]
fn question_mark_and_f1_show_the_help_in_both_keymaps_and_search_up_has_no_key() {
    assert_eq!(listed(Preset::Vi, Action::Help), "?, F1");
    assert_eq!(listed(Preset::Cua, Action::Help), "F1, ?");
    assert_eq!(listed(Preset::Vi, Action::SearchBackward), "none");
    assert_eq!(listed(Preset::Cua, Action::SearchBackward), "none");
    Preset::VARIANTS.iter().for_each(|&preset| {
        let keymap = Keymap::from(preset);
        assert_eq!(keymap.alone(press("?")), Some(Action::Help), "{preset:?}");
        assert!(!keymap.acts_in_fields(press("?")), "{preset:?}");
        assert!(!keymap.acts_when_captured(press("?"), true), "{preset:?}");
    });
}
