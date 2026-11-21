// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cue lists, running cues, warnings, and the audio device, all visible at
//! the same time, with the warnings above the audio device, which a show sets
//! once, so that the first warning shows whole at the least height of the
//! window

mod devices;

use iced::{
    Element, Fill,
    widget::{
        Radio, button::DEFAULT_PADDING, column, container, progress_bar, radio, row, rule,
        scrollable, text, text::Wrapping,
    },
};
use unicode_width::UnicodeWidthStr;

use super::{
    CURRENT_LIST, SIDEBAR_SCROLL, SPACING, button, char_width,
    cue_table::{RADIO, SCROLL_BAR},
    faults::{self, Warning},
    format, outputs, warnings,
    widget::shortened,
};
use crate::{
    back::{Active, Cue},
    front::{App, Message},
};

/// Thickness of the progress bars of running cues, thin enough that several
/// running cues fit in the sidebar at once
const BAR: f32 = 10.0;

/// Most warnings that the sidebar lists, so that the Default device list
/// below them stays in view, the rest being counted in a line of their own
const WARNINGS: usize = 3;

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
                .map(|cue| active_cue(active, cue, columns as usize))
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
        .map(|problem| text(problem).style(text::danger).into());
    let files = [&app.file_problem, &app.keep_problem, &app.device_problem]
        .into_iter()
        .flatten()
        .map(|problem| text(problem).style(text::warning).into());
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
    let everything: Vec<Element<'_, Message>> = outputs_failing
        .chain(network)
        .chain(files)
        .chain(settings)
        .chain(problems)
        .chain(stranded)
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
fn current<'a>(entry: Radio<'a, Message>, shows: bool) -> Element<'a, Message> {
    if shows {
        container(entry).id(CURRENT_LIST).into()
    } else {
        entry.into()
    }
}

/// Returns a warning that `message` answers when it is pressed
fn pressing(told: String, message: Message) -> Element<'static, Message> {
    button(text(told)).on_press(message).width(Fill).into()
}

/// Returns the name of a running cue and how it runs, each cut to `columns`
/// at the end, so that the level goes before the phase, and an entry keeps
/// its two lines as a fade starts.  The name leaves room for the Stop button.
fn entry(active: &Active, cue: &Cue, columns: usize) -> (String, String) {
    let stop = (4.0 + (2.0 * DEFAULT_PADDING.left + SPACING) / char_width()).ceil();
    (
        shortened(&cue.to_string(), columns.saturating_sub(stop as usize)),
        shortened(&format::activity(active, cue), columns),
    )
}

fn active_cue<'a>(active: &Active, cue: &Cue, columns: usize) -> Element<'a, Message> {
    let (name, how) = entry(active, cue, columns);
    column![
        row![
            text(name).width(Fill).wrapping(Wrapping::None),
            button("Stop").on_press(Message::Stop(cue.id)),
        ]
        .spacing(SPACING),
        progress_bar(0.0..=1.0, active.progress()).girth(BAR),
        text(how).wrapping(Wrapping::None).style(if active.paused {
            text::warning
        } else {
            text::success
        }),
    ]
    .spacing(SPACING / 2.0)
    .into()
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

#[cfg(test)]
mod tests {
    use unicode_width::UnicodeWidthStr;

    use super::{entry, label, limited};
    use crate::back::{Command, Cue, CueId, CueList, Kind, Workspace, loaded, sound};

    #[test]
    fn a_long_list_name_gives_way_to_the_count_of_its_cues() {
        assert_eq!(label("Main", 12, 20), "Main (12)");
        assert_eq!(label("Act one, scene two", 7, 12), "Act one\u{2026} (7)");
        assert!(label(&"x".repeat(60), 120, 16).ends_with(" (120)"));
    }

    #[test]
    fn a_running_cue_keeps_two_lines_and_loses_its_level_first() {
        let cue = Cue::new(CueId(1), "1", Kind::Audio, "Rain")
            .file("rain.ogg")
            .looping(true)
            .build();
        let running = Workspace::new(vec![CueList::new("Main", vec![cue])])
            .apply(loaded(1, "rain.ogg", sound(10.0)))
            .apply(Command::Go { list: 0 })
            .apply(Command::Tick(1.0))
            .apply(Command::Panic)
            .apply(Command::Tick(0.1));
        let active = &running.active()[0];
        let cue = running.cue(CueId(1)).expect("the cue is there");
        assert_eq!(entry(active, cue, 40).0, "1 Rain");
        let (_, how) = entry(active, cue, 18);
        assert!(how.width() <= 18 && how.starts_with("looping, fading"));
        assert!(how.ends_with('\u{2026}'));
    }

    #[test]
    fn the_sidebar_lists_three_warnings_and_counts_the_rest() {
        assert_eq!(limited(vec![1, 2, 3]), (vec![1, 2, 3], 0));
        assert_eq!(limited(vec![1, 2, 3, 4, 5, 6, 7, 8]), (vec![1, 2, 3], 5));
        assert_eq!(limited(Vec::<u8>::new()), (Vec::new(), 0));
    }
}
