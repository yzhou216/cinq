// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Levels at which an audio or video cue sends to each output of the show
//!
//! Each output has a field, which holds the level that the cue sends at,
//! or nothing where the cue does not send there.  Typing a level sends the
//! cue there, and emptying the field takes the send away.  The fields lie
//! side by side in rows that wrap, so that most of the outputs show at once,
//! and scroll within the inspector, so that every output stays in reach, as
//! the 32 outputs of a Dante Virtual Soundcard need.  The name of an output
//! that cannot play now shows in yellow where cinq tries its device again
//! by itself, and in red where the output must change, followed in words by
//! what is wrong with it, and the header tells how many of the outputs that
//! the cue sends to cannot, so that the color is never the only sign.

#[cfg(test)]
mod tests;

use iced::{
    Center, Element, Fill,
    widget::{column, row, scrollable, text, text::Wrapping},
};
use unicode_width::UnicodeWidthStr;

use super::{
    LEVEL_ROOM,
    output::{refused, styled, typing},
    rejection,
};
use crate::{
    back::{Cue, Fault, Sends},
    front::{
        App, Message, Setting,
        view::{SENDS, SPACING, button, char_width, faults, format, widget::shortened},
    },
};

/// Returns the faults of the outputs that cannot play and that the cue
/// sends to, since an output that it sends nothing to cannot spoil it
fn silent<'a>(app: &'a App, sends: &Sends) -> Vec<&'a Fault> {
    app.workspace
        .outputs()
        .iter()
        .filter(|output| sends.contains_key(&output.id))
        .filter_map(|output| faults::fault(app, output.id))
        .collect()
}

/// Columns that the name of an output keeps in the sends, which lie in rows
/// that wrap, so that more of them show.  A fault told after the name is not
/// cut.
const NAME_ROOM: usize = 20;

/// Space between the entries of a row, wider than the space inside an entry,
/// where the name is padded to the longest, so that a name reads with its own
/// field and not with the unit before it
const ENTRY_SPACING: f32 = 3.0 * SPACING;
const _: () = assert!(ENTRY_SPACING > 2.0 * SPACING);

/// Returns the name of an output, cut to [`NAME_ROOM`] columns, with its
/// fault, if it has one, in words, as `House L (not found)`, so that the
/// color of the name is not the only sign
fn named(name: &str, fault: Option<&Fault>) -> String {
    let name = shortened(name, NAME_ROOM);
    match fault {
        Some(fault) => format!("{name} ({})", fault.brief()),
        None => name,
    }
}

/// Returns the columns of the widest of `names`, as the font shows them, in
/// which a wide character such as a Chinese one takes two
fn widest(names: &[String]) -> usize {
    names.iter().map(|name| name.width()).max().unwrap_or(0)
}

pub(super) fn view<'a>(app: &'a App, cue: &'a Cue) -> Element<'a, Message> {
    let sends = app.workspace.sends(cue);
    let names: Vec<String> = app
        .workspace
        .outputs()
        .iter()
        .map(|output| named(&output.name, faults::fault(app, output.id)))
        .collect();
    // Names take the room of the longest, so that the fields line up and every
    // send takes the same width in the rows that wrap
    let name_room = char_width() * widest(&names) as f32;
    let outputs = app
        .workspace
        .outputs()
        .iter()
        .zip(names)
        .map(|(output, name)| {
            let level = sends.get(&output.id).map(|level| format::decibels(*level));
            let setting = Setting::Send(cue.id, output.id);
            let (field, shown) = typing(app, setting, "off", level.unwrap_or_default(), LEVEL_ROOM);
            let valid = rejection(app, setting, &shown).is_none();
            let field = styled(app, field, &shown, "off", LEVEL_ROOM, valid);
            let fault = faults::fault(app, output.id);
            let name = text(name)
                .width(name_room)
                .wrapping(Wrapping::None)
                .style(match fault {
                    Some(fault) if fault.retried() => text::warning,
                    Some(_) => text::danger,
                    None => text::default,
                });
            row![name, field, text("dB")]
                .spacing(SPACING)
                .align_y(Center)
                .into()
        });
    let silent = silent(app, &sends);
    // The line that tells what is wrong with a send takes the place of the
    // count, since no room is left beside a field in a row of its own
    let told = match refusal(app, cue) {
        Some(reason) => refused(&reason),
        None => text(match silent.len() {
            0 => String::new(),
            count => format!("{} cannot play now", format::count(count, "output")),
        })
        .style(if silent.iter().all(|fault| fault.retried()) {
            text::warning
        } else {
            text::danger
        }),
    };
    let header = row![
        text("Outputs"),
        button("Edit outputs").on_press(Message::ShowOutputs),
        told
    ]
    .spacing(SPACING)
    .align_y(Center);
    column![
        header,
        // Bar beside the fields, rather than over the last of each row
        scrollable(
            row(outputs)
                .spacing(ENTRY_SPACING)
                .wrap()
                .vertical_spacing(SPACING / 2.0)
        )
        .spacing(SPACING)
        .id(SENDS)
        .height(Fill)
    ]
    .spacing(SPACING)
    .into()
}

/// Returns the reason that the text typed into the field of a send of `cue`
/// is not applied, if it is not
fn refusal(app: &App, cue: &Cue) -> Option<String> {
    app.typed
        .as_ref()
        .filter(|(setting, _)| matches!(setting, Setting::Send(id, _) if *id == cue.id))
        .and_then(|(setting, typed)| rejection(app, *setting, typed))
}
