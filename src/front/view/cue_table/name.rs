// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Name cell of the cue list
//!
//! A name moves right by the depth of its cue and, in a list that holds a
//! group, leaves a gutter of [`INDENT`] columns before it, so that the names
//! of the cues in a group lie one level right of the name of the group.  A
//! group has its mark in the gutter, as a tree view marks its nodes: `-` where
//! the group is expanded and `+` where it is collapsed, and a collapsed group
//! tells after its name how many cues it hides, as Vim tells the lines of a
//! closed fold.  A click on the mark flips the group.  A list with no group
//! has no gutter, so that its names lie where they always did.

use std::iter;

use iced::{
    Element, Font,
    font::Weight,
    padding,
    widget::{container, mouse_area, rich_text, row, space, span, text},
};

use super::{INDENT, ROW_PADDING};
use crate::{
    back::{Cue, Kind},
    front::{
        Message,
        view::{FONT, char_width},
    },
};

/// Returns the mark of a group, which fills the gutter
fn mark(cue: &Cue) -> &'static str {
    if cue.collapsed { "+ " } else { "- " }
}

/// Returns what a collapsed group tells after its name, which is how many
/// cues it hides, with a no-break space so that a narrow column wraps the note
/// whole
fn hidden_note(cue: &Cue, hidden: usize) -> Option<String> {
    cue.collapsed.then(|| format!("  ({hidden}\u{a0}hidden)"))
}

/// Returns the message of a click on the gutter of `cue`, which flips a group
/// and leaves the other cues to the click on their row
fn flip(cue: &Cue) -> Option<Message> {
    (cue.kind == Kind::Group).then_some(Message::FlipGroup(cue.id))
}

/// Returns the cell that shows `name` for `cue`, which lies `depth` levels
/// deep and, if a group, holds `hidden` cues that the table leaves out, with
/// `gutter` columns of the font free before the name for the mark of a group
pub(super) fn cell<'a>(
    cue: &Cue,
    name: String,
    depth: u8,
    hidden: usize,
    gutter: usize,
) -> Element<'a, Message> {
    let vertical = padding::vertical(ROW_PADDING);
    let indent = space().width(f32::from(depth) * INDENT as f32 * char_width());
    let before: Element<'a, Message> = match flip(cue) {
        Some(flip) => mouse_area(container(text(mark(cue))).padding(vertical))
            .on_press(flip)
            .into(),
        None => space().width(gutter as f32 * char_width()).into(),
    };
    let name: Element<'a, Message> = match cue.kind {
        Kind::Group => {
            let bold = Font {
                weight: Weight::Bold,
                ..FONT
            };
            let spans: Vec<text::Span<'a, ()>> = iter::once(span(name).font(bold))
                .chain(hidden_note(cue, hidden).map(span))
                .collect();
            rich_text(spans).into()
        }
        _ => text(name).into(),
    };
    row![indent, before, container(name).padding(vertical)].into()
}

#[cfg(test)]
mod tests;
