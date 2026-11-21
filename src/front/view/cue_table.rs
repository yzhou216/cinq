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
//! row of the collapsed group that hides it where one does.  Widths are counted in columns of
//! the monospace font, of which a wide character such as one of Chinese
//! takes two, so that the header, a table of its own above the rows that
//! scroll, lines up with them.

mod columns;
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
use itertools::Itertools;
use strum::VariantArray;

use super::{CUES, SELECTED, char_width, giving_way, marked_row};
use crate::{
    back::{Cue, Groups, Workspace},
    front::{App, Message, keymap::Action},
};
use columns::{Column, hidden_problem, state};
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
/// column gives way
const NAME_ROOM: usize = 16;

/// Most columns of the font that the Target column takes, past which a
/// long file name wraps
const TARGET_ROOM: usize = 24;

/// Columns of the font that a name moves right for each group that holds its
/// cue
const INDENT: usize = 2;

/// Columns of the font that the State column holds, so that the column
/// keeps its width as cues run: as many as `10:00.0 left` takes, the
/// longest state of a cue whose sound is under a hundred minutes
const STATE_ROOM: usize = 12;

pub(super) fn view(app: &App) -> Element<'_, Message> {
    responsive(move |size| list(app, size.width - SCROLL_BAR)).into()
}

fn list<'a>(app: &'a App, room: f32) -> Element<'a, Message> {
    let layout = Rc::new(Layout::of(app.current()));
    let fitting = fitting(app, &layout, room);
    let table = |titled: bool, cues: Vec<&'a Cue>| {
        let layout = Rc::clone(&layout);
        let columns = fitting.iter().map(move |&(column, width)| {
            table_column(app, column, width, titled, Rc::clone(&layout))
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
        _ if !app.editable() => "No cues yet.  Leave show mode to add one.".to_owned(),
        Some(keys) => format!("No cues yet.  Press {keys} or Add cue to add one."),
        None => "No cues yet.  Press Add cue to add one.".to_owned(),
    };
    container(text(hint))
        .padding([ROW_PADDING, CELL_PADDING])
        .into()
}

/// Returns the columns that a list `room` wide shows, each with its width.
/// When showing them all leaves a name less than [`NAME_ROOM`] columns of
/// the font, past the gutter that a list holding a group keeps for the mark,
/// the yielding columns give way in turn, those without text in any row
/// first, as they tell nothing, and the inspector shows them all.
fn fitting(app: &App, layout: &Layout, room: f32) -> Vec<(Column, Option<f32>)> {
    let groups = &layout.groups;
    let all: Vec<(Column, Option<f32>)> = Column::VARIANTS
        .iter()
        .map(|&column| (column, column.width(&app.workspace, groups)))
        .collect();
    let yielding: Vec<Column> = Column::YIELDING
        .into_iter()
        .sorted_by_cached_key(|column| column.holds_text(&app.workspace, groups))
        .collect();
    let name_room = |columns: &[(Column, Option<f32>)]| {
        room - columns
            .iter()
            .map(|(_, width)| width.unwrap_or(0.0) + CELL_SPACE)
            .sum::<f32>()
    };
    giving_way(
        &yielding,
        |gone| {
            all.iter()
                .copied()
                .filter(|(column, _)| !gone.contains(column))
                .collect()
        },
        |columns: &Vec<_>| {
            name_room(columns) >= (NAME_ROOM + layout.gutter()) as f32 * char_width()
        },
    )
}

fn table_column<'a>(
    app: &'a App,
    column: Column,
    width: Option<f32>,
    titled: bool,
    layout: Rc<Layout<'a>>,
) -> table::Column<'a, 'a, &'a Cue, Message> {
    let header: Element<'a, Message> = if titled {
        text(column.title()).into()
    } else {
        space().into()
    };
    table::column(header, move |cue| cell(app, column, cue, &layout))
        .width(width.map_or(Fill, Length::Fixed))
}

/// Returns the cell of `cue` in `column`, padded inside rather than by the
/// table, so that a click anywhere in it selects the cue, but on the mark of
/// a group, which flips it.  The cell of a name pads its parts itself, so that
/// the mark takes the height of the row.
fn cell<'a>(app: &'a App, column: Column, cue: &'a Cue, layout: &Layout) -> Element<'a, Message> {
    let groups = &layout.groups;
    let vertical = padding::vertical(ROW_PADDING);
    let (content, padded): (Element<'a, Message>, _) = match column {
        Column::Name => {
            let name = column.text(&app.workspace, groups, cue);
            let hidden = groups.members(cue.id).len();
            (
                name::cell(cue, name, layout.depth(cue), hidden, layout.gutter()),
                false,
            )
        }
        Column::Number => (
            radio(cue.number.clone(), cue.id, app.selected, Message::Select).into(),
            true,
        ),
        Column::Target => (
            text(column.text(&app.workspace, groups, cue))
                .wrapping(Wrapping::WordOrGlyph)
                .into(),
            true,
        ),
        Column::State => {
            let selected = app.selected == Some(cue.id);
            let hidden = hidden_problem(&app.workspace, groups, cue);
            let (state, style) = state(&app.workspace, cue, layout.is_next(cue), hidden, selected);
            (text(state).style(style).into(), true)
        }
        _ => (text(column.text(&app.workspace, groups, cue)).into(), true),
    };
    let cell =
        container(content)
            .width(Fill)
            .padding(if padded { vertical } else { padding::all(0) });
    mouse_area(cell).on_press(Message::Select(cue.id)).into()
}
