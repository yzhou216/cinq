// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keymaps of the file's own that start each from another, and the keys
//! that a line of them comes to

use std::ops::{ControlFlow, Range};

use toml::de::{DeTable, DeValue};

use super::{Problem, bind, spanned};
use crate::front::keymap::{Keymap, vi};

/// Keys that a keymap starts from, or the problem that leaves it none
type Start = Result<(Keymap, Vec<Problem>), Problem>;

/// `named_at` is the part of the text that names the keymap, if any, where a
/// problem with the name lies.  Each keymap of the line that [`line()`] finds
/// binds its keys over those of the one after it, from the end of the line
/// back to the keymap named.
pub(super) fn build(name: &str, named_at: Option<Range<usize>>, keymaps: &DeTable) -> Start {
    let (line, root) = line(name, named_at, keymaps);
    line.into_iter().rev().fold(root, |start, (name, entries)| {
        let (start, problems) =
            start.unwrap_or_else(|problem| (Keymap::empty(name), vec![problem]));
        let bindings = entries.iter().filter(|(key, _)| key.get_ref() != "from");
        let (keymap, unbound) = bind(start.named(name), bindings, &format!("keymaps.{name}: "));
        Ok((keymap, [problems, unbound].concat()))
    })
}

/// Returns the keymaps of the file's own that start each from the next, the
/// named one first, and what the last of them starts from.  A keymap met
/// again ends the line, since it starts from itself and so from no keys.
/// The line is walked rather than recursed into, so that a file of
/// thousands of keymaps cannot overflow the stack of the thread that reads
/// it.
fn line<'a, 'de>(
    name: &'a str,
    named_at: Option<Range<usize>>,
    keymaps: &'a DeTable<'de>,
) -> (Vec<(&'a str, &'a DeTable<'de>)>, Start) {
    let mut line = Vec::new();
    let mut next = (name, named_at);
    loop {
        let (name, named_at) = next;
        let entries = match declared(name, named_at.clone(), keymaps) {
            ControlFlow::Continue(entries) => entries,
            ControlFlow::Break(root) => return (line, root),
        };
        if line.iter().any(|&(seen, _)| seen == name) {
            let looped = format!("keymap {name} starts from itself");
            return (line, Err(Problem::at(named_at, looped)));
        }
        line.push((name, entries));
        next = match entries.get("from").map(spanned) {
            Some((DeValue::String(from), span)) => (from.as_ref(), Some(span)),
            Some((_, span)) => {
                let unnamed = format!("keymaps.{name}.from should be the name of a keymap");
                return (line, Err(Problem::at(span, unnamed)));
            }
            None => return (line, Ok((Keymap::empty(name), Vec::new()))),
        };
    }
}

/// Returns the table of the keymap of the file's own that `name` names, or
/// what ends a line of keymaps there: the keys of a shipped keymap, or the
/// problem with the name
fn declared<'a, 'de>(
    name: &str,
    named_at: Option<Range<usize>>,
    keymaps: &'a DeTable<'de>,
) -> ControlFlow<Start, &'a DeTable<'de>> {
    if name == vi::NAME {
        return ControlFlow::Break(Ok((Keymap::default(), Vec::new())));
    }
    match keymaps.get(name).map(spanned) {
        Some((DeValue::Table(entries), _)) => ControlFlow::Continue(entries),
        Some((_, span)) => {
            let malformed = format!("keymaps.{name} should be a table");
            ControlFlow::Break(Err(Problem::at(span, malformed)))
        }
        // Vi stands in for the keymap that cinq shipped before, so that an
        // operator who relied on it keeps GO and Panic, where a keymap that
        // starts from an unknown one has no keys
        None if name == "cua" => {
            let retired = "cinq no longer ships the cua keymap, so vi stands in for it";
            ControlFlow::Break(Ok((
                Keymap::default(),
                vec![Problem::at(named_at, retired)],
            )))
        }
        None => {
            let unknown = format!("no keymap is named `{name}`");
            ControlFlow::Break(Err(Problem::at(named_at, unknown)))
        }
    }
}
