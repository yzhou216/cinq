// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Imports of Rust's standard library and of LLVM's libunwind, which it
//! links

use std::{ffi::c_void, mem, process::abort, ptr};

use super::{Resolved, find, import, not_implemented};
use crate::reactos::system::{
    ERROR_TIMEOUT, Proc, SetErrorMode, SetLastError, Sleep, SystemFunction036,
};

const STATUS_TIMEOUT: i32 = 0x102;

static WAIT: Resolved = Resolved::new();
static WAKE_ONE: Resolved = Resolved::new();
static WAKE_ALL: Resolved = Resolved::new();

import!(
    "api-ms-win-core-synch-l1-2-0.dll",
    16,
    fn WaitOnAddress(
        address: *const c_void,
        compare: *const c_void,
        size: usize,
        milliseconds: u32,
    ) -> i32 {
        wait_on_address(address, compare, size, milliseconds)
    }
);
import!(
    "api-ms-win-core-synch-l1-2-0.dll",
    4,
    fn WakeByAddressSingle(address: *const c_void) {
        wake(&WAKE_ONE, "RtlWakeAddressSingle\0", address)
    }
);
import!(
    "api-ms-win-core-synch-l1-2-0.dll",
    4,
    fn WakeByAddressAll(address: *const c_void) {
        wake(&WAKE_ALL, "RtlWakeAddressAll\0", address)
    }
);
import!(
    "bcryptprimitives.dll",
    8,
    fn ProcessPrng(data: *mut u8, size: usize) -> i32 {
        i32::from(unsafe { SystemFunction036(data, size as u32) } != 0)
    }
);
import!(
    "kernel32.dll",
    4,
    fn AcquireSRWLockExclusive(lock: *mut c_void) {
        abort()
    }
);
import!(
    "kernel32.dll",
    4,
    fn AcquireSRWLockShared(lock: *mut c_void) {
        abort()
    }
);
import!(
    "kernel32.dll",
    4,
    fn ReleaseSRWLockExclusive(lock: *mut c_void) {
        abort()
    }
);
import!(
    "kernel32.dll",
    4,
    fn ReleaseSRWLockShared(lock: *mut c_void) {
        abort()
    }
);
import!(
    "kernel32.dll",
    16,
    fn K32EnumProcessModules(
        process: *mut c_void,
        modules: *mut *mut c_void,
        size: u32,
        needed: *mut u32,
    ) -> i32 {
        type EnumProcessModules =
            unsafe extern "system" fn(*mut c_void, *mut *mut c_void, u32, *mut u32) -> i32;
        match find("psapi.dll\0", "EnumProcessModules\0") {
            Some(psapi) => unsafe {
                mem::transmute::<Proc, EnumProcessModules>(psapi)(process, modules, size, needed)
            },
            None => not_implemented(0),
        }
    }
);
import!(
    "kernel32.dll",
    20,
    fn CompareStringOrdinal(
        a: *const u16,
        a_size: i32,
        b: *const u16,
        b_size: i32,
        ignore_case: i32,
    ) -> i32 {
        not_implemented(0)
    }
);
import!(
    "kernel32.dll",
    16,
    fn CreateWaitableTimerExW(
        attributes: *mut c_void,
        name: *const u16,
        flags: u32,
        access: u32,
    ) -> *mut c_void {
        not_implemented(ptr::null_mut())
    }
);
import!(
    "kernel32.dll",
    16,
    fn GetFileInformationByHandleEx(
        file: *mut c_void,
        class: i32,
        information: *mut c_void,
        size: u32,
    ) -> i32 {
        not_implemented(0)
    }
);
import!(
    "kernel32.dll",
    16,
    fn SetFileInformationByHandle(
        file: *mut c_void,
        class: i32,
        information: *mut c_void,
        size: u32,
    ) -> i32 {
        not_implemented(0)
    }
);
import!(
    "kernel32.dll",
    16,
    fn GetFinalPathNameByHandleW(file: *mut c_void, path: *mut u16, size: u32, flags: u32) -> u32 {
        not_implemented(0)
    }
);
import!(
    "kernel32.dll",
    8,
    fn SetThreadErrorMode(mode: u32, old: *mut u32) -> i32 {
        let previous = unsafe { SetErrorMode(mode) };
        if !old.is_null() {
            unsafe { *old = previous };
        }
        1
    }
);
import!(
    "kernel32.dll",
    0,
    fn IsThreadAFiber() -> i32 {
        0
    }
);

/// Waits with `RtlWaitOnAddress` where ReactOS has it, or else polls, which
/// callers take as a spurious wake
fn wait_on_address(
    address: *const c_void,
    compare: *const c_void,
    size: usize,
    milliseconds: u32,
) -> i32 {
    type RtlWaitOnAddress =
        unsafe extern "system" fn(*const c_void, *const c_void, usize, *const i64) -> i32;
    let timed_out = match WAIT.get("ntdll.dll\0", "RtlWaitOnAddress\0") {
        Some(wait) => {
            let timeout = -i64::from(milliseconds) * 10_000;
            let timeout = if milliseconds == u32::MAX {
                ptr::null()
            } else {
                &raw const timeout
            };
            let wait = unsafe { mem::transmute::<Proc, RtlWaitOnAddress>(wait) };
            unsafe { wait(address, compare, size, timeout) == STATUS_TIMEOUT }
        }
        None => {
            let unchanged = (0..size).all(|offset| unsafe {
                ptr::read_volatile(address.cast::<u8>().add(offset))
                    == *compare.cast::<u8>().add(offset)
            });
            if unchanged && milliseconds != 0 {
                unsafe { Sleep(1) };
            }
            unchanged && milliseconds == 0
        }
    };
    if timed_out {
        unsafe { SetLastError(ERROR_TIMEOUT) };
    }
    i32::from(!timed_out)
}

fn wake(rtl: &Resolved, name: &str, address: *const c_void) {
    if let Some(wake) = rtl.get("ntdll.dll\0", name) {
        unsafe { mem::transmute::<Proc, unsafe extern "system" fn(*const c_void)>(wake)(address) }
    }
}
