// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Which columns of the cue list show, and how wide, in the room there is

use strum::VariantArray;

use super::{CELL_SPACE, Column, NAME_ROOM, TARGET_ROOM, layout::Layout};
use crate::front::{
    App,
    view::{char_width, giving_way},
};

/// What gives way when the room is short, in turn
#[derive(Clone, Copy, PartialEq)]
enum Step {
    /// The Target column narrows to [`TARGET_ROOM`] columns of the font
    Narrow,
    Drop(Column),
}

/// Returns the columns that a list `room` wide shows, each with its width.
/// A column that holds no text in any row is not shown at all.  When showing
/// the rest leaves a name less than [`NAME_ROOM`] columns of the font, past
/// the gutter that a list holding a group keeps for the mark, the Target
/// column narrows to [`TARGET_ROOM`], which it takes whole up to a longer
/// width otherwise, and then the yielding columns give way in turn.  The
/// inspector shows them all.
pub(super) fn fitting(app: &App, layout: &Layout, room: f32) -> Vec<(Column, Option<f32>)> {
    let groups = &layout.groups;
    let all: Vec<(Column, Option<f32>)> = Column::VARIANTS
        .iter()
        .filter(|column| column.shown(&app.workspace, groups))
        .map(|&column| (column, column.width(&app.workspace, groups)))
        .collect();
    let steps: Vec<Step> = std::iter::once(Step::Narrow)
        .chain(
            Column::YIELDING
                .into_iter()
                .filter(|yielding| all.iter().any(|(column, _)| column == yielding))
                .map(Step::Drop),
        )
        .collect();
    let name_room = |columns: &[(Column, Option<f32>)]| {
        room - columns
            .iter()
            .map(|(_, width)| width.unwrap_or(0.0) + CELL_SPACE)
            .sum::<f32>()
    };
    giving_way(
        &steps,
        |gone| {
            let narrow = gone.contains(&Step::Narrow);
            all.iter()
                .filter(|(column, _)| !gone.contains(&Step::Drop(*column)))
                .map(|&(column, width)| match (column, width) {
                    (Column::Target, Some(width)) if narrow => {
                        (column, Some(width.min(TARGET_ROOM as f32 * char_width())))
                    }
                    shown => shown,
                })
                .collect()
        },
        |columns: &Vec<_>| {
            name_room(columns) >= (NAME_ROOM + layout.gutter()) as f32 * char_width()
        },
    )
}
