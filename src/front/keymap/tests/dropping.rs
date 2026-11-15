// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of a key that completes or continues no command, which drops the
//! keys typed so far, and of the keys of Vim for the next and previous row

use super::*;

#[test]
fn key_that_ends_no_command_drops_the_keys_typed_so_far_and_itself() {
    ["d i", "g w", "4 w", "2 g w"]
        .into_iter()
        .for_each(|text| assert_eq!(typed(text), Outcome::default(), "{text}"));
    // Next key starts afresh
    assert_actions(&[
        ("g w j", Action::Next, None),
        ("2 g j", Action::Next, Some(2)),
    ]);
}

#[test]
fn g_j_and_g_k_move_the_selection_as_j_and_k_do_for_a_vim_user_who_types_them() {
    assert_actions(&[
        ("g j", Action::Next, None),
        ("3 g k", Action::Previous, Some(3)),
    ]);
    assert_eq!(
        listed(Preset::Vi, Action::Next),
        "j, gj, ArrowDown, Control+n, +"
    );
}

#[test]
fn digits_past_the_largest_count_are_not_kept() {
    let digits = "9 ".repeat(100);
    assert_eq!(typed(&digits).pending.len(), 20);
    let most = format!("{digits} j");
    assert_eq!(action(&most), Some((Action::Next, Some(usize::MAX))));
    // After an operator, the second count saturates the same way
    assert_eq!(typed(&format!("d {digits}")).pending.len(), 21);
    // Key that is no digit still ends the keys
    assert_eq!(typed(&format!("{digits} w")), Outcome::default());
}
