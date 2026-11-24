// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cue lists, running cues, warnings, and the audio device, all visible at
//! the same time, with the warnings above the audio device, which a show sets
//! once, so that the first warning shows whole at the least height of the
//! window

mod devices;
mod running;
#[cfg(test)]
mod tests;

use iced::{
    Element, Fill,
    widget::{column, container, rule, scrollable, text},
};
use unicode_width::UnicodeWidthStr;

use super::{
    CURRENT_LIST, SIDEBAR, SIDEBAR_SCROLL, SPACING, button, char_width,
    cue_table::{RADIO, SCROLL_BAR},
    faults::{self, Warning},
    format, outputs, radio, warnings,
    widget::shortened,
};
use crate::front::{App, Message};

/// Most warnings that the sidebar lists, so that the Default device list
/// below them stays in view, the rest being counted in a line of their own
const WARNINGS: usize = 3;

/// Least width of the sidebar, enough for the name of a short cue list, such
/// as `Act 1`, with its count beside the radio button, so that a narrow
/// window does not cut the names of the lists to the same two letters
const LEAST: f32 = 160.0;

/// Returns the width of the sidebar in a window `window` wide, which is a
/// quarter of it, so that a narrow window, such as one tiled beside another,
/// leaves the cue list room, but at least [`LEAST`] and at most [`SIDEBAR`]
pub(super) fn width(window: f32) -> f32 {
    (window / 4.0).clamp(LEAST, SIDEBAR)
}

pub(super) fn view(app: &App, width: f32) -> Element<'_, Message> {
    let workspace = &app.workspace;
    let columns = (width - 2.0 * SPACING - SCROLL_BAR) / char_width();
    // Each list takes one line, so that the rest of the sidebar stays in view,
    // and the inspector holds the whole name
    let room = columns - RADIO / char_width();
    let lists = workspace.lists().iter().enumerate().map(|(index, list)| {
        let entry = radio(
            label(&list.name, list.cues.len(), room as usize),
            index,
            (!app.outputs_shown).then_some(app.list),
            Message::SwitchList,
        );
        current(entry, !app.outputs_shown && index == app.list)
    });
    let active = workspace
        .active()
        .iter()
        .filter_map(|active| {
            workspace
                .cue(active.id)
                .map(|cue| running::active_cue(active, cue, columns as usize))
        })
        .collect::<Vec<_>>();
    let outputs_failing = faults::warnings(app).into_iter().map(|warning| {
        let told = match &warning {
            Warning::Device {
                device,
                fault,
                outputs: affected,
            } => {
                let state = fault.state().to_lowercase();
                let reason = fault
                    .reason()
                    .map(|reason| format!(": {reason}"))
                    .unwrap_or_default();
                let count = format::count(affected.len(), "output");
                format!("{}: {state} ({count}){reason}", outputs::device(device))
            }
            Warning::Channels { output, fault } => {
                format!("{}: {}", output.name, fault.state().to_lowercase())
            }
        };
        pressing(told, Message::InspectOutput(warning.output()))
    });
    let network = app
        .network_problems
        .iter()
        .chain(&app.osc_problem)
        .map(|problem| text(problem).style(text::danger).into());
    let files = [&app.file_problem, &app.keep_problem, &app.device_problem]
        .into_iter()
        .flatten()
        .map(|problem| text(problem).style(text::warning).into());
    // Latest alone, whatever the flood of commands that did nothing
    let ignored = app
        .osc_ignored
        .iter()
        .map(|line| text(line).style(text::warning).into());
    let settings = app
        .config
        .warnings()
        .map(|warning| text(warning).style(text::warning).into());
    let count = warnings(app);
    let warned = text(format!("Warnings ({count})")).style(if count > 0 {
        text::warning
    } else {
        text::default
    });
    let problems = workspace
        .problems()
        .map(|(cue, problem)| pressing(format!("{cue}: {problem}"), Message::Select(cue.id)));
    let stranded = workspace.stranded().map(|cue| {
        pressing(
            format!("{cue}: sends to no output"),
            Message::Select(cue.id),
        )
    });
    let unlit = workspace
        .unlit_cues()
        .map(|(cue, unlit)| pressing(format!("{cue}: {unlit}"), Message::Select(cue.id)));
    let everything: Vec<Element<'_, Message>> = outputs_failing
        .chain(network)
        .chain(files)
        .chain(ignored)
        .chain(settings)
        .chain(problems)
        .chain(stranded)
        .chain(unlit)
        .collect();
    let (shown, left) = limited(everything);
    let more = (left > 0).then(|| text(format!("and {left} more")).style(text::warning).into());
    scrollable(
        column![
            text("Cue lists"),
            column(lists).spacing(SPACING),
            rule::horizontal(1),
            current(
                radio(
                    format!("Outputs ({})", workspace.outputs().len()),
                    (),
                    app.outputs_shown.then_some(()),
                    |()| Message::ShowOutputs,
                ),
                app.outputs_shown,
            ),
            rule::horizontal(1),
            text(format!("Active ({})", workspace.active().len())),
            // An empty list leaves out its body, as the header tells that it is empty
            (!active.is_empty()).then(|| column(active).spacing(SPACING)),
            rule::horizontal(1),
            warned,
            (!shown.is_empty()).then(|| column(shown.into_iter().chain(more)).spacing(SPACING)),
            rule::horizontal(1),
            text("Default device"),
            devices::view(app),
        ]
        .spacing(SPACING)
        .padding(SPACING)
        .width(Fill),
    )
    .spacing(0)
    .id(SIDEBAR_SCROLL)
    .width(width)
    .height(Fill)
    .into()
}

/// Returns `entry`, marked for the sidebar to scroll to when it `shows`
fn current<'a>(entry: Element<'a, Message>, shows: bool) -> Element<'a, Message> {
    if shows {
        container(entry).id(CURRENT_LIST).into()
    } else {
        entry
    }
}

/// Returns a warning that `message` answers when it is pressed
fn pressing(told: String, message: Message) -> Element<'static, Message> {
    button(text(told)).on_press(message).width(Fill).into()
}

/// Splits `warnings` into the first that the sidebar lists and how many it
/// leaves out
fn limited<T>(warnings: Vec<T>) -> (Vec<T>, usize) {
    let left = warnings.len().saturating_sub(WARNINGS);
    (warnings.into_iter().take(WARNINGS).collect(), left)
}

/// Returns the name of a list with the number of its cues, in `columns`
/// columns, where the name gives way and the number stays whole
fn label(name: &str, count: usize, columns: usize) -> String {
    let count = format!(" ({count})");
    format!(
        "{}{count}",
        shortened(name, columns.saturating_sub(count.width()))
    )
}
