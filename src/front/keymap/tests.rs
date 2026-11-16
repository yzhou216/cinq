// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the key bindings, with the helpers that read the keys they
//! type

mod cancel;
mod dropping;
mod file;
mod manual;
mod naming;
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
fn no_key_is_bound_twice_in_one_place_of_a_shipped_keymap() {
    let places: [fn(Action) -> bool; 2] = [Action::acts_in_cue_list, Action::acts_in_menu];
    let twice: Vec<_> = Preset::VARIANTS
        .iter()
        .flat_map(|preset| {
            places.into_iter().flat_map(move |acts_in| {
                Action::VARIANTS
                    .iter()
                    .filter(move |&&action| acts_in(action))
                    .flat_map(|&action| preset.keys(action))
                    .duplicates()
                    .map(|keys| (preset.name(), *keys))
            })
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

#[test]
fn a_key_may_pick_in_a_menu_and_scroll_the_cue_list() {
    let keymap = Keymap::default();
    let menu = |text: &str| keymap.in_menu(press(text));
    assert_actions(&[
        ("Control+e", Action::ScrollDown, None),
        ("3 Control+y", Action::ScrollUp, Some(3)),
    ]);
    assert_eq!(menu("Control+e"), Some(Action::CloseMenu));
    assert_eq!(menu("Control+y"), Some(Action::Pick));
    // Keys of the list are no keys of a menu
    assert_eq!(menu("Control+d"), None);
}

#[test]
fn a_clash_in_the_cue_list_or_in_a_menu_is_still_reported() {
    let in_list = Keymap::default().with(Action::Next, vec![keys("j"), keys("Control+e")]);
    assert_eq!(
        in_list.conflicts(),
        [
            "`Control+e` is bound to both next and scroll_down, so scroll_down goes without",
            "`Control+e` is bound to both next and close_menu, so close_menu goes without",
        ]
    );
    let in_menu = Keymap::default().with(Action::Pick, vec![keys("Control+e")]);
    assert_eq!(
        in_menu.conflicts(),
        ["`Control+e` is bound to both pick and close_menu, so close_menu goes without"]
    );
    let across = Keymap::default().with(Action::Pick, vec![keys("j")]);
    assert_eq!(
        across.conflicts(),
        ["`j` is bound to both next and pick, so pick goes without"]
    );
}

#[test]
fn a_clash_over_g_j_g_k_and_the_search_keys_is_found_in_the_place_they_act_in() {
    let with = |action, text: &str| Keymap::default().with(action, vec![keys(text)]);
    assert_eq!(
        with(Action::ScrollDown, "g j").conflicts(),
        ["`g j` is bound to both next and scroll_down, so scroll_down goes without"]
    );
    assert_eq!(
        with(Action::ScrollUp, "/").conflicts(),
        ["`/` is bound to both scroll_up and search, so search goes without"]
    );
    assert_eq!(
        with(Action::Pick, "g k").conflicts(),
        ["`g k` is bound to both previous and pick, so pick goes without"]
    );
    // Search keys act in the cue list only, so a menu may use them
    assert_eq!(with(Action::Pick, "/").conflicts(), Vec::<String>::new());
    assert_eq!(
        with(Action::CloseMenu, "n").conflicts(),
        Vec::<String>::new()
    );
}
