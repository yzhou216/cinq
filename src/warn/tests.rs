// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of when a warning comes out

use super::{LIMIT, Warnings};

#[test]
fn warning_comes_out_once_after_cinq() {
    let (warnings, first) = Warnings::default().warned("a.mp4: skipped its tags");
    assert_eq!(first, "cinq: a.mp4: skipped its tags\n");
    let (warnings, again) = warnings.warned("a.mp4: skipped its tags");
    assert_eq!(again, "");
    let (_, other) = warnings.warned("b.mp4: skipped its tags");
    assert_eq!(other, "cinq: b.mp4: skipped its tags\n");
}

#[test]
fn warnings_end_with_one_line_after_the_limit() {
    let warnings = (0..LIMIT).fold(Warnings::default(), |warnings, number| {
        let (warnings, line) = warnings.warned(&format!("warning {number}"));
        assert_eq!(line, format!("cinq: warning {number}\n"));
        warnings
    });
    let (warnings, repeated) = warnings.warned("warning 0");
    assert_eq!(repeated, "");
    let (warnings, last) = warnings.warned("one more");
    assert_eq!(last, "cinq: further warnings go unreported\n");
    let (warnings, silent) = warnings.warned("yet another");
    assert_eq!(silent, "");
    assert_eq!(warnings.given.len(), LIMIT);
}
