// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the configuration file, which chooses a keymap, replaces the
//! keys of the actions it names, sets how far ahead sounds are made ready,
//! and reports what it cannot use

use std::{fs, io, path::Path};

use tempfile::TempDir;

use super::*;
use crate::back::READY_AHEAD;

fn messages(problems: &[config::Problem]) -> Vec<&str> {
    problems
        .iter()
        .map(|problem| problem.message.as_str())
        .collect()
}

#[test]
fn file_chooses_a_keymap_and_replaces_the_keys_of_the_actions_it_names() {
    let (keymap, _, _, problems) = config::parse(
        "keymap = \"vi\"\n[keys]\ngo = \"F5\"\nnext = [\"m\", \"ArrowDown\", \"m\"]\ncut = []\n",
    );
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(keymap.name(), "vi");
    assert_eq!(keymap.keys(Action::Go), [keys("F5")]);
    // Keys listed twice for one action are bound once
    assert_eq!(keymap.keys(Action::Next), [keys("m"), keys("ArrowDown")]);
    assert!(keymap.keys(Action::Cut).is_empty());
    assert_eq!(keymap.keys(Action::Undo), [keys("u")]);
}

#[test]
fn file_reports_what_it_cannot_use_and_keeps_the_rest() {
    let (keymap, _, _, problems) =
        config::parse("colour = 1\n[keys]\nfly = \"f\"\nnext = [\"Foo\", \"m\"]\nlast = \"1 G\"\n");
    assert_eq!(
        messages(&problems),
        [
            "no setting is named `colour`",
            "no action is named `fly`",
            "next: `Foo` names no key",
            "last: `1 G` starts with a digit, which counts",
            "last keeps G, Control+End",
        ]
    );
    assert_eq!(keymap.keys(Action::Next), [keys("m")]);
    assert_eq!(
        keymap.keys(Action::Last),
        Keymap::default().keys(Action::Last)
    );
}

#[test]
fn setting_below_a_table_is_told_to_belong_above_it() {
    let (_, _, address, problems) =
        config::parse("[keys]\ngo = \"F5\"\nosc_listen = \"127.0.0.1:53000\"\n");
    assert_eq!(
        messages(&problems),
        ["`osc_listen` is a setting, which belongs above the first table"]
    );
    assert_eq!(
        address, None,
        "the setting is not read from inside the table"
    );
}

#[test]
fn entry_that_no_key_can_be_read_from_keeps_the_keys_of_the_action() {
    let (keymap, _, _, problems) =
        config::parse("[keys]\ngo = \"Spacebar\"\npanic = [\"Esc\", 1]\ndelete = []\n");
    assert_eq!(
        messages(&problems),
        [
            "go: `Spacebar` names no key",
            "go keeps Space",
            "panic: `Esc` names no key",
            "panic should be keys or a list of keys",
            "panic keeps Escape",
        ]
    );
    assert_eq!(keymap.keys(Action::Go), Keymap::default().keys(Action::Go));
    assert_eq!(
        keymap.keys(Action::Panic),
        Keymap::default().keys(Action::Panic)
    );
    // An empty list is written on purpose
    assert!(keymap.keys(Action::Delete).is_empty());
}

#[test]
fn entry_that_some_key_can_be_read_from_binds_those() {
    let (keymap, _, _, problems) = config::parse("[keys]\ngo = [\"Spacebar\", \"F5\"]\n");
    assert_eq!(messages(&problems), ["go: `Spacebar` names no key"]);
    assert_eq!(keymap.keys(Action::Go), [keys("F5")]);
}

#[test]
fn file_that_is_not_toml_leaves_the_defaults() {
    let (keymap, _, _, problems) = config::parse("[keys\n");
    assert_eq!(problems.len(), 1);
    assert_eq!(keymap.keys(Action::Go), Keymap::default().keys(Action::Go));
}

#[test]
fn file_reports_bindings_that_hide_others() {
    let (_, _, _, problems) = config::parse("[keys]\nhelp = \"g\"\n");
    assert_eq!(
        messages(&problems),
        [
            "`g` comes first, so `g j` can never be typed",
            "`g` comes first, so `g k` can never be typed",
            "`g` comes first, so `g g` can never be typed",
            "`g` comes first, so `g t` can never be typed",
            "`g` comes first, so `g T` can never be typed",
        ]
    );
}

#[test]
fn file_is_in_the_directory_named_on_the_command_line_or_else_that_of_the_platform() {
    assert_eq!(
        config::file(None),
        dirs::config_dir().map(|directory| directory.join("cinq").join("config.toml"))
    );
    // Windows takes a drive for the root of an absolute path
    let show = Path::new(if cfg!(windows) { r"C:\" } else { "/" }).join("show");
    assert_eq!(config::file(Some(&show)), Some(show.join("config.toml")));
    assert!(config::file(Some(Path::new("show"))).is_some_and(|file| file.is_absolute()));
}

#[test]
fn file_defines_keymaps_of_its_own_from_others_or_from_no_keys() {
    let (keymap, _, _, problems) = config::parse(
        "keymap = \"booth\"\n[keymaps.booth]\nfrom = \"vi\"\ngo = \"F5\"\n[keys]\nhelp = \"F12\"\n",
    );
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(keymap.name(), "booth");
    assert_eq!(keymap.keys(Action::Go), [keys("F5")]);
    assert_eq!(keymap.keys(Action::Undo), [keys("u")]);
    assert_eq!(keymap.keys(Action::Help), [keys("F12")]);
    let (keymap, _, _, problems) =
        config::parse("keymap = \"mine\"\n[keymaps.mine]\ngo = \"Space\"\npanic = \"Escape\"\n");
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(keymap.keys(Action::Go), [keys("Space")]);
    assert!(keymap.keys(Action::Next).is_empty());
}

const RETIRED: &str = "cinq no longer ships the cua keymap, so vi stands in for it";

/// Returns the part of `text` that the first problem lies in
fn lying<'a>(problems: &[config::Problem], text: &'a str) -> Option<&'a str> {
    problems
        .first()
        .and_then(|problem| problem.span.clone())
        .map(|span| &text[span])
}

#[test]
fn file_that_chooses_the_cua_keymap_is_told_so_and_gets_the_keys_of_vi() {
    let text = "keymap = \"cua\"\n";
    let (keymap, _, _, problems) = config::parse(text);
    assert_eq!(messages(&problems), [RETIRED]);
    assert_eq!(lying(&problems, text), Some("\"cua\""));
    assert_eq!(keymap.name(), "vi");
    [Action::Go, Action::Panic, Action::Next, Action::Cancel]
        .into_iter()
        .for_each(|action| {
            assert_eq!(keymap.keys(action), Keymap::default().keys(action));
        });
}

#[test]
fn keymap_of_the_file_that_starts_from_cua_starts_from_vi_and_the_file_is_told_so() {
    let text = "keymap = \"booth\"\n[keymaps.booth]\nfrom = \"cua\"\ngo = \"F5\"\n";
    let (keymap, _, _, problems) = config::parse(text);
    assert_eq!(messages(&problems), [RETIRED]);
    assert_eq!(lying(&problems, text), Some("\"cua\""));
    assert_eq!(keymap.name(), "booth");
    assert_eq!(keymap.keys(Action::Go), [keys("F5")]);
    assert_eq!(keymap.keys(Action::Panic), [keys("Escape")]);
    assert_eq!(
        keymap.keys(Action::Next),
        Keymap::default().keys(Action::Next)
    );
}

#[test]
fn file_that_declares_a_keymap_named_cua_uses_it_and_raises_nothing() {
    let text = "keymap = \"cua\"\n[keymaps.cua]\ngo = \"F5\"\n[keymaps.booth]\nfrom = \"cua\"\n";
    let (keymap, _, _, problems) = config::parse(text);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(keymap.name(), "cua");
    assert_eq!(keymap.keys(Action::Go), [keys("F5")]);
    assert!(keymap.keys(Action::Next).is_empty());
    let (keymap, _, _, problems) = config::parse(
        "keymap = \"booth\"\n[keymaps.cua]\ngo = \"F5\"\n[keymaps.booth]\nfrom = \"cua\"\n",
    );
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(keymap.keys(Action::Go), [keys("F5")]);
}

#[test]
fn a_name_that_only_looks_like_cua_is_an_unknown_keymap() {
    let (keymap, _, _, problems) = config::parse("keymap = \"cua2\"\n");
    assert_eq!(messages(&problems), ["no keymap is named `cua2`"]);
    assert_eq!(keymap.keys(Action::Go), Keymap::default().keys(Action::Go));
}

#[test]
fn a_line_of_thousands_of_keymaps_each_starting_from_the_next_keeps_the_last_keys() {
    let line: String = (0..5000)
        .map(|at| format!("[keymaps.k{at}]\nfrom = \"k{}\"\n", at + 1))
        .collect();
    let text = format!("keymap = \"k0\"\n{line}[keymaps.k5000]\nfrom = \"vi\"\ngo = \"F5\"\n");
    let (keymap, _, _, problems) = config::parse(&text);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(keymap.name(), "k0");
    assert_eq!(keymap.keys(Action::Go), [keys("F5")]);
}

#[test]
fn file_reports_what_is_wrong_with_its_keymaps() {
    // Keymap that is not there leaves vi
    let (keymap, _, _, problems) = config::parse("keymap = \"emacs\"\n");
    assert_eq!(messages(&problems), ["no keymap is named `emacs`"]);
    assert_eq!(keymap.name(), "vi");
    let (keymap, _, _, problems) =
        config::parse("keymap = \"a\"\n[keymaps.a]\nfrom = \"b\"\n[keymaps.b]\nfrom = \"a\"\n");
    assert_eq!(messages(&problems), ["keymap a starts from itself"]);
    assert_eq!(keymap.name(), "a");
    assert!(keymap.keys(Action::Go).is_empty());
    let (_, _, _, problems) = config::parse(
        "keymap = \"a\"\n[keymaps.a]\nfrom = \"b\"\n[keymaps.b]\nfrom = \"c\"\n[keymaps.c]\nfrom = \"b\"\n",
    );
    assert_eq!(messages(&problems), ["keymap b starts from itself"]);
    let (_, _, _, problems) = config::parse("[keymaps.vi]\ngo = \"F5\"\n");
    assert_eq!(
        messages(&problems),
        ["keymaps.vi goes unused, since vi is a shipped keymap"]
    );
    let (_, _, _, problems) =
        config::parse("keymap = \"booth\"\n[keymaps.booth]\nfrom = \"emacs\"\nfly = \"f\"\n");
    assert_eq!(
        messages(&problems),
        [
            "no keymap is named `emacs`",
            "keymaps.booth: no action is named `fly`",
        ]
    );
}

#[test]
fn file_sets_the_seconds_ahead_to_make_sounds_ready() {
    let (_, sound, _, problems) = config::parse("");
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(sound.ready_ahead, READY_AHEAD);
    let message = "ready_ahead should be a number of seconds, 0 or more";
    let cases = [
        ("10", 10.0, None),
        ("2.5", 2.5, None),
        ("0", 0.0, None),
        ("-1", READY_AHEAD, Some(message)),
        ("\"x\"", READY_AHEAD, Some(message)),
    ];
    cases.into_iter().for_each(|(value, seconds, problem)| {
        let (_, sound, _, problems) = config::parse(&format!("ready_ahead = {value}\n"));
        assert_eq!(sound.ready_ahead, seconds, "{value}");
        assert_eq!(messages(&problems), problem.as_slice(), "{value}");
    });
}

#[test]
fn file_names_the_audio_device_that_sound_plays_on() {
    let (_, sound, _, problems) = config::parse("");
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(sound.device, None);
    let message = "audio_device should be an ID that cinq --audio-devices lists";
    let cases = [
        (
            "\"alsa:plughw:CARD=1,DEV=0\"",
            Some("alsa:plughw:CARD=1,DEV=0"),
            None,
        ),
        // Unknown ID, which only opening the device finds
        ("\"nothing\"", Some("nothing"), None),
        ("\"\"", None, Some(message)),
        ("3", None, Some(message)),
    ];
    cases.into_iter().for_each(|(value, device, problem)| {
        let (_, sound, _, problems) = config::parse(&format!("audio_device = {value}\n"));
        assert_eq!(sound.device.as_deref(), device, "{value}");
        assert_eq!(messages(&problems), problem.as_slice(), "{value}");
    });
}

#[test]
fn file_made_for_editing_changes_no_key_and_leaves_a_file_there_alone() -> io::Result<()> {
    let home = TempDir::new()?;
    let file = home.path().join("cinq").join("config.toml");
    config::make(&file)?;
    let made = config::load(Some(&file));
    assert!(matches!(&made.source, config::Source::File(read) if *read == file));
    assert!(made.problems.is_empty(), "{:?}", made.problems);
    assert_eq!(made.keymap.name(), "vi");
    fs::write(&file, "keymap = \"booth\"\n")?;
    config::make(&file)?;
    assert_eq!(fs::read_to_string(&file)?, "keymap = \"booth\"\n");
    Ok(())
}

#[test]
fn file_gives_the_address_to_take_osc_commands_on() {
    let (_, _, listen, problems) = config::parse("");
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(listen, None);
    let message = "osc_listen must be IP:port, as 127.0.0.1:53000";
    let host = "osc_listen must be IP:port, not a host name";
    let cases = [
        ("\"0.0.0.0:53000\"", Some("0.0.0.0:53000"), None),
        ("\"127.0.0.1:53000\"", Some("127.0.0.1:53000"), None),
        ("\"[::]:53000\"", Some("[::]:53000"), None),
        ("\"localhost:53000\"", None, Some(host)),
        ("\"0.0.0.0\"", None, Some(message)),
        ("\"0.0.0.0:0\"", None, Some(message)),
        ("\"0.0.0.0:99999\"", None, Some(message)),
        ("53000", None, Some(message)),
    ];
    cases.into_iter().for_each(|(value, address, problem)| {
        let (_, _, listen, problems) = config::parse(&format!("osc_listen = {value}\n"));
        assert_eq!(
            listen,
            address.map(|address| address.parse().unwrap()),
            "{value}"
        );
        assert_eq!(messages(&problems), problem.as_slice(), "{value}");
    });
}
