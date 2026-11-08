// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fiber local storage, whose callback runs the TLS destructors of Rust's
//! standard library when a thread exits
//!
//! ReactOS runs a thread's FLS callbacks only after it has freed the
//! thread's TLS, while that callback drains a list kept in TLS, so that it
//! read and wrote freed memory and corrupted the heap.  Where ntdll has the
//! FLS of Windows, these functions call it.  Elsewhere each FLS key is a TLS
//! slot, whose callback runs from the program's own TLS callback, which the
//! loader calls before it frees TLS, as Windows orders the two.

use std::{
    ffi::c_void,
    mem, ptr,
    sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
};

use super::{
    imports::{define, find},
    system::{
        ERROR_NO_MORE_ITEMS, Proc, SetLastError, TlsAlloc, TlsFree, TlsGetValue, TlsSetValue,
    },
};

type Callback = unsafe extern "system" fn(*mut c_void);

const DLL_THREAD_DETACH: u32 = 3;
const ERROR_INVALID_PARAMETER: u32 = 87;
const OUT_OF_INDEXES: u32 = u32::MAX;

static KEYS: [Key; 64] = [const { Key::new() }; 64];
static USED: AtomicUsize = AtomicUsize::new(0);

#[unsafe(link_section = ".CRT$XLY")]
#[used]
static RUNDOWN: unsafe extern "system" fn(*mut c_void, u32, *mut c_void) = rundown;

define!(
    windows("FlsAlloc\0"),
    4,
    fn FlsAlloc(callback: Option<Callback>) -> u32 {
        alloc(callback)
    }
);
define!(
    windows("FlsFree\0"),
    4,
    fn FlsFree(key: u32) -> i32 {
        opened(key).map_or(0, |key| {
            key.run();
            key.ready.store(false, Ordering::Release);
            unsafe { TlsFree(key.slot.load(Ordering::Relaxed)) }
        })
    }
);
define!(
    windows("FlsSetValue\0"),
    8,
    fn FlsSetValue(key: u32, value: *mut c_void) -> i32 {
        opened(key).map_or(0, |key| unsafe {
            TlsSetValue(key.slot.load(Ordering::Relaxed), value)
        })
    }
);
define!(
    windows("FlsGetValue\0"),
    4,
    fn FlsGetValue(key: u32) -> *mut c_void {
        opened(key).map_or(ptr::null_mut(), |key| unsafe {
            TlsGetValue(key.slot.load(Ordering::Relaxed))
        })
    }
);

/// Key emulated by a TLS slot
struct Key {
    slot: AtomicU32,
    callback: AtomicUsize,
    ready: AtomicBool,
}

impl Key {
    const fn new() -> Self {
        Self {
            slot: AtomicU32::new(0),
            callback: AtomicUsize::new(0),
            ready: AtomicBool::new(false),
        }
    }

    fn open(&self, slot: u32, callback: Option<Callback>) {
        self.slot.store(slot, Ordering::Relaxed);
        self.callback.store(
            callback.map_or(0, |callback| callback as usize),
            Ordering::Relaxed,
        );
        self.ready.store(true, Ordering::Release);
    }

    /// Runs the callback on the current thread's value, if it has one
    fn run(&self) {
        let slot = self.slot.load(Ordering::Relaxed);
        let value = unsafe { TlsGetValue(slot) };
        let callback = self.callback.load(Ordering::Relaxed);
        if !value.is_null() && callback != 0 {
            unsafe {
                TlsSetValue(slot, ptr::null_mut());
                mem::transmute::<usize, Callback>(callback)(value);
            }
        }
    }
}

/// FLS function `name` of kernel32.dll, where ntdll runs FLS callbacks
/// before it frees TLS, as that of Windows does
fn windows(name: &str) -> Option<Proc> {
    let native = ["RtlProcessFlsData\0", "RtlFlsAlloc\0"]
        .iter()
        .any(|ntdll| find("ntdll.dll\0", ntdll).is_some());
    native.then(|| find("kernel32.dll\0", name)).flatten()
}

fn alloc(callback: Option<Callback>) -> u32 {
    // The loader finds the TLS callback only if the linker keeps it
    let _ = unsafe { ptr::read_volatile(&raw const RUNDOWN) };
    let index = USED.fetch_add(1, Ordering::Relaxed);
    let opened = KEYS.get(index).and_then(|key| {
        let slot = unsafe { TlsAlloc() };
        (slot != OUT_OF_INDEXES).then(|| key.open(slot, callback))
    });
    opened.map_or_else(
        || {
            unsafe { SetLastError(ERROR_NO_MORE_ITEMS) };
            OUT_OF_INDEXES
        },
        |()| index as u32,
    )
}

fn opened(key: u32) -> Option<&'static Key> {
    let key = KEYS
        .get(key as usize)
        .filter(|key| key.ready.load(Ordering::Acquire));
    if key.is_none() {
        unsafe { SetLastError(ERROR_INVALID_PARAMETER) };
    }
    key
}

unsafe extern "system" fn rundown(_module: *mut c_void, reason: u32, _reserved: *mut c_void) {
    if reason == DLL_THREAD_DETACH && windows("FlsAlloc\0").is_none() {
        KEYS.iter()
            .filter(|key| key.ready.load(Ordering::Acquire))
            .for_each(Key::run);
    }
}
