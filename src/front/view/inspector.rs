// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Inspector, which shows every setting of the selected cue on three tabs,
//! and the light patch of the show on a fourth, or those of the selected
//! output, without tabs, while the outputs show

mod field;
mod forms;
mod header;
mod light;
mod locked;
mod output;
pub(in crate::front) use output::rejection;
use output::{refused, styled, typing};
mod sends;

use iced::{
    Center, Element, Size,
    widget::{column, container, progress_bar, row, rule, slider, text},
};
use strum::VariantArray;

use super::{INSPECTOR, SPACING, char_width, check, cue_table::RADIO, format, keep_keys, radio};
use crate::{
    back::{Cue, Edit, Kind, LEVELS, Media},
    front::{App, Message, Setting, Tab},
};
use field::Field;
use forms::forms;
use header::inspected;

/// Characters that the level field holds, as many as `-60.0` takes, with
/// one to spare
const LEVEL_ROOM: f32 = 6.0;

/// Least height of the inspector, which a window of 1280 by 960 pixels or
/// less keeps
const LEAST_HEIGHT: f32 = 240.0;

/// Fields of the Basics tab, one column after the other
const BASICS: [&[Field]; 2] = [
    &[Field::Number, Field::Name, Field::Target, Field::Notes],
    &[
        Field::PreWait,
        Field::Duration,
        Field::PostWait,
        Field::Then,
    ],
];

/// Shares of the width that the two forms of the Basics tab take: the first
/// holds the name and the file, which are long, and the second numbers and a
/// drop-down list, which need less
const BASICS_SHARES: [u16; 2] = [3, 2];

/// Fields of the Time & Loops tab, one column after the other: the clip with
/// its length, and the loop within the clip
const TIME: [&[Field]; 2] = [
    &[Field::Start, Field::End, Field::Duration],
    &[Field::LoopStart, Field::LoopEnd],
];

/// Returns the height of the inspector in a window `window` high, which is
/// [`LEAST_HEIGHT`] or a quarter of the window, whichever is more, so that a
/// tall window shows more sends, instruments, and levels at once
fn height(window: f32) -> f32 {
    LEAST_HEIGHT.max(window / 4.0)
}

/// Returns the columns of text that fit in a line of the page beside the tabs,
/// in a window `width` wide
fn columns(width: f32) -> usize {
    let tabs = Tab::VARIANTS
        .iter()
        .map(|tab| tab.to_string().chars().count())
        .max()
        .unwrap_or_default();
    let page = width - 4.0 * SPACING - 1.0 - RADIO - tabs as f32 * char_width();
    (page / char_width()).max(0.0) as usize
}

pub(super) fn view(app: &App, window: Size) -> Element<'_, Message> {
    let columns = columns(window.width);
    let tabs = column![text("Inspector")]
        .extend(
            Tab::VARIANTS
                .iter()
                .filter(|_| !app.outputs_shown)
                .map(|&tab| radio(tab.to_string(), tab, Some(app.tab), Message::Tab)),
        )
        .spacing(SPACING);
    let body = match (app.tab, app.selected.and_then(|id| app.workspace.cue(id))) {
        _ if app.outputs_shown => output::view(app),
        (Tab::Patch, _) => light::patch(app),
        (Tab::Basics, Some(cue)) => inspected(
            app,
            cue,
            columns,
            &BASICS,
            forms(app, cue, BASICS, BASICS_SHARES, || None),
        ),
        (Tab::Time, Some(cue)) => inspected(
            app,
            cue,
            columns,
            if cue.kind.plays_file() { &TIME } else { &[] },
            time(app, cue),
        ),
        (Tab::Levels, Some(cue)) => inspected(app, cue, columns, &[], levels(app, cue)),
        (_, None) if app.current().cues.is_empty() => {
            text("This list has no cues to inspect").into()
        }
        (_, None) => text("Select a cue to inspect it").into(),
    };
    keep_keys(
        container(
            row![tabs, rule::vertical(1), body]
                .spacing(SPACING)
                .padding(SPACING)
                .height(height(window.height)),
        )
        .id(INSPECTOR),
        &app.config.keymap,
    )
}

fn lock(app: &App) -> &'static str {
    if app.editable() {
        ""
    } else {
        " (read-only in show mode)"
    }
}

/// Returns what the Time & Loops tab says of a cue that plays no file, which
/// for a light cue tells where its fade time is
fn no_file(kind: Kind) -> String {
    match kind {
        kind @ (Kind::Light | Kind::Fade) => {
            format!("{kind} cues play no file; the Duration on the Basics tab is the fade time")
        }
        Kind::Wait => {
            "Wait cues play no file; the Duration on the Basics tab is how long they wait"
                .to_owned()
        }
        kind => format!("{kind} cues play no file"),
    }
}

fn time<'a>(app: &'a App, cue: &'a Cue) -> Element<'a, Message> {
    match cue.kind {
        Kind::Picture => {
            return text("Picture cues show their file until something stops them").into();
        }
        kind if !kind.plays_file() => return text(no_file(kind)).into(),
        _ => {}
    }
    let id = cue.id;
    let looping = move || {
        Some(check(
            "Loop until stopped or devamped",
            cue.looping,
            app.editable()
                .then_some(move |looping| Message::Edit(id, Edit::Loop(looping))),
            &app.config.keymap,
        ))
    };
    forms(app, cue, TIME, [1, 1], looping)
}

fn levels<'a>(app: &'a App, cue: &'a Cue) -> Element<'a, Message> {
    let label = match cue.kind {
        Kind::Video if matches!(cue.media, Media::Loaded { sound: None, .. }) => {
            return text("The file of this cue holds no sound").into();
        }
        Kind::Audio | Kind::Video => "Level",
        Kind::Fade => "Fade target to",
        Kind::Light => return light::levels(app, cue),
        Kind::Picture
        | Kind::Start
        | Kind::Stop
        | Kind::Pause
        | Kind::Devamp
        | Kind::GoTo
        | Kind::Network
        | Kind::Group
        | Kind::Wait
        | Kind::Memo => {
            return text(format!("{} cues have no level", cue.kind)).into();
        }
    };
    let id = cue.id;
    let editable = app.editable();
    let (field, typed) = typing(
        app,
        Setting::Level(id),
        "",
        format::decibels(cue.level),
        LEVEL_ROOM,
    );
    let reason = rejection(app, Setting::Level(id), &typed);
    let field = styled(app, field, &typed, "", LEVEL_ROOM, reason.is_none());
    let level: Element<'_, Message> = if editable {
        slider(LEVELS, cue.level, move |db| {
            Message::Edit(id, Edit::Level(db))
        })
        .step(0.5_f32)
        .into()
    } else {
        progress_bar(LEVELS, cue.level).into()
    };
    let stop = (cue.kind == Kind::Fade).then(|| {
        check(
            "Stop target when done",
            cue.stop_target,
            editable.then_some(move |stop| Message::Edit(id, Edit::StopTarget(stop))),
            &app.config.keymap,
        )
    });
    column![
        row![text(label), level, field, text("dB")]
            .extend(reason.map(|reason| refused(&reason).into()))
            .spacing(SPACING)
            .align_y(Center),
        stop,
        cue.kind.plays_file().then(|| sends::view(app, cue)),
    ]
    .spacing(SPACING)
    .into()
}

#[cfg(test)]
mod tests {
    use super::{LEAST_HEIGHT, height, no_file};
    use crate::back::Kind;

    #[test]
    fn a_cue_with_a_duration_tells_on_the_time_tab_where_it_is() {
        assert!(no_file(Kind::Light).contains("Duration on the Basics tab"));
        assert!(no_file(Kind::Fade).contains("fade time"));
        assert!(no_file(Kind::Wait).contains("how long they wait"));
        assert_eq!(no_file(Kind::Memo), "Memo cues play no file");
    }

    #[test]
    fn the_inspector_keeps_its_least_height_until_a_quarter_of_the_window_is_more() {
        assert_eq!(height(768.0), LEAST_HEIGHT);
        assert_eq!(height(960.0), LEAST_HEIGHT);
        assert_eq!(height(1440.0), 360.0);
    }
}
