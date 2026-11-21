// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Light patch of the show, and the levels of a light cue, each by instrument

use iced::{
    Center, Element, Fill,
    widget::{column, row, scrollable, space, table, text, text_input},
};

use super::{
    SPACING, lock,
    locked::locked,
    output::{address_help, help, styled, typing},
    rejection,
};
use crate::{
    back::{Cue, Instrument, PatchEdit},
    front::{
        App, Message, Setting,
        view::{ADDRESS, button, theme},
    },
};

/// Characters that `32768/512` and `unchanged` take, with one to spare
const ROOM: f32 = 10.0;

#[cfg(test)]
mod tests;

pub(super) fn patch(app: &App) -> Element<'_, Message> {
    let editable = app.editable();
    let edit = move |edit| editable.then_some(Message::Patch(edit));
    let reason = refusal(app, |setting| matches!(setting, Setting::Address(_)));
    let instruments = table::table(
        [
            table::column(text("Name"), move |instrument: &Instrument| {
                let id = instrument.id;
                if editable {
                    text_input("", &instrument.name)
                        .style(theme::field(true))
                        .on_input(move |name| Message::Patch(PatchEdit::Name(id, name)))
                        .into()
                } else {
                    locked(&instrument.name, "", None, Fill)
                }
            })
            .width(Fill),
            table::column(text("Address"), |instrument: &Instrument| {
                field(
                    app,
                    Setting::Address(instrument.id),
                    instrument.address_text(),
                    "",
                )
            }),
            table::column(text("Now"), |instrument: &Instrument| {
                text(format!("{:.0}%", app.workspace.light_level(instrument.id)))
            }),
            table::column(text(""), move |instrument: &Instrument| {
                button("Remove").on_press_maybe(edit(PatchEdit::Remove(instrument.id)))
            }),
        ],
        app.workspace.patch(),
    );
    let body: Element<'_, Message> = if app.workspace.patch().is_empty() {
        text("The light patch has no instruments yet").into()
    } else {
        scrollable(instruments).height(Fill).into()
    };
    // Below the table, since the column of addresses is too narrow for it.  A
    // row of its own takes the room of an instrument at 1024x768, so it shows
    // only while it helps.
    let help = shows_help(app, &reason).then(|| help(address_help(), reason));
    column![
        row![
            text(format!("Light patch of the show{}", lock(app))),
            space::horizontal(),
            button("Add instrument").on_press_maybe(edit(PatchEdit::Add)),
        ]
        .align_y(Center),
        body,
    ]
    .extend(help)
    .spacing(SPACING)
    .width(Fill)
    .into()
}

/// Returns whether the help on addresses shows, which it does while an address
/// has the keyboard or one is refused
fn shows_help(app: &App, refusal: &Option<String>) -> bool {
    app.address_held || refusal.is_some()
}

pub(super) fn levels<'a>(app: &'a App, cue: &'a Cue) -> Element<'a, Message> {
    if app.workspace.patch().is_empty() {
        return text("The light patch has no instruments yet").into();
    }
    let instruments = table::table(
        [
            table::column(text("Instrument"), |instrument: &Instrument| {
                text(instrument.to_string())
            })
            .width(Fill),
            table::column(text("Level (%)"), move |instrument: &Instrument| {
                let level = cue
                    .lights
                    .get(&instrument.id)
                    .map_or_else(String::new, ToString::to_string);
                field(
                    app,
                    Setting::Light(cue.id, instrument.id),
                    level,
                    "unchanged",
                )
            })
            .width(Fill),
        ],
        app.workspace.patch(),
    );
    // Under the table, since a reason wrapped in a cell would move the rows
    let reason = refusal(
        app,
        |setting| matches!(setting, Setting::Light(id, _) if *id == cue.id),
    );
    column![scrollable(instruments).height(Fill)]
        .extend(reason.map(|reason| help("", Some(reason))))
        .spacing(SPACING)
        .into()
}

/// Returns the reason that the text typed into the latest field is not
/// applied, if the field is of a setting that `of` accepts and it is not
fn refusal(app: &App, of: impl Fn(&Setting) -> bool) -> Option<String> {
    app.typed
        .as_ref()
        .filter(|(setting, _)| of(setting))
        .and_then(|(setting, typed)| rejection(app, *setting, typed))
}

/// Returns the field of `setting`, red while the text in it is not applied
fn field<'a>(
    app: &App,
    setting: Setting,
    value: String,
    placeholder: &str,
) -> Element<'a, Message> {
    let (input, shown) = typing(app, setting, placeholder, value, ROOM);
    let input = match setting {
        Setting::Address(_) => input.id(ADDRESS),
        _ => input,
    };
    let valid = rejection(app, setting, &shown).is_none();
    styled(app, input, &shown, placeholder, ROOM, valid)
}
