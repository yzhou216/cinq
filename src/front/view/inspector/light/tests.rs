// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of when the light patch shows its help on addresses

use iced::advanced::widget::{Tree, tree};

use super::{patch, shows_help};
use crate::{
    back::{Command, PatchEdit, Workspace},
    front::{App, view::widget::focus::State},
};

#[test]
fn the_help_on_addresses_shows_only_while_an_address_has_the_keyboard_or_is_refused() {
    let app = |address_held| App {
        address_held,
        ..App::default()
    };
    let refused = Some("addresses are 1 to 512".to_owned());
    assert!(!shows_help(&app(false), &None));
    assert!(shows_help(&app(true), &None));
    assert!(shows_help(&app(false), &refused));
}

/// Returns the state of every widget that takes the keyboard like a button,
/// in the order of the tree
fn states(tree: &mut Tree) -> Vec<&mut State> {
    let own = (tree.tag == tree::Tag::of::<State>()).then(|| tree.state.downcast_mut::<State>());
    own.into_iter()
        .chain(tree.children.iter_mut().flat_map(states))
        .collect()
}

/// Returns whether each of those has the keyboard
fn held(tree: &mut Tree) -> Vec<bool> {
    states(tree)
        .into_iter()
        .map(|state| state.focused)
        .collect()
}

/// Returns an app whose light patch holds `count` instruments
fn patched(count: usize) -> App {
    let workspace = (0..count).fold(Workspace::default(), |workspace, _| {
        workspace.apply(Command::Patch(PatchEdit::Add))
    });
    App {
        workspace,
        ..App::default()
    }
}

#[test]
fn the_keyboard_stays_at_the_place_of_a_removed_button_for_the_row_that_slides_in() {
    // iced matches the state of widgets by position, so that the Remove
    // button of the row below takes the place and the focus of the one gone
    let before = patched(3);
    let element = patch(&before);
    let mut tree = Tree::new(&element);
    // Add instrument comes first, and the Remove button of the first row next
    states(&mut tree)[1].focused = true;
    assert_eq!(held(&mut tree), [false, true, false, false]);
    let after = patched(2);
    tree.diff(patch(&after));
    assert_eq!(held(&mut tree), [false, true, false]);
}
