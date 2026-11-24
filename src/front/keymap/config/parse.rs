// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading the text of the configuration file into key bindings and the
//! settings of the sound output, along with the problems that leave parts
//! of it unused, each in the part of the text that it lies in

use std::{
    net::{IpAddr, SocketAddr},
    ops::{ControlFlow, Range},
    slice,
};

use itertools::Itertools;
use toml::{
    Spanned,
    de::{DeString, DeTable, DeValue},
};

use super::{Problem, Sound};
use crate::{
    back::READY_AHEAD,
    front::keymap::{Action, Keymap, binding, vi},
};

const SETTINGS: [&str; 6] = [
    "audio_device",
    "keymap",
    "keymaps",
    "keys",
    "osc_listen",
    "ready_ahead",
];

pub(in crate::front::keymap) fn parse(
    text: &str,
) -> (Keymap, Sound, Option<SocketAddr>, Vec<Problem>) {
    let table = match DeTable::parse(text) {
        Ok(table) => table.into_inner(),
        Err(error) => {
            let problem = Problem::at(error.span(), format!("is not TOML: {}", error.message()));
            return (Keymap::default(), Sound::default(), None, vec![problem]);
        }
    };
    let unknown = table
        .keys()
        .filter(|name| !SETTINGS.contains(&name.get_ref().as_ref()))
        .map(|name| Problem::at(name.span(), format!("no setting is named `{name}`")));
    let none = DeTable::new();
    let (keymaps, malformed) = match table.get("keymaps").map(spanned) {
        Some((DeValue::Table(keymaps), _)) => (keymaps, Vec::new()),
        Some((_, span)) => (&none, vec![Problem::at(span, "keymaps should be a table")]),
        None => (&none, Vec::new()),
    };
    let shadowed = keymaps
        .keys()
        .filter(|name| name.get_ref() == vi::NAME)
        .map(|name| {
            let unused = format!("keymaps.{name} goes unused, since {name} is a shipped keymap");
            Problem::at(name.span(), unused)
        });
    let default = (vi::NAME, None);
    let ((name, named_at), unnamed) = match table.get("keymap").map(spanned) {
        Some((DeValue::String(name), span)) => ((name.as_ref(), Some(span)), None),
        Some((_, span)) => {
            let unnamed = Problem::at(span, "keymap should be the name of a keymap");
            (default, Some(unnamed))
        }
        None => (default, None),
    };
    let (chosen, built) =
        build(name, named_at, keymaps).unwrap_or_else(|problem| (Keymap::default(), vec![problem]));
    let keys_table = match table.get("keys").map(spanned) {
        Some((DeValue::Table(keys), _)) => Some(keys),
        _ => None,
    };
    let (keymap, unbound) = match table.get("keys").map(spanned) {
        Some((DeValue::Table(keys), _)) => bind(chosen, keys, ""),
        Some((_, span)) => (chosen, vec![Problem::at(span, "keys should be a table")]),
        None => (chosen, Vec::new()),
    };
    let (device, unrouted) = audio_device(&table);
    let (ready_ahead, unready) = ready_ahead(&table);
    let (osc_listen, unlistened) = osc_listen(&table);
    let problems = unknown
        .chain(malformed)
        .chain(shadowed)
        .chain(unnamed)
        .chain(built)
        .chain(unbound)
        .chain(unrouted)
        .chain(unready)
        .chain(unlistened)
        .chain(keymap.clashes().into_iter().map(|(action, said)| {
            // Entry of `[keys]` that the operator wrote, if the action has one
            let entry = keys_table
                .and_then(|keys| keys.keys().find(|name| name.get_ref() == action.name()));
            Problem::at(entry.map(|name| name.span()), said)
        }))
        .collect::<Vec<_>>();
    // Order of the file, so that a terminal and the sidebar read top down, by the
    // end of a place so that an array follows the elements that it holds, and those in no place last
    let problems = problems
        .into_iter()
        .sorted_by_key(|problem| {
            problem
                .span
                .as_ref()
                .map_or((true, 0), |span| (false, span.end))
        })
        .collect();
    (
        keymap,
        Sound {
            device,
            ready_ahead,
        },
        osc_listen,
        problems,
    )
}

/// Returns the address that `osc_listen` gives, or none to listen to
/// nothing, with a problem when the setting is not an IP address and a
/// port above 0.  A host name is not taken, since it could name no
/// address or several.
fn osc_listen(table: &DeTable) -> (Option<SocketAddr>, Option<Problem>) {
    let Some((value, span)) = table.get("osc_listen").map(spanned) else {
        return (None, None);
    };
    let address = match value {
        DeValue::String(text) => text
            .parse::<SocketAddr>()
            .ok()
            .filter(|address| address.port() > 0),
        _ => None,
    };
    match address {
        Some(address) => (Some(address), None),
        None => {
            let bad = match value {
                DeValue::String(text) if is_host_name(text) => {
                    "osc_listen must be IP:port, not a host name"
                }
                _ => "osc_listen must be IP:port, as 127.0.0.1:53000",
            };
            (None, Some(Problem::at(span, bad)))
        }
    }
}

/// Returns whether `text` is a host name and a port, which `osc_listen` does
/// not take, as against an IP address with a bad port or none
fn is_host_name(text: &str) -> bool {
    text.rsplit_once(':').is_some_and(|(host, port)| {
        port.parse::<u16>().is_ok_and(|port| port > 0)
            && host.parse::<IpAddr>().is_err()
            && !host.starts_with('[')
    })
}

/// Returns the ID that `audio_device` gives, or none for the default
/// device, with a problem when the setting is not a string that could be
/// an ID.  Whether a device has that ID shows only as the device opens.
fn audio_device(table: &DeTable) -> (Option<String>, Option<Problem>) {
    match table.get("audio_device").map(spanned) {
        Some((DeValue::String(id), _)) if !id.is_empty() => (Some(id.to_string()), None),
        Some((_, span)) => {
            let bad = "audio_device should be an ID that cinq --audio-devices lists";
            (None, Some(Problem::at(span, bad)))
        }
        None => (None, None),
    }
}

/// Returns the seconds that `ready_ahead` sets, or the default with a
/// problem when the setting is not a number, 0 or more
fn ready_ahead(table: &DeTable) -> (f64, Option<Problem>) {
    let Some((value, span)) = table.get("ready_ahead").map(spanned) else {
        return (READY_AHEAD, None);
    };
    let seconds = match value {
        DeValue::Integer(whole) => i64::from_str_radix(whole.as_str(), whole.radix())
            .ok()
            .map(|whole| whole as f64),
        DeValue::Float(fraction) => fraction.as_str().parse().ok(),
        _ => None,
    };
    match seconds.filter(|seconds| (0.0..=f64::MAX).contains(seconds)) {
        Some(seconds) => (seconds, None),
        None => {
            let bad = "ready_ahead should be a number of seconds, 0 or more";
            (READY_AHEAD, Some(Problem::at(span, bad)))
        }
    }
}

/// Keys that a keymap starts from, or the problem that leaves it none
type Start = Result<(Keymap, Vec<Problem>), Problem>;

/// `named_at` is the part of the text that names the keymap, if any, where a
/// problem with the name lies.  Each keymap of the line that [`line()`] finds
/// binds its keys over those of the one after it, from the end of the line
/// back to the keymap named.
fn build(name: &str, named_at: Option<Range<usize>>, keymaps: &DeTable) -> Start {
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

fn bind<'a>(
    keymap: Keymap,
    entries: impl IntoIterator<Item = (&'a Spanned<DeString<'a>>, &'a Spanned<DeValue<'a>>)>,
    place: &str,
) -> (Keymap, Vec<Problem>) {
    entries
        .into_iter()
        .fold((keymap, Vec::new()), |(keymap, problems), (name, value)| {
            let (keymap, more) = entry(keymap, name, value);
            let more = more.into_iter().map(|problem| Problem {
                message: format!("{place}{}", problem.message),
                ..problem
            });
            (keymap, problems.into_iter().chain(more).collect())
        })
}

fn entry(
    keymap: Keymap,
    name: &Spanned<DeString>,
    value: &Spanned<DeValue>,
) -> (Keymap, Vec<Problem>) {
    let Ok(action) = name.get_ref().parse::<Action>() else {
        // A setting added below a table lands in it, where only the cause helps
        let unknown = if SETTINGS.contains(&name.get_ref().as_ref()) {
            format!("`{name}` is a setting, which belongs above the first table")
        } else {
            format!("no action is named `{name}`")
        };
        return (keymap, vec![Problem::at(name.span(), unknown)]);
    };
    let values = match value.get_ref() {
        DeValue::Array(values) => &values[..],
        _ => slice::from_ref(value),
    };
    let (sequences, problems): (Vec<_>, Vec<_>) = values
        .iter()
        .map(|value| {
            let keys = match value.get_ref() {
                DeValue::String(text) => {
                    binding(text).map_err(|problem| format!("{name}: {problem}"))
                }
                _ => Err(format!("{name} should be keys or a list of keys")),
            };
            keys.map_err(|problem| Problem::at(value.span(), problem))
        })
        .partition_result();
    let sequences: Vec<_> = sequences.into_iter().unique().collect();
    // Entry of which no key can be read keeps the keys of the action, so that
    // a slip leaves GO and Panic bound, but an empty list unbinds on purpose
    if sequences.is_empty() && !problems.is_empty() && keymap.key(action).is_some() {
        let kept = Problem::at(
            value.span(),
            format!("{name} keeps {}", keymap.listed(action)),
        );
        return (keymap, [problems, vec![kept]].concat());
    }
    (keymap.with(action, sequences), problems)
}

fn spanned<T>(value: &Spanned<T>) -> (&T, Range<usize>) {
    (value.get_ref(), value.span())
}
