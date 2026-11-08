// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Functions of the C runtime that ReactOS's ucrtbase.dll exports only as
//! stubs, which return nothing useful
//!
//! cosmic-text places each glyph with `truncf`, so that the stub drew all of
//! iced's text at the top of a window.  These replace the stubs on Windows
//! too.  They take `log`, `exp`, and the processor's square root, which
//! ReactOS has, and none calls the standard library's method of its own
//! name, which would call itself.

#[cfg(test)]
mod tests;

use std::f64::consts::LN_2;

/// 2 to the 28th, past which `x * x` swamps 1
const LARGE: f64 = 268_435_456.0;

#[cfg(reactos)]
macro_rules! export {
    ($($name:ident($type:ty) = $value:expr;)*) => {$(
        const _: () = {
            #[unsafe(export_name = stringify!($name))]
            extern "C" fn function(x: $type) -> $type {
                $value(x)
            }
            #[used]
            #[unsafe(export_name = concat!("_imp__", stringify!($name)))]
            static IMPORT: extern "C" fn($type) -> $type = function;
        };
    )*};
}

#[cfg(reactos)]
export! {
    trunc(f64) = trunc;
    truncf(f32) = truncf;
    log1p(f64) = log1p;
    log1pf(f32) = |x: f32| log1p(x.into()) as f32;
    acosh(f64) = acosh;
    acoshf(f32) = |x: f32| acosh(x.into()) as f32;
    asinh(f64) = asinh;
    asinhf(f32) = |x: f32| asinh(x.into()) as f32;
    cbrtf(f32) = |x: f32| cbrt(x.into()) as f32;
}

/// `x` with the bits of its fraction below the binary point cleared
pub fn trunc(x: f64) -> f64 {
    let exponent = ((x.to_bits() >> 52) & 0x7ff) as i32 - 0x3ff;
    match exponent {
        52.. => x,
        ..0 => f64::from_bits(x.to_bits() & (1 << 63)),
        _ => f64::from_bits(x.to_bits() & !(u64::MAX >> (12 + exponent))),
    }
}

pub fn truncf(x: f32) -> f32 {
    let exponent = ((x.to_bits() >> 23) & 0xff) as i32 - 0x7f;
    match exponent {
        23.. => x,
        ..0 => f32::from_bits(x.to_bits() & (1 << 31)),
        _ => f32::from_bits(x.to_bits() & !(u32::MAX >> (9 + exponent))),
    }
}

/// Kahan's `log1p`, which makes up for the rounding of `1 + x`
pub fn log1p(x: f64) -> f64 {
    let u = 1.0 + x;
    if u == 1.0 {
        x
    } else if u.is_finite() && u > 0.0 {
        u.ln() * (x / (u - 1.0))
    } else {
        u.ln()
    }
}

pub fn acosh(x: f64) -> f64 {
    if x > LARGE {
        x.ln() + LN_2
    } else {
        let t = x - 1.0;
        log1p(t + (2.0 * t + t * t).sqrt())
    }
}

pub fn asinh(x: f64) -> f64 {
    let a = x.abs();
    let y = if a > LARGE {
        a.ln() + LN_2
    } else {
        log1p(a + a * a / (1.0 + (1.0 + a * a).sqrt()))
    };
    y.copysign(x)
}

/// Newton's method from `exp(ln(x) / 3)`, in the form that cannot overflow
pub fn cbrt(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let a = x.abs();
    let y = (0..2).fold((a.ln() / 3.0).exp(), |y, _| (2.0 * y + a / (y * y)) / 3.0);
    y.copysign(x)
}
