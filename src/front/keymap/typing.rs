// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading keys typed in a row, which wait until they complete a command

mod alone;

use super::{Action, Keymap, Press};

/// What a key press amounts to, after the keys typed before it
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    /// Keys of a command typed halfway, which wait for more
    pub pending: Vec<Press>,
    /// Action that the keys complete, with the count typed before them
    pub action: Option<(Action, Option<usize>)>,
}

impl Keymap {
    /// Keys that complete or continue no command are dropped, with the keys
    /// typed so far, as Vim beeps and drops them, so that a key mistyped
    /// halfway through a command, such as the `x` of `d x`, never starts a
    /// command of its own.  The keys of GO and Panic act at once all the
    /// same.
    pub fn parse(&self, pending: &[Press], press: Press) -> Outcome {
        if let Some((action, _)) = self.at_once_bound(press) {
            return Outcome {
                pending: Vec::new(),
                action: Some((action, None)),
            };
        }
        if press.digit().is_some() && saturated(pending) {
            return Outcome {
                pending: pending.to_vec(),
                action: None,
            };
        }
        let keys = [pending, &[press]].concat();
        let (digits, command) = split_count(&keys);
        let count = counted(digits);
        match self.find(command) {
            Some(action) => Outcome {
                pending: Vec::new(),
                action: Some((action, count)),
            },
            None if self.bound().any(|(_, bound)| bound.starts_with(command)) => Outcome {
                pending: keys,
                action: None,
            },
            None => Outcome::default(),
        }
    }

    /// Returns what can follow the keys typed so far, apart from a count
    /// before them: the rest of each binding that they begin, with its
    /// action, in the order of the actions on the help page.  A count alone
    /// waits for any command, so nothing in particular follows it.
    pub fn continuations<'a>(
        &'a self,
        pending: &'a [Press],
    ) -> impl Iterator<Item = (&'a [Press], Action)> + 'a {
        let (_, typed) = split_count(pending);
        self.bound()
            .filter(move |_| !typed.is_empty())
            .filter_map(move |(action, keys)| {
                keys.strip_prefix(typed)
                    .filter(|rest| !rest.is_empty())
                    .map(|rest| (rest, action))
            })
    }

    fn find(&self, keys: &[Press]) -> Option<Action> {
        self.bound()
            .find_map(|(action, bound)| (bound == keys).then_some(action))
    }
}

fn split_count(keys: &[Press]) -> (&[Press], &[Press]) {
    keys.split_at(keys.iter().take_while(|key| key.digit().is_some()).count())
}

/// Returns the count that digit keys type, or none for no digit or only zeros
fn counted(digits: &[Press]) -> Option<usize> {
    let count = digits
        .iter()
        .filter_map(|key| key.digit())
        .fold(0, |count: usize, digit| {
            count.saturating_mul(10).saturating_add(digit)
        });
    Some(count).filter(|&count| count > 0)
}

/// Tells whether the digits that end `keys` already make the largest count,
/// so that one more changes nothing and a held digit key costs nothing
fn saturated(keys: &[Press]) -> bool {
    let start = keys
        .iter()
        .rposition(|key| key.digit().is_none())
        .map_or(0, |last| last + 1);
    counted(&keys[start..]) == Some(usize::MAX)
}
