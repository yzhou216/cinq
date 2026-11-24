// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cue list, one row per cue that the list shows, under a header that stays in
//! view
//!
//! A collapsed group takes the row of the cues that it holds, as a closed fold
//! takes one line in Vim, so that the rows drawn are those of
//! [`Groups::shown`], in order.  The selection only drives the inspector,
//! while the State column marks the standby cue that GO fires as NEXT, on the
//! row of the collapsed group that hides it where one does.  Widths are
//! counted in columns of the monospace font, of which a wide character such
//! as one of Chinese takes two, so that the header, a table of its own above
//! the rows that scroll, lines up with them.

mod columns;
mod fitting;
mod layout;
mod name;
#[cfg(test)]
mod tests;

use std::rc::Rc;

use iced::{
    Element, Fill, Length, padding,
    widget::{
        Radio, column, container, mouse_area, radio, responsive, rule, scrollable, space, table,
        text, text::Wrapping,
    },
};

use super::{CUES, SELECTED, faults, marked_row};
use crate::{
    back::{Cue, Groups, Workspace},
    front::{App, Message, keymap::Action},
};
use columns::{Column, counted, shown_lines};
use fitting::fitting;
use layout::Layout;

/// Returns the texts of a cue that a search matches, which are those of the
/// columns of the list apart from the waits, which are timings, and the
/// state, which changes as the show runs, so that a search finds what the list
/// shows of the cue, as Vim finds the text of a line
pub(in crate::front) fn searchable(
    workspace: &Workspace,
    groups: &Groups,
    cue: &Cue,
) -> Vec<String> {
    use Column::{Action, Name, Number, Target, Then, Type};
    [Number, Type, Name, Target, Action, Then]
        .map(|column| column.text(workspace, groups, cue))
        .to_vec()
}

/// Width of the scroll bar, which the list leaves free at its right, so
/// that the bar covers no text and the rows keep the width of the header
/// with or without the bar
pub(super) const SCROLL_BAR: f32 = 10.0;

/// iced's own vertical padding of table cells
pub(super) const ROW_PADDING: f32 = 5.0;

/// iced's own horizontal padding of table cells, which both tables set,
/// since the widths of the columns count it
const CELL_PADDING: f32 = 10.0;

/// Width of the line between columns, which both tables set as well
const SEPARATOR: f32 = 1.0;

pub(super) const CELL_SPACE: f32 = 2.0 * CELL_PADDING + SEPARATOR;

pub(super) const RADIO: f32 = Radio::<Message>::DEFAULT_SIZE + Radio::<Message>::DEFAULT_SPACING;

/// Fewest columns of the font that the list keeps for a name before any
/// column gives way, however short its names
const NAME_LEAST: usize = 16;

/// Most columns of the font that the Target column takes while the room
/// is short, past which a long file name wraps
const TARGET_MOST: usize = 48;

/// Most columns of the font that the Target column grows to when the room
/// leaves a name all that it needs, up to [`NAME_MOST`] columns
const TARGET_GROWN: usize = 80;

/// Most columns of the font that a name is given first call on, before any
/// column that can gives way, so that a name of up to as many reads on one
/// line however wide the window, and a very long name does not keep the Target
/// column narrow in a wide window
const NAME_MOST: usize = 48;

/// Columns of the font that the Target column narrows to before any column
/// gives way, where the list is too narrow for a name to keep what its widest
/// name needs, enough for a long path to wrap onto two lines, since a column
/// that wraps onto more makes every row taller than the name needs
const TARGET_ROOM: usize = 36;

/// Columns of the font that a name moves right for each group that holds its
/// cue
const INDENT: usize = 2;

/// Fewest columns of the font that the State column holds, so that the column
/// keeps its width as cues run: as many as `10:00.0 left` takes, which is the
/// longest state of a cue whose sound is under a hundred minutes.  It grows
/// to what the longest problem of a row takes, such as `NO DESTINATION`, and
/// to `OUTPUT FAILED` while a cue plays on a failed output, which no running
/// cue changes, so that a short list keeps room for the Then column.
const STATE_ROOM: usize = 12;

/// Columns of the font that `OUTPUT FAILED` takes
const FAILED_ROOM: usize = 13;

/// Fewest columns of the font that the State column keeps, as many as
/// `DESTINATION` takes, the longest word of its states, so that a narrow list
/// never breaks a word of it
const STATE_LEAST: usize = 11;

pub(super) fn view(app: &App) -> Element<'_, Message> {
    responsive(move |size| list(app, size.width - SCROLL_BAR)).into()
}

fn list<'a>(app: &'a App, room: f32) -> Element<'a, Message> {
    let layout = Rc::new(Layout::of(app.current()));
    let fitting = fitting(app, &layout, room);
    // Where the Target column is gone, a collapsed group tells its count after its name
    let counting = fitting.iter().all(|&(column, _)| column != Column::Target);
    let table = |titled: bool, cues: Vec<&'a Cue>| {
        let layout = Rc::clone(&layout);
        let columns = fitting.iter().map(move |&(column, width)| {
            table_column(app, column, width, titled, counting, Rc::clone(&layout))
        });
        table::table(columns, cues)
            .padding_x(CELL_PADDING)
            .separator_x(SEPARATOR)
    };
    let header = table(true, Vec::new()).padding_y(ROW_PADDING);
    let rows = if app.current().cues.is_empty() {
        empty(app)
    } else {
        let selected = app.selected.and_then(|id| layout.row_of(id));
        let rows = table(false, layout.rows()).padding_y(0);
        marked_row(rows, fitting.len(), selected, SELECTED)
    };
    scrolling(header, rows, CUES)
}

/// Returns the `header` above the `rows`, which alone scroll in the
/// scrollable `id`, whose bar takes the width that both leave free
pub(super) fn scrolling<'a>(
    header: impl Into<Element<'a, Message>>,
    rows: impl Into<Element<'a, Message>>,
    id: &'static str,
) -> Element<'a, Message> {
    let free = padding::right(SCROLL_BAR);
    column![
        container(header).padding(free),
        rule::horizontal(1),
        scrollable(container(rows).padding(free))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(SCROLL_BAR)
                    .scroller_width(SCROLL_BAR),
            ))
            .id(id)
            .height(Fill),
    ]
    .into()
}

fn empty(app: &App) -> Element<'_, Message> {
    let hint = match app.config.keymap.key(Action::AddBelow) {
        // Cues added here would not be those of the show that failed to open
        _ if app.open_failed && app.file_problem.is_some() => {
            "The show could not be opened.  See the warning in the sidebar.".to_owned()
        }
        _ if !app.editable() => "No cues yet.  Leave show mode to add one.".to_owned(),
        Some(keys) => format!("No cues yet.  Press {keys} or Add cue to add one."),
        None => "No cues yet.  Press Add cue to add one.".to_owned(),
    };
    container(text(hint))
        .padding([ROW_PADDING, CELL_PADDING])
        .into()
}

/// Returns `content` as text that breaks a word too wide for its column
/// rather than draws it over the next column
fn broken<'a>(content: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(content).wrapping(Wrapping::WordOrGlyph)
}

fn table_column<'a>(
    app: &'a App,
    column: Column,
    width: Option<f32>,
    titled: bool,
    counting: bool,
    layout: Rc<Layout<'a>>,
) -> table::Column<'a, 'a, &'a Cue, Message> {
    let header: Element<'a, Message> = if titled {
        text(column.title()).into()
    } else {
        space().into()
    };
    table::column(header, move |cue| cell(app, column, cue, counting, &layout))
        .width(width.map_or(Fill, Length::Fixed))
}

/// Returns the cell of `cue` in `column`, padded inside rather than by the
/// table, so that a click anywhere in it selects the cue, but on the mark of
/// a group, which flips it.  The cell of a name pads its parts itself, so that
/// the mark takes the height of the row.
fn cell<'a>(
    app: &'a App,
    column: Column,
    cue: &'a Cue,
    counting: bool,
    layout: &Layout,
) -> Element<'a, Message> {
    let groups = &layout.groups;
    let vertical = padding::vertical(ROW_PADDING);
    let (content, padded): (Element<'a, Message>, _) = match column {
        Column::Name => {
            let count = counted(&app.workspace, groups, cue).filter(|_| counting);
            let name = column.text(&app.workspace, groups, cue) + &count.unwrap_or_default();
            (
                name::cell(cue, name, layout.depth(cue), layout.gutter()),
                false,
            )
        }
        Column::Number => (
            radio(cue.number.clone(), cue.id, app.selected, Message::Select).into(),
            true,
        ),
        Column::Target => (
            broken(column.text(&app.workspace, groups, cue)).into(),
            true,
        ),
        Column::State => {
            let selected = app.selected == Some(cue.id);
            let failing = faults::failing(app);
            let lines = shown_lines(
                &app.workspace,
                groups,
                cue,
                layout.is_next(cue),
                &failing,
                &app.network_failed,
                selected,
            )
            .into_iter()
            .map(|(line, style)| broken(line).style(style).into());
            (iced::widget::column(lines).into(), true)
        }
        _ => (
            broken(column.text(&app.workspace, groups, cue)).into(),
            true,
        ),
    };
    let cell =
        container(content)
            .width(Fill)
            .padding(if padded { vertical } else { padding::all(0) });
    mouse_area(cell).on_press(Message::Select(cue.id)).into()
}
