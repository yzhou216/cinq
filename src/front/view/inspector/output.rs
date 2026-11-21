// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Settings of the output that the outputs view has selected
//!
//! A name that another output has, channels that are neither one nor two,
//! and a level out of range stay in their fields, marked, without applying,
//! as a level of a cue does.  Each field is as wide as what it holds, and
//! the line beside it, in dim text, tells what the setting takes and what it
//! does.  That line turns red and tells why where the value is not applied,
//! so that the color is never the only sign.

mod device;
mod reasons;
pub(super) use reasons::address_help;
pub(in crate::front) use reasons::rejection;
#[cfg(test)]
mod tests;

use iced::{
    Center, Element, Fill, Padding,
    widget::{Text, column, container, row, scrollable, text, text_input},
};

use super::{LEVEL_ROOM, lock, locked::locked};
use crate::{
    back::Output,
    front::{
        App, Message, Setting,
        view::{FORMS, OUTPUT_NAME, SPACING, char_width, dropdown_width, format, theme},
    },
};

/// Columns of the font that the labels of the settings take, as many as
/// the longest of them
const LABEL: f32 = 14.0;

/// Columns of the font that the name field holds, with room for a long
/// name of a speaker
const NAME_ROOM: f32 = 24.0;

/// Columns of the font that the channels field holds, as many as `256,256`
/// takes, with one to spare
const CHANNELS_ROOM: f32 = 8.0;

/// Columns of the font that the device list shows of a name, as many as
/// `Dante Virtual Soundcard, not found` takes, so that most device names
/// show whole and a longer one ends in an ellipsis
const DEVICE_ROOM: f32 = 34.0;

/// Returns the width of a field that holds `columns` columns of text
pub(super) fn room(columns: f32) -> f32 {
    char_width() * columns + text_input::DEFAULT_PADDING.x()
}

/// Returns the field of `setting` that holds `columns` columns of text, with
/// the text that it shows: what was typed into it, or else `value`
pub(super) fn typing<'a>(
    app: &App,
    setting: Setting,
    placeholder: &str,
    value: String,
    columns: f32,
) -> (text_input::TextInput<'a, Message>, String) {
    let shown = match &app.typed {
        Some((typed, text)) if *typed == setting => text.clone(),
        _ => value,
    };
    let input = text_input(placeholder, &shown)
        .on_input_maybe(
            app.editable()
                .then_some(move |text| Message::SettingTyped(setting, text)),
        )
        .width(room(columns));
    (input, shown)
}

/// Returns `input` as `typing` made it, in the style of whether `valid`, while
/// the show can be edited, or else `shown` in a box that the keyboard skips
pub(super) fn styled<'a>(
    app: &App,
    input: text_input::TextInput<'a, Message>,
    shown: &str,
    placeholder: &str,
    columns: f32,
    valid: bool,
) -> Element<'a, Message> {
    if app.editable() {
        input.style(theme::field(valid)).into()
    } else {
        locked(shown, placeholder, None, room(columns).into())
    }
}

/// Returns the reason that a value is not applied, in red
pub(super) fn refused<'a>(reason: &str) -> Text<'a> {
    text(format!("Not applied: {reason}")).style(text::danger)
}

/// Returns the line of help beside a setting: `hint`, in dim text, or the
/// reason that the value is not applied, in red, where there is one
pub(super) fn help<'a>(hint: impl Into<String>, rejection: Option<String>) -> Element<'a, Message> {
    match rejection {
        Some(reason) => refused(&reason).width(Fill).into(),
        None => text(hint.into()).style(text::secondary).width(Fill).into(),
    }
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let Some(output) = app.output.and_then(|id| app.workspace.output(id)) else {
        return text("Select an output to inspect it").into();
    };
    let id = output.id;
    // Field for a setting, with the reason that the text in it is not
    // applied, if it is not
    let field = |setting: Setting,
                 placeholder: &str,
                 value: String,
                 columns: f32,
                 reject: &dyn Fn(&str) -> Option<String>| {
        let (input, shown) = typing(app, setting, placeholder, value, columns);
        let rejection = reject(&shown);
        (input.style(theme::field(rejection.is_none())), rejection)
    };
    let (name, name_rejection) = field(
        Setting::OutputName(id),
        "Name",
        output.name.clone(),
        NAME_ROOM,
        &|name| reasons::name(app, output, name),
    );
    let (channels, channels_rejection) = field(
        Setting::Channels(id),
        "1-2",
        output.channels.to_string(),
        CHANNELS_ROOM,
        &reasons::channels,
    );
    let (new_cues, new_cues_rejection) = field(
        Setting::NewCues(id),
        "off",
        output.new_cues.map(format::decibels).unwrap_or_default(),
        LEVEL_ROOM,
        &reasons::level,
    );
    let sending = app
        .workspace
        .cues()
        .filter(|cue| cue.kind.plays_file() && app.workspace.sends(cue).contains_key(&id))
        .count();
    let settings = [
        (
            "Name",
            [name.id(OUTPUT_NAME).into(), help("", name_rejection)],
        ),
        (
            "Device",
            [device(app, output), help(device::hint(app, output), None)],
        ),
        (
            "Channels",
            [
                channels.into(),
                help(reasons::channels_help(), channels_rejection),
            ],
        ),
        (
            "New cues send",
            [
                // Unit keeps a gap from the help that follows it, which
                // would otherwise read on from it
                container(row![new_cues, text("dB")].spacing(SPACING).align_y(Center))
                    .padding(Padding::ZERO.right(SPACING))
                    .into(),
                help(
                    "New cues send here at this level; empty for none",
                    new_cues_rejection,
                ),
            ],
        ),
    ];
    // Scrolls, so that no field is cut to a sliver in a low or narrow window
    // and Tab can reach one below the part in view
    scrollable(
        column![text(format!("Output: {}{}", output.name, lock(app)))]
            .extend(settings.into_iter().map(|(label, cells)| {
                row![text(label).width(char_width() * LABEL)]
                    .extend(cells)
                    .spacing(SPACING)
                    .align_y(Center)
                    .into()
            }))
            .push(text(match sending {
                1 => "1 cue sends here".to_owned(),
                count => format!("{count} cues send here"),
            }))
            .spacing(SPACING)
            .width(Fill),
    )
    .spacing(SPACING)
    .id(FORMS)
    .width(Fill)
    .height(Fill)
    .into()
}

/// Returns the list of the devices that the output can play on, as wide
/// as its name and no wider, whichever device it holds
fn device<'a>(app: &'a App, output: &'a Output) -> Element<'a, Message> {
    container(device::view(app, output))
        .max_width(dropdown_width(DEVICE_ROOM))
        .into()
}
