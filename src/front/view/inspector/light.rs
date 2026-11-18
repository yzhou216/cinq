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
    output::{help, typing},
    rejection,
};
use crate::{
    back::{Cue, Instrument, PatchEdit},
    front::{
        App, Message, Setting,
        view::{button, theme},
    },
};

/// Characters that `32768/512` and `unchanged` take, with one to spare
const ROOM: f32 = 10.0;

pub(super) fn patch(app: &App) -> Element<'_, Message> {
    let editable = app.editable();
    let edit = move |edit| editable.then_some(Message::Patch(edit));
    let reason = app
        .typed
        .as_ref()
        .filter(|(setting, _)| matches!(setting, Setting::Address(_)))
        .and_then(|(setting, typed)| rejection(app, *setting, typed));
    let instruments = table::table(
        [
            table::column(text("Name"), move |instrument: &Instrument| {
                let id = instrument.id;
                text_input("", &instrument.name).on_input_maybe(
                    editable.then_some(move |name| Message::Patch(PatchEdit::Name(id, name))),
                )
            })
            .width(Fill),
            table::column(text("Address"), |instrument: &Instrument| {
                input(
                    app,
                    Setting::Address(instrument.id),
                    instrument.address_text(),
                    "",
                )
                .0
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
    column![
        row![
            text(format!("Light patch of the show{}", lock(app))),
            space::horizontal(),
            button("Add instrument").on_press_maybe(edit(PatchEdit::Add)),
        ]
        .align_y(Center),
        scrollable(instruments).height(Fill),
    ]
    // Below the table, since the column of addresses is too narrow for it
    .extend(reason.map(|reason| help("", Some(reason))))
    .spacing(SPACING)
    .width(Fill)
    .into()
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
    scrollable(instruments).height(Fill).into()
}

/// Returns the field of `setting`, red while the text in it is not applied,
/// with the reason that it is not
fn input<'a>(
    app: &App,
    setting: Setting,
    value: String,
    placeholder: &str,
) -> (text_input::TextInput<'a, Message>, Option<String>) {
    let (input, shown) = typing(app, setting, placeholder, value, ROOM);
    let rejection = rejection(app, setting, &shown);
    (input.style(theme::field(rejection.is_none())), rejection)
}

/// Returns the field of `setting`, with beside it the reason that the text
/// in it is not applied, if it is not
fn field<'a>(
    app: &App,
    setting: Setting,
    value: String,
    placeholder: &str,
) -> Element<'a, Message> {
    let (input, rejection) = input(app, setting, value, placeholder);
    row![input, help("", rejection)]
        .spacing(SPACING)
        .align_y(Center)
        .into()
}
