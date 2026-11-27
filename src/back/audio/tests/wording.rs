// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the words that tell why a sound file cannot be played

use std::fs;

use tempfile::TempDir;

use super::silence;
use crate::back::audio::probe;

/// Returns why the file that holds `bytes` cannot be played
fn failing(bytes: &[u8]) -> String {
    let directory = TempDir::new().expect("a temporary directory can be made");
    let path = directory.path().join("sound.wav");
    fs::write(&path, bytes).expect("the file can be written");
    probe(&path).expect_err("the file cannot be played")
}

#[test]
fn file_that_no_decoder_reads_is_not_a_sound_file_that_cinq_can_play() {
    let words = "not a sound file that cinq can play";
    assert_eq!(failing(b"not a sound"), words);
    assert_eq!(failing(&[]), words);
    // Header that stops where its format chunk begins
    assert_eq!(failing(&silence(1.0)[..14]), words);
}

#[test]
fn file_that_is_not_there_is_not_found() {
    let directory = TempDir::new().expect("a temporary directory can be made");
    let reason = probe(&directory.path().join("none.wav")).expect_err("the file is not there");
    assert_eq!(reason, "file not found");
}
