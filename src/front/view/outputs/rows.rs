// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rows of the outputs, with the titles of their columns above

use iced::{
    Element, Fill, padding,
    widget::{container, mouse_area, radio, scrollable, table, text, text::Wrapping},
};
use strum::VariantArray;

use super::Column;
use crate::{
    back::Output,
    front::{
        App, Message,
        view::{OUTPUTS, SELECTED_OUTPUT, cue_table::ROW_PADDING, marked_row},
    },
};

pub(super) fn list(app: &App) -> Element<'_, Message> {
    let columns = Column::VARIANTS.iter().map(|&column| {
        let title = container(text(<&str>::from(column))).padding(padding::vertical(ROW_PADDING));
        table::column(title, move |(row, output): (usize, &Output)| {
            cell(app, column, row, output)
        })
        .width(column.width(app))
    });
    let outputs = table::table(columns, app.workspace.outputs().iter().enumerate()).padding_y(0);
    scrollable(marked_row(
        outputs,
        Column::VARIANTS.len(),
        app.output_row(),
        SELECTED_OUTPUT,
    ))
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
        Column::State => text(shown).style(text::danger).into(),
        _ => text(shown).wrapping(Wrapping::WordOrGlyph).into(),
    };
    let cell = container(content)
        .width(Fill)
        .padding(padding::vertical(ROW_PADDING));
    mouse_area(cell)
        .on_press(Message::SelectOutput(output.id))
        .into()
}
