// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Imports that ReactOS lacks, defined in the program
//!
//! Each becomes a function of the program, together with the address that
//! the import library would have the loader fill in, `__imp__Name@bytes`.
//! The program's own objects come before the import libraries, so that the
//! linker takes these names from them and imports nothing for them.  The
//! function calls the system's own, from its DLL or else from ReactOS's
//! `*_vista.dll` companion of it, and only without either falls back to an
//! older way, or fails as callers expect the function to fail on Windows.

mod crates;
mod runtime;

use std::{
    ffi::c_void,
    mem,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::system::{
    ERROR_CALL_NOT_IMPLEMENTED, GetModuleHandleA, GetProcAddress, LoadLibraryA, Proc, SetLastError,
};

/// Defines `$name`, whose arguments take `$bytes` bytes, as a function that
/// calls the function that `$real` finds, or else runs the body that
/// follows, and the import address that points to it
macro_rules! define {
    ($real:expr, $bytes:literal, fn $name:ident($($argument:ident: $type:ty),* $(,)?) -> $output:ty $fallback:block) => {
        #[expect(non_snake_case)]
        #[unsafe(export_name = stringify!($name))]
        unsafe extern "system" fn $name($($argument: $type),*) -> $output {
            match $real {
                Some(real) => unsafe {
                    std::mem::transmute::<
                        $crate::reactos::system::Proc,
                        unsafe extern "system" fn($($type),*) -> $output,
                    >(real)($($argument),*)
                },
                None => $fallback,
            }
        }
        const _: () = {
            #[used]
            #[unsafe(export_name = concat!("_imp__", stringify!($name), "@", $bytes))]
            static IMPORT: unsafe extern "system" fn($($type),*) -> $output = $name;
        };
    };
}
pub(super) use define;

/// Defines the import `$name` of `$dll`, which falls back to the body that
/// follows
macro_rules! import {
    ($dll:literal, $bytes:literal, fn $name:ident($($argument:ident: $type:ty),* $(,)?) -> $output:ty $fallback:block) => {
        $crate::reactos::imports::define!(
            {
                static REAL: $crate::reactos::imports::Resolved =
                    $crate::reactos::imports::Resolved::new();
                REAL.get(concat!($dll, "\0"), concat!(stringify!($name), "\0"))
            },
            $bytes,
            fn $name($($argument: $type),*) -> $output $fallback
        );
    };
    ($dll:literal, $bytes:literal, fn $name:ident($($argument:ident: $type:ty),* $(,)?) $fallback:block) => {
        $crate::reactos::imports::import!($dll, $bytes, fn $name($($argument: $type),*) -> () $fallback);
    };
}
pub(super) use import;

/// Returns whether the system has WASAPI, without which cpal panics
pub fn has_wasapi() -> bool {
    module("mmdevapi.dll\0").is_some()
}

/// Function `name` of `dll`, both ending in NUL
pub fn find(dll: &str, name: &str) -> Option<Proc> {
    module(dll).and_then(|module| unsafe { GetProcAddress(module, name.as_ptr()) })
}

fn module(dll: &str) -> Option<*mut c_void> {
    [GetModuleHandleA, LoadLibraryA]
        .into_iter()
        .map(|open| unsafe { open(dll.as_ptr()) })
        .find(|module| !module.is_null())
}

/// Function `name` of `dll` or of its companion, in which ReactOS keeps the
/// functions of later Windows that kernel32.dll would hold for API sets
fn resolve(dll: &str, name: &str) -> Option<Proc> {
    let companion = match dll.strip_suffix(".dll\0") {
        Some(base) if !base.starts_with("api-ms-") => format!("{base}_vista.dll\0"),
        _ => "kernel32_vista.dll\0".to_owned(),
    };
    find(dll, name).or_else(|| find(&companion, name))
}

/// Returns `value` as a function of Windows does that it does not implement
fn not_implemented<T>(value: T) -> T {
    unsafe { SetLastError(ERROR_CALL_NOT_IMPLEMENTED) };
    value
}

/// Function that [`resolve`] found, kept without a lock, since the locks of
/// the standard library wait with `WaitOnAddress`
pub struct Resolved(AtomicUsize);

impl Resolved {
    const UNKNOWN: usize = 0;
    const MISSING: usize = 1;

    pub const fn new() -> Self {
        Self(AtomicUsize::new(Self::UNKNOWN))
    }

    pub fn get(&self, dll: &str, name: &str) -> Option<Proc> {
        let address = match self.0.load(Ordering::Relaxed) {
            Self::UNKNOWN => {
                let found = resolve(dll, name).map_or(Self::MISSING, |proc| proc as usize);
                self.0.store(found, Ordering::Relaxed);
                found
            }
            known => known,
        };
        (address != Self::MISSING).then(|| unsafe { mem::transmute::<usize, Proc>(address) })
    }
}
