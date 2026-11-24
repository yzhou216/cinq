// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Light patch of the show, and the levels of a light cue, each by instrument

use iced::{
    Center, Element, Fill, Padding,
    widget::{
        button::DEFAULT_PADDING, column, container, responsive, row, scrollable, space, table,
        text, text_input,
    },
};
use unicode_width::UnicodeWidthStr;

use super::{
    SPACING, lock,
    locked::locked,
    output::{address_help, help, styled, typing},
    rejection,
};
use crate::{
    back::{Cue, Instrument, PatchEdit},
    front::{
        App, Message, Setting, Tab,
        view::{
            ADDRESS, button, char_width,
            cue_table::{CELL_SPACE, SCROLL_BAR},
            reached, theme,
            widget::hint,
        },
    },
};

/// Characters that `32768/512` and `unchanged` take, with one to spare
const ROOM: f32 = 10.0;

/// Columns of the font that the name of an instrument keeps before the Now
/// column gives way, enough to read what a name starts with
const NAME_ROOM: f32 = 16.0;

#[cfg(test)]
mod tests;

pub(super) fn patch(app: &App) -> Element<'_, Message> {
    responsive(move |size| page(app, size.width)).into()
}

/// Returns whether the table of the patch keeps its Now column in `room`
/// pixels, which it does while the Name column keeps [`NAME_ROOM`] columns
/// beside the others
fn now_fits(room: f32) -> bool {
    let button = "Remove".width() as f32 * char_width() + 2.0 * DEFAULT_PADDING.x();
    with_now(room, button + address_room(), 4.0)
}

/// Returns whether a table keeps its Now column in `room` pixels, beside a
/// Name column of [`NAME_ROOM`] columns, `others` pixels of other columns,
/// and the cell spaces of its `columns`
fn with_now(room: f32, others: f32, columns: f32) -> bool {
    let now = "100%".width() as f32 * char_width();
    let name = NAME_ROOM * char_width() + text_input::DEFAULT_PADDING.x();
    name + now + others + columns * CELL_SPACE + SCROLL_BAR <= room
}

/// Returns whether the table of levels keeps its Now column in `room` pixels
fn levels_now_fits(room: f32) -> bool {
    with_now(room, address_room(), 3.0)
}

/// Returns the level that the stage has now at an instrument
fn now_cell<'a>(app: &App, instrument: &Instrument) -> Element<'a, Message> {
    if app
        .workspace
        .patch()
        .iter()
        .all(|patched| patched.id != instrument.id)
    {
        return text("").into();
    }
    text(format!("{:.0}%", app.workspace.light_level(instrument.id))).into()
}

fn address_room() -> f32 {
    super::output::room(ROOM)
}

/// Returns the light patch laid out for an inspector `width` pixels wide
fn page(app: &App, width: f32) -> Element<'_, Message> {
    let editable = app.editable();
    let edit = move |edit| editable.then_some(Message::Patch(edit));
    let reason = refusal(app, |setting| matches!(setting, Setting::Address(_)));
    let name = table::column(text("Name"), move |instrument: &Instrument| {
        let id = instrument.id;
        if editable {
            reached(
                text_input("", &instrument.name)
                    .style(theme::field(true))
                    .on_input(move |name| Message::Patch(PatchEdit::Name(id, name))),
            )
        } else {
            locked(&instrument.name, "", None, Fill)
        }
    })
    .width(Fill);
    let address = table::column(text("Address"), |instrument: &Instrument| {
        field(
            app,
            Setting::Address(instrument.id),
            instrument.address_text(),
            "",
        )
    });
    // Gives way to the name, which the operator reads and edits
    let now = table::column(text("Now"), |instrument: &Instrument| {
        now_cell(app, instrument)
    })
    .align_y(Center);
    let remove = table::column(text(""), move |instrument: &Instrument| {
        button("Remove")
            .on_press_maybe(edit(PatchEdit::Remove(instrument.id)))
            .reachable_and_leave(&app.config.keymap)
    });
    let instruments = table::table(
        [name, address]
            .into_iter()
            .chain(now_fits(width).then_some(now))
            .chain([remove])
            .collect::<Vec<_>>(),
        app.workspace.patch(),
    );
    // Left of the names, room for the labels of hint mode, which the Setting
    // column gives in the other tabs, so that they do not meet the labels of
    // the tabs beside them
    let body: Element<'_, Message> = if app.workspace.patch().is_empty() {
        text("The light patch has no instruments yet").into()
    } else {
        container(scrollable(instruments).height(Fill))
            .padding(Padding::ZERO.left(hint::width(2)))
            .into()
    };
    // Below the table, since the column of addresses is too narrow for it.  A
    // row of its own takes the room of an instrument at 1024x768, so it shows
    // only while it helps.
    let help = shows_help(app, &reason).then(|| help(address_help(), reason));
    column![
        row![
            text(format!("Light patch of the show{}", lock(app))),
            space::horizontal(),
            button("Add instrument")
                .on_press_maybe(edit(PatchEdit::Add))
                .reachable(&app.config.keymap),
        ]
        .spacing(SPACING)
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

/// Returns what the levels tell while no instrument is patched, and where one
/// is added
fn no_instruments() -> String {
    format!(
        "The light patch has no instruments yet.  Add one on the {} tab",
        Tab::Patch
    )
}

pub(super) fn levels<'a>(app: &'a App, cue: &'a Cue) -> Element<'a, Message> {
    if app.workspace.patch().is_empty() && cue.lights.is_empty() {
        return text(no_instruments()).into();
    }
    responsive(move |size| levels_page(app, cue, size.width)).into()
}

/// Returns the rows of the levels of `cue`: the instruments of the patch, then
/// each that the cue names and the patch lacks, so that the operator can read
/// and clear a level that no instrument takes
fn rows(patch: &[Instrument], cue: &Cue) -> Vec<Instrument> {
    patch
        .iter()
        .cloned()
        .chain(
            cue.lights
                .keys()
                .filter(|id| patch.iter().all(|instrument| instrument.id != **id))
                .map(|id| Instrument {
                    id: *id,
                    name: format!("{} (unpatched)", id.0),
                    universe: 1,
                    address: 1,
                }),
        )
        .collect()
}

/// Returns the levels of `cue` laid out for an inspector `width` pixels wide
fn levels_page<'a>(app: &'a App, cue: &'a Cue, width: f32) -> Element<'a, Message> {
    // Beside the level that the cue sets, the level that the stage has now,
    // which gives way first as in the patch
    let now = table::column(text("Now"), |instrument: Instrument| {
        now_cell(app, &instrument)
    })
    .align_y(Center);
    // Columns take the room of their text, so that a field stays near its
    // name instead of across the width
    let instruments = table::table(
        [
            table::column(text("Instrument"), |instrument: Instrument| {
                text(instrument.to_string())
            })
            .align_y(Center),
            table::column(text("Level (%)"), move |instrument: Instrument| {
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
            }),
        ]
        .into_iter()
        .chain(levels_now_fits(width).then_some(now))
        .collect::<Vec<_>>(),
        rows(app.workspace.patch(), cue),
    );
    // Under the table, since a reason wrapped in a cell would move the rows
    let reason = refusal(
        app,
        |setting| matches!(setting, Setting::Light(id, _) if *id == cue.id),
    );
    column![scrollable(instruments).width(Fill).height(Fill)]
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
