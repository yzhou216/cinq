// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of when the light patch shows its help on addresses

use super::shows_help;
use crate::front::App;

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
