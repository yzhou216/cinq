// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the commands that arrive over OSC: that each does what its key or
//! button does, in show mode, over the help page, and out of hint mode, and
//! that a cue number that no cue has does nothing

use std::net::{IpAddr, Ipv4Addr};

use super::*;
use crate::{
    back::{Command, Media, Remote},
    front::keymap::config::Config,
};

/// Returns the state of [`show`] once the files of its cues have loaded as
/// ten seconds of sound, so that its cues can play
fn playable(selected: Option<CueId>) -> App {
    let app = show(selected);
    let loads: Vec<Command> = app
        .workspace
        .cues()
        .map(|cue| Command::Loaded {
            cue: cue.id,
            file: cue.file.clone(),
            media: Media::Loaded {
                duration: 10.0,
                sound: Some(10.0),
            },
        })
        .collect();
    app.send_all(loads)
}

/// Host that the commands come from
const FROM: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5));

fn remote(app: App, remote: Remote) -> App {
    after(app, [Message::Remote(FROM, remote)])
}

fn running(app: &App) -> Vec<u32> {
    app.workspace.active().iter().map(|run| run.id.0).collect()
}

fn number(text: &str) -> String {
    text.to_owned()
}

#[test]
fn go_fires_the_standby_of_the_list_in_view_as_the_key_does() {
    let by_key = after(playable(None), [Message::Go]);
    let by_osc = remote(playable(None), Remote::Go);
    assert_eq!(running(&by_osc), [1]);
    assert_eq!(running(&by_osc), running(&by_key));
    assert_eq!(by_osc.workspace.lists()[0].standby, Some(CueId(2)));
    let preshow = after(playable(None), [Message::SwitchList(1)]);
    assert_eq!(running(&remote(preshow, Remote::Go)), [20]);
}

#[test]
fn go_and_panic_act_in_show_mode_and_close_the_help_page() {
    let app = App {
        help: true,
        ..after(playable(None), [Message::ShowMode(true)])
    };
    let gone = remote(app, Remote::Go);
    assert_eq!(running(&gone), [1]);
    assert!(!gone.help);
    let app = App { help: true, ..gone };
    let panicked = remote(app, Remote::Panic);
    assert!(!panicked.help);
    let by_key = after(
        App {
            help: true,
            ..after(playable(None), [Message::Go])
        },
        [Message::Panic],
    );
    assert_eq!(
        format!("{:?}", panicked.workspace.active()),
        format!("{:?}", by_key.workspace.active())
    );
}

#[test]
fn every_command_leaves_hint_mode() {
    let commands = [
        Remote::Go,
        Remote::Panic,
        Remote::Pause,
        Remote::Resume,
        Remote::Start(number("1")),
        Remote::Stop(number("1")),
        Remote::PauseCue(number("1")),
        Remote::ResumeCue(number("1")),
        Remote::Playhead(number("3")),
    ];
    for command in commands {
        let (app, _) = playable(None).hinting();
        assert!(app.hints.is_some());
        assert!(remote(app, command.clone()).hints.is_none(), "{command:?}");
    }
}

#[test]
fn start_fires_the_cue_of_that_number_and_leaves_standby_alone() {
    let app = remote(playable(None), Remote::Start(number("21")));
    assert_eq!(running(&app), [21]);
    assert_eq!(app.workspace.lists()[1].standby, Some(CueId(20)));
    assert_eq!(app.workspace.lists()[0].standby, Some(CueId(1)));
}

#[test]
fn pause_cue_resume_cue_and_stop_act_on_the_cue_of_that_number() {
    let app = after(playable(None), [Message::Go, Message::Go]);
    let held = remote(app, Remote::PauseCue(number("2")));
    let paused = |app: &App, id: u32| app.workspace.active_of(CueId(id)).unwrap().paused;
    assert!(paused(&held, 2));
    assert!(!paused(&held, 1));
    let freed = remote(held, Remote::ResumeCue(number("2")));
    assert!(!paused(&freed, 2));
    let stopped = remote(freed, Remote::Stop(number("1")));
    assert_eq!(running(&stopped), [2]);
}

#[test]
fn pause_and_resume_act_on_every_running_cue() {
    let app = after(playable(None), [Message::Go, Message::Go]);
    let held = remote(app, Remote::Pause);
    assert!(held.workspace.active().iter().all(|run| run.paused));
    let freed = remote(held, Remote::Resume);
    assert!(freed.workspace.active().iter().all(|run| !run.paused));
}

#[test]
fn playhead_moves_standby_and_selects_the_cue_in_its_list() {
    let app = remote(playable(Some(CueId(2))), Remote::Playhead(number("21")));
    assert_eq!(app.workspace.lists()[1].standby, Some(CueId(21)));
    assert_eq!(app.selected, Some(CueId(21)));
    assert_eq!(app.list, 1);
    assert!(app.workspace.active().is_empty());
    let gone = after(app, [Message::Go]);
    assert_eq!(running(&gone), [21]);
}

#[test]
fn cue_number_that_no_cue_has_does_nothing() {
    let before = playable(Some(CueId(2)));
    for command in [
        Remote::Start(number("99")),
        Remote::Stop(number("99")),
        Remote::PauseCue(number("99")),
        Remote::ResumeCue(number("99")),
        Remote::Playhead(number("99")),
    ] {
        let app = remote(playable(Some(CueId(2))), command);
        assert!(app.workspace.active().is_empty());
        assert_eq!(app.selected, before.selected);
        assert_eq!(app.workspace.lists()[0].standby, Some(CueId(1)));
    }
}

#[test]
fn failure_to_listen_shows_until_the_address_changes() {
    let address = |text: &str| {
        let mut config = Config::default();
        config.osc_listen = Some(text.parse().unwrap());
        config
    };
    let app = App {
        config: address("127.0.0.1:53000"),
        ..App::default()
    };
    let failed = after(app, [Message::OscFailed("OSC: cannot listen".to_owned())]);
    assert!(failed.osc_problem.is_some());
    let same = after(
        failed,
        [Message::Reconfigured(Box::new(address("127.0.0.1:53000")))],
    );
    assert!(same.osc_problem.is_some());
    let moved = after(
        same,
        [Message::Reconfigured(Box::new(address("127.0.0.1:53001")))],
    );
    assert!(moved.osc_problem.is_none());
}

#[test]
fn command_that_moves_the_selection_takes_the_keyboard_out_of_a_field() {
    let units = |app: App, message| app.next(message).1.units();
    let at = |id| playable(Some(CueId(id)));
    // The key of GO reveals the selection that it moves, and no key unfocuses
    assert_eq!(
        units(at(1), Message::Remote(FROM, Remote::Go)),
        units(at(1), Message::Go) + 1
    );
    // Selection stays, as it does when it is not standby
    assert_eq!(
        units(at(5), Message::Remote(FROM, Remote::Go)),
        units(at(5), Message::Go)
    );
    assert_eq!(units(at(5), Message::Remote(FROM, Remote::Pause)), 0);
    let playhead = |number: &str| Message::Remote(FROM, Remote::Playhead(number.to_owned()));
    assert_eq!(units(at(2), playhead("5")), units(at(2), playhead("2")) + 1);
}

#[test]
fn listening_clears_the_failure_to_listen() {
    let failed = after(
        App::default(),
        [Message::OscFailed("OSC: cannot".to_owned())],
    );
    assert!(failed.osc_problem.is_some());
    assert!(after(failed, [Message::OscListening]).osc_problem.is_none());
}

#[test]
fn playhead_selects_as_a_click_does_closing_search_and_leaving_the_outputs() {
    let searching = super::search::opened(2);
    assert!(searching.search.is_some());
    let moved = remote(searching, Remote::Playhead(number("4")));
    assert!(moved.search.is_none());
    assert_eq!(moved.selected, Some(CueId(4)));
    let outputs = after(playable(Some(CueId(2))), [Message::ShowOutputs]);
    assert!(outputs.outputs_shown);
    let moved = remote(outputs, Remote::Playhead(number("5")));
    assert!(!moved.outputs_shown);
    assert_eq!(moved.selected, Some(CueId(5)));
    assert!(after(moved, [Message::Back(1)]).outputs_shown);
}

#[test]
fn cue_number_that_no_cue_has_leaves_typed_keys_be_and_hint_mode_only_for_a_new_line() {
    let (hinting, _) = playable(None).hinting();
    let first = remote(hinting, Remote::Start(number("none")));
    assert!(first.hints.is_none(), "the sidebar gains a line");
    let (again, _) = first.hinting();
    assert!(
        remote(again, Remote::Start(number("none"))).hints.is_some(),
        "a line that was there moves nothing"
    );
    let halfway = typing(playable(None), "2");
    assert!(!halfway.keys.is_empty());
    let kept = remote(halfway, Remote::Stop(number("none")));
    assert!(!kept.keys.is_empty());
    let dropped = remote(typing(playable(None), "2"), Remote::Pause);
    assert!(dropped.keys.is_empty());
}

#[test]
fn command_that_did_nothing_shows_as_one_line_until_a_command_acts() {
    let first = remote(playable(None), Remote::Start(number("99")));
    assert_eq!(
        first.osc_ignored.as_deref(),
        Some("OSC from 192.168.1.5: no cue is numbered \"99\"")
    );
    let second = remote(first, Remote::Stop(number("98")));
    assert_eq!(
        second.osc_ignored.as_deref(),
        Some("OSC from 192.168.1.5: no cue is numbered \"98\""),
        "the latest replaces the line"
    );
    let heard = after(
        second,
        [Message::OscIgnored("OSC: not an OSC packet".into())],
    );
    assert_eq!(heard.osc_ignored.as_deref(), Some("OSC: not an OSC packet"));
    assert!(remote(heard, Remote::Pause).osc_ignored.is_none());
}

#[test]
fn line_of_a_command_that_did_nothing_goes_with_the_address() {
    let address = |text: &str| {
        let mut config = Config::default();
        config.osc_listen = Some(text.parse().unwrap());
        config
    };
    let app = App {
        config: address("127.0.0.1:53000"),
        ..App::default()
    };
    let told = after(app, [Message::OscIgnored("OSC: not an OSC packet".into())]);
    let moved = after(
        told,
        [Message::Reconfigured(Box::new(address("127.0.0.1:53001")))],
    );
    assert!(moved.osc_ignored.is_none());
}
