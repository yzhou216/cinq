// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the replacements against the C runtime of the system that runs
//! the tests

use super::{acosh, asinh, cbrt, log1p, trunc, truncf};

// The C runtime's own, since the standard library computes acosh in Rust,
// which overflows on the largest values
unsafe extern "C" {
    #[link_name = "log1p"]
    safe fn c_log1p(x: f64) -> f64;
    #[link_name = "acosh"]
    safe fn c_acosh(x: f64) -> f64;
    #[link_name = "asinh"]
    safe fn c_asinh(x: f64) -> f64;
    #[link_name = "cbrt"]
    safe fn c_cbrt(x: f64) -> f64;
}

/// Values across the range, with both zeros, the infinities, NaN, the
/// smallest subnormal, and the largest finite
const VALUES: [f64; 20] = [
    0.0,
    -0.0,
    5e-324,
    1e-300,
    1e-10,
    0.25,
    0.5,
    1.0,
    1.5,
    2.7,
    -2.7,
    27.0,
    1e6,
    4_503_599_627_370_495.5,
    1e300,
    f64::MAX,
    -f64::MAX,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
];

/// Whether `a` is within four units in the last place of `b`
fn close(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
        || (a.is_nan() && b.is_nan())
        || (a - b).abs() <= 4.0 * f64::EPSILON * b.abs().max(f64::MIN_POSITIVE)
}

#[test]
fn truncation_matches_bit_for_bit() {
    VALUES.iter().filter(|x| !x.is_nan()).for_each(|&x| {
        assert_eq!(trunc(x).to_bits(), x.trunc().to_bits(), "trunc({x})");
        let x = x as f32;
        assert_eq!(truncf(x).to_bits(), x.trunc().to_bits(), "truncf({x})");
    });
    assert!(trunc(f64::NAN).is_nan() && truncf(f32::NAN).is_nan());
}

#[test]
fn functions_match_closely() {
    VALUES
        .iter()
        .chain(&VALUES.map(|x| x - 1.0))
        .for_each(|&x| {
            assert!(close(log1p(x), c_log1p(x)), "log1p({x}) = {}", log1p(x));
            assert!(close(acosh(x), c_acosh(x)), "acosh({x}) = {}", acosh(x));
            assert!(close(asinh(x), c_asinh(x)), "asinh({x}) = {}", asinh(x));
            assert!(close(cbrt(x), c_cbrt(x)), "cbrt({x}) = {}", cbrt(x));
        });
}
