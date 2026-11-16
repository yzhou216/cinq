// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of what a menu tells the interface when it opens and closes

use super::*;

/// Returns the `MenuOpen` messages that opening a menu and then `act` send
fn announced(act: impl Fn(Choice<'_>, &mut Shell<'_, Message>)) -> Vec<bool> {
    let keymap = Keymap::default();
    let shared = Shared::default();
    let list = dropdown(OPTIONS, Some(OPTIONS[0]), Message::Step, &keymap);
    let choice = Choice::new(&list, &shared);
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    choice.open(&mut shell);
    act(choice, &mut shell);
    messages
        .iter()
        .filter_map(|message| match message {
            Message::MenuOpen(open) => Some(*open),
            _ => None,
        })
        .collect()
}

#[test]
fn the_interface_hears_when_a_menu_opens_and_each_way_it_closes() {
    assert_eq!(announced(|_, _| {}), [true]);
    let closes = [
        announced(typing(named(Named::Enter))),
        announced(typing_with(letter("y"), Modifiers::CTRL)),
        announced(typing_with(letter("e"), Modifiers::CTRL)),
        announced(typing(named(Named::Escape))),
        announced(typing(named(Named::Space))),
        announced(|choice, shell| choice.click_beside(false, shell)),
        announced(|choice, shell| choice.close(shell)),
    ];
    closes
        .iter()
        .for_each(|heard| assert_eq!(heard, &[true, false]));
    // Keys that keep it open say nothing
    assert_eq!(announced(typing(letter("j"))), [true]);
}

#[test]
fn a_list_without_options_never_opens_and_says_nothing() {
    let keymap = Keymap::default();
    let list = dropdown(Vec::<isize>::new(), None, Message::Step, &keymap);
    let shared = Shared::default();
    let mut messages = Vec::new();
    Choice::new(&list, &shared).open(&mut Shell::new(&mut messages));
    assert!(!shared.is_open());
    assert!(messages.is_empty());
}
