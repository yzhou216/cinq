// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the colors that pictures take with BT.601 and BT.709, by their
//! size

use rust_h264::decoder::{ChromaFormat, Frame};

use super::{HIGH_DEFINITION, rgba};

fn picture(width: u32, lines: u32, luma: u8, blue: u8, red: u8) -> Frame {
    let pixels = (width * lines) as usize;
    let shared = (width.div_ceil(2) * lines.div_ceil(2)) as usize;
    Frame {
        width,
        height: lines,
        y: vec![luma; pixels],
        u: vec![blue; shared],
        v: vec![red; shared],
        pic_order_cnt: 0,
        chroma_format: ChromaFormat::Yuv420,
        bit_depth: 8,
    }
}

fn all_close(pixels: &[u8], color: [u8; 4]) -> bool {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .all(|pixel| pixel.iter().zip(color).all(|(&a, b)| a.abs_diff(b) <= 1))
}

#[test]
fn black_gray_and_white_keep_their_shade() {
    let (_, lines) = HIGH_DEFINITION;
    let shades = [
        (16, [0, 0, 0, 255]),
        (126, [128, 128, 128, 255]),
        (235, [255, 255, 255, 255]),
    ];
    [2, lines].into_iter().for_each(|lines| {
        shades.iter().for_each(|&(luma, color)| {
            let pixels = rgba(&picture(2, lines, luma, 128, 128)).pixels;
            assert!(all_close(&pixels, color), "{luma} in {lines} lines");
        });
    });
}

#[test]
fn pictures_take_the_matrix_of_their_size() {
    let (width, lines) = HIGH_DEFINITION;
    // BT.601 holds below high definition in width and lines, and BT.709 from
    // either on, as in a wide picture cropped from high definition
    [
        (2, lines - 2, 81, 90),
        (2, lines, 63, 102),
        (width - 2, 534, 81, 90),
        (width, 534, 63, 102),
    ]
    .iter()
    .for_each(|&(width, lines, luma, blue)| {
        let pixels = rgba(&picture(width, lines, luma, blue, 240)).pixels;
        assert!(all_close(&pixels, [255, 0, 0, 255]), "{width} by {lines}");
    });
}

#[test]
fn picture_cropped_to_no_width_converts_to_no_pixels() {
    let (_, lines) = HIGH_DEFINITION;
    assert!(rgba(&picture(0, lines, 63, 102, 240)).pixels.is_empty());
    assert!(rgba(&picture(0, 2, 63, 102, 240)).pixels.is_empty());
}

#[test]
fn each_pixel_of_a_square_takes_the_shade_of_its_own_luma() {
    let square = Frame {
        y: vec![16, 235, 235, 16],
        ..picture(2, 2, 0, 128, 128)
    };
    let pixels = rgba(&square).pixels;
    let (black, white) = ([0, 0, 0, 255], [255, 255, 255, 255]);
    [black, white, white, black]
        .iter()
        .zip(pixels.as_chunks::<4>().0)
        .for_each(|(&color, pixel)| assert!(all_close(pixel, color), "{pixel:?}"));
}

#[test]
fn picture_of_an_odd_size_converts_all_but_its_last_row_and_column() {
    let pixels = rgba(&picture(3, 3, 126, 128, 128)).pixels;
    let rows: Vec<_> = pixels.as_chunks::<4>().0.chunks(3).collect();
    rows[..2].iter().for_each(|row| {
        assert!(all_close(row[..2].as_flattened(), [128, 128, 128, 255]));
        assert_eq!(row[2], [0, 0, 0, 0]);
    });
    assert!(rows[2].iter().all(|&pixel| pixel == [0, 0, 0, 0]));
}
