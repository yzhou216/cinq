// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Names cut short to fit their room, in a list, the status bar, or a notice

#[cfg(test)]
mod tests;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Returns `name` cut short to `columns` columns of the font, which is
/// monospace, with an ellipsis in place of the rest
pub(in crate::front::view) fn shortened(name: &str, columns: usize) -> String {
    if name.width() <= columns {
        return name.to_owned();
    }
    let kept: String = name
        .chars()
        .scan(0, |taken, character| {
            *taken += character.width().unwrap_or(0);
            (*taken < columns).then_some(character)
        })
        .collect();
    format!("{kept}\u{2026}")
}
