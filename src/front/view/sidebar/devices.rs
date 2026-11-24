// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! List of the audio devices, which chooses the default device of the
//! machine, the one that outputs on the default device play on

use iced::Element;

use crate::{
    back::Device,
    front::{
        App, Message,
        view::{
            devices::{Choice, choices, default_label},
            dropdown, faults, keep_keys, outputs, theme,
        },
    },
};

/// Returns what a warning calls `device`: the ID of the device that the
/// Default device list chooses where the warning is about the default, so
/// that the warning names what to fix
pub(super) fn named(app: &App, device: &Device) -> String {
    match device {
        Device::Default => default_label(app),
        device => outputs::device(device).to_owned(),
    }
}

/// Returns whether an output on the default device cannot play, while the
/// list shows a device that it does not find
pub(super) fn missing(app: &App) -> bool {
    app.workspace
        .outputs()
        .iter()
        .any(|output| output.device == Device::Default && faults::fault(app, output.id).is_some())
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let options = choices(&app.audio_devices, "System default");
    let chosen = app.config.sound.device.as_deref();
    let selected = options
        .iter()
        .find(|option| option.id.as_deref() == chosen)
        .cloned();
    let keymap = &app.config.keymap;
    // Device that is not there reads in yellow, as its warning does, rather
    // than in the gray of a list where nothing is chosen
    let lost = selected.is_none() && missing(app);
    let list = dropdown(
        options,
        selected,
        |choice: Choice| Message::ChooseAudioDevice(choice.id),
        keymap,
    )
    // Device that is not there goes by its ID, which the warnings name
    .placeholder(chosen.unwrap_or_default())
    .on_open(Message::ListAudioDevices)
    // Outputs on the default device play on the device that this list
    // chooses, which the Device list of an output tells in blue while its
    // highlight rests on the default
    .lit(app.menu && app.default_highlighted);
    let list = if lost {
        list.placeholder_color(theme::PALETTE.warning)
    } else {
        list
    };
    keep_keys(list, keymap)
}
