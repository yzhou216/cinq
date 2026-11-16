// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rows of the outputs, with the titles of their columns above

use iced::{
    Element, Fill, padding,
    widget::{container, mouse_area, radio, scrollable, table, text, text::Wrapping},
};

use super::{Column, fitting};
use crate::{
    back::{Fault, Output},
    front::{
        App, Message,
        view::{OUTPUTS, SELECTED_OUTPUT, cue_table::ROW_PADDING, faults, marked_row},
    },
};

pub(super) fn list(app: &App, room: f32) -> Element<'_, Message> {
    let shown = fitting(app, room);
    let columns = shown.iter().copied().map(|(column, width)| {
        let title = container(text(<&str>::from(column))).padding(padding::vertical(ROW_PADDING));
        table::column(title, move |(row, output): (usize, &Output)| {
            cell(app, column, row, output)
        })
        .width(width)
    });
    let outputs = table::table(columns, app.workspace.outputs().iter().enumerate()).padding_y(0);
    scrollable(marked_row(
        outputs,
        shown.len(),
        app.output_row(),
        SELECTED_OUTPUT,
    ))
    // Bar beside the table, rather than over the state of each output
    .spacing(0)
    .id(OUTPUTS)
    .height(Fill)
    .into()
}

/// Returns the cell of `output` in `column`, padded inside rather than by
/// the table, so that a click anywhere in it selects the output
fn cell<'a>(app: &'a App, column: Column, row: usize, output: &'a Output) -> Element<'a, Message> {
    let shown = column.text(app, row, output);
    let content: Element<'a, Message> = match column {
        Column::Number => radio(shown, output.id, app.output, Message::SelectOutput).into(),
        Column::State => {
            let retried = faults::fault(app, output.id).is_some_and(Fault::retried);
            text(shown)
                .style(if retried { text::warning } else { text::danger })
                .into()
        }
        _ => text(shown).wrapping(Wrapping::WordOrGlyph).into(),
    };
    let cell = container(content)
        .width(Fill)
        .padding(padding::vertical(ROW_PADDING));
    mouse_area(cell)
        .on_press(Message::SelectOutput(output.id))
        .into()
}
