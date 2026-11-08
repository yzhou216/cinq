// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What lets the 32-bit program for Windows run on ReactOS as well
//!
//! ReactOS's loader refuses a program whose imports it cannot resolve, and
//! Rust's standard library and cinq's crates import functions of later
//! Windows that ReactOS lacks, or keeps in private `*_vista.dll` companions
//! of its DLLs.  [`imports`] defines each of them in the program itself.
//! [`fls`] works around ReactOS running the callbacks of fiber local
//! storage after it has freed a thread's TLS, [`math`] replaces functions
//! of the C runtime that ReactOS exports only as stubs, and [`dialog`]
//! shows the older file dialogs in place of those that ReactOS lacks.  On
//! Windows the imports, fiber local storage, and file dialogs are the
//! system's own.

#[cfg(reactos)]
mod dialog;
#[cfg(reactos)]
mod fls;
#[cfg(reactos)]
mod imports;
mod math;
#[cfg(reactos)]
mod system;

#[cfg(reactos)]
pub use imports::has_wasapi;
