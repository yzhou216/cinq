// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the reasons that a setting of an output does not apply, and of
//! the help beside the device list

use super::{device, reasons};
use crate::{
    back::{
        Command, CueId, CueList, Device, Fault, InstrumentId, Output, OutputEdit, OutputId,
        Workspace,
    },
    front::{App, Setting, keymap::config::Config},
};

/// Returns the interface of a show of two outputs, Main and Output 1
fn app() -> App {
    App {
        workspace: Workspace::new(vec![CueList::new("Main", vec![])])
            .apply(Command::add_output(1, 1)),
        ..App::default()
    }
}

/// Returns Main, the first output of the show of `app`
fn main(app: &App) -> Output {
    app.workspace.output(OutputId(1)).expect("Main").clone()
}

#[test]
fn a_name_that_is_empty_or_that_another_output_has_is_rejected_with_its_reason() {
    let app = app();
    let main = main(&app);
    let rejected = |name| reasons::name(&app, &main, name);
    assert_eq!(rejected("Main"), None, "its own name");
    assert_eq!(rejected("Rear"), None);
    assert_eq!(rejected("").as_deref(), Some("the name is empty"));
    assert_eq!(rejected("  ").as_deref(), Some("the name is empty"));
    assert_eq!(
        rejected("Output 1").as_deref(),
        Some("another output has this name")
    );
}

#[test]
fn channels_that_are_neither_one_nor_two_are_rejected_and_the_help_names_what_a_channel_is() {
    ["3", "1-2", "1,3", "256"]
        .into_iter()
        .for_each(|text| assert_eq!(reasons::channels(text), None, "{text}"));
    ["", "1-23-4", "1-3", "0", "257", "-2"]
        .into_iter()
        .for_each(|text| {
            assert_eq!(
                reasons::channels(text).as_deref(),
                Some("channels are 3 for one or 1-2 for two, up to 256"),
                "{text:?}"
            );
        });
    assert_eq!(
        reasons::channels_help(),
        "Channels 1 to 256: 3 for one, 1-2 or 1,3 for two"
    );
}

#[test]
fn a_level_out_of_range_is_rejected_but_an_empty_one_sends_nothing() {
    ["", "  ", "+0.0", "-6 dB", "12", "-60"]
        .into_iter()
        .for_each(|text| assert_eq!(reasons::level(text), None, "{text:?}"));
    ["loud", "13", "-61"].into_iter().for_each(|text| {
        assert_eq!(
            reasons::level(text).as_deref(),
            Some("levels run from -60 to +12 dB, or empty for none"),
            "{text:?}"
        );
    });
}

#[test]
fn a_device_that_is_not_there_gets_help_and_one_that_is_gets_none() {
    let missing = App {
        audio_problems: vec![(OutputId(1), Fault::Missing("not found".to_owned()))],
        ..app()
    };
    assert_eq!(
        device::hint(&missing, &main(&missing)),
        "Connect it, or choose another Default device in the sidebar"
    );
    let channels = App {
        audio_problems: vec![(OutputId(1), Fault::Channels(2))],
        ..app()
    };
    assert_eq!(device::hint(&channels, &main(&channels)), "");
    assert_eq!(device::hint(&app(), &main(&app())), "");
}

#[test]
fn a_missing_default_device_keeps_its_name_and_points_to_the_sidebar() {
    let missing = |device: &str| App {
        audio_devices: vec![(
            "alsa:pipewire".to_owned(),
            "PipeWire Sound Server".to_owned(),
        )],
        config: Config::default().with_audio_device(Some(device.to_owned())),
        audio_problems: vec![(OutputId(1), Fault::Missing("not found".to_owned()))],
        ..app()
    };
    let gone = missing("alsa:gone");
    assert_eq!(
        device::placeholder(&gone, &main(&gone)),
        "Default device (alsa:gone), not found"
    );
    assert_eq!(
        device::hint(&gone, &main(&gone)),
        "Connect it, or choose another Default device in the sidebar"
    );
    // Device that is there needs neither, and a named device keeps its own
    let there = App {
        audio_problems: vec![],
        ..missing("alsa:pipewire")
    };
    assert_eq!(
        device::placeholder(&there, &main(&there)),
        "Default device (PipeWire Sound Server)"
    );
    let named = App {
        workspace: gone.workspace.clone().apply(Command::EditOutput(
            OutputId(1),
            OutputEdit::Device(Device::Named {
                id: "alsa:usb".to_owned(),
                name: "USB Audio".to_owned(),
            }),
        )),
        ..gone
    };
    assert_eq!(
        device::placeholder(&named, &main(&named)),
        "USB Audio, not found"
    );
    assert_eq!(
        device::hint(&named, &main(&named)),
        "Connect the device, or pick another"
    );
}

#[test]
fn a_light_level_or_an_address_out_of_range_is_rejected_with_its_reason() {
    let app = app();
    let reason = |setting, text| reasons::rejection(&app, setting, text);
    let level = Setting::Light(CueId(1), InstrumentId(1));
    ["", "0", "100", "12.5%"]
        .into_iter()
        .for_each(|text| assert_eq!(reason(level, text), None, "{text:?}"));
    ["101", "-1", "bright"].into_iter().for_each(|text| {
        assert_eq!(
            reason(level, text).as_deref(),
            Some("levels run from 0 to 100 percent, or empty for unchanged"),
            "{text:?}"
        );
    });
    let address = Setting::Address(InstrumentId(1));
    ["1", "512", "2/12", "32768/512"]
        .into_iter()
        .for_each(|text| assert_eq!(reason(address, text), None, "{text:?}"));
    ["0", "513", "0/1", "32769/1", ""]
        .into_iter()
        .for_each(|text| assert!(reason(address, text).is_some(), "{text:?}"));
}
