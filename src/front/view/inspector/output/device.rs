// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Device list of the output that the outputs view has selected

use iced::Element;

use crate::{
    back::{Device, Fault, Output},
    front::{
        App, Message,
        view::{
            devices::{Choice, choices, chosen},
            dropdown, faults, keep_keys, outputs, theme,
        },
    },
};

/// Returns whether the device of the output is not there
pub(super) fn missing(app: &App, output: &Output) -> bool {
    matches!(faults::fault(app, output.id), Some(Fault::Missing(_)))
}

/// Returns the list of the devices that the output can play on, which
/// shows a device that is not there by its name
pub(super) fn view<'a>(app: &'a App, output: &'a Output) -> Element<'a, Message> {
    let id = output.id;
    // Device that is not there, which reads so in words and in yellow
    // rather than in the gray of a list where nothing is chosen
    let missing = missing(app, output);
    let name = outputs::device(&output.device);
    let placeholder = if missing {
        format!("{name}, not found")
    } else {
        name.to_owned()
    };
    let options = choices(&app.audio_devices, outputs::device(&Device::Default));
    let selected = chosen(&options, &output.device)
        .cloned()
        .filter(|_| !missing);
    let keymap = &app.config.keymap;
    let list = dropdown(
        options,
        selected,
        move |choice: Choice| {
            let device = match choice.id {
                None => Device::Default,
                Some(device) => Device::Named {
                    id: device,
                    name: choice.name,
                },
            };
            Message::OutputDevice(id, device)
        },
        keymap,
    )
    .placeholder(placeholder);
    let list = if missing {
        list.placeholder_color(theme::PALETTE.warning)
    } else {
        list
    }
    .on_open(Message::ListAudioDevices);
    keep_keys(list, keymap)
}

/// Returns the help beside the list: what to do where the device is not
/// there, and nothing where it is
pub(super) fn hint(app: &App, output: &Output) -> &'static str {
    if missing(app, output) {
        "Connect the device, or pick another"
    } else {
        ""
    }
}
