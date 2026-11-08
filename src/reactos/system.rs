// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Functions of Windows that the workarounds call, all of which ReactOS has

use std::ffi::c_void;

/// Function that `GetProcAddress` found, to be transmuted to its type
pub type Proc = unsafe extern "system" fn() -> isize;

#[derive(PartialEq)]
#[repr(C)]
pub struct Guid(pub u32, pub u16, pub u16, pub [u8; 8]);

pub const ERROR_CALL_NOT_IMPLEMENTED: u32 = 120;
pub const ERROR_NO_MORE_ITEMS: u32 = 259;
pub const ERROR_TIMEOUT: u32 = 1460;
pub const E_INVALIDARG: i32 = 0x8007_0057_u32 as i32;
pub const E_NOTIMPL: i32 = 0x8000_4001_u32 as i32;
pub const E_OUTOFMEMORY: i32 = 0x8007_000E_u32 as i32;

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    pub fn GetProcAddress(module: *mut c_void, name: *const u8) -> Option<Proc>;
    pub fn LoadLibraryA(name: *const u8) -> *mut c_void;
    pub fn SetErrorMode(mode: u32) -> u32;
    pub fn SetLastError(code: u32);
    pub fn Sleep(milliseconds: u32);
    pub fn TlsAlloc() -> u32;
    pub fn TlsFree(slot: u32) -> i32;
    pub fn TlsGetValue(slot: u32) -> *mut c_void;
    pub fn TlsSetValue(slot: u32, value: *mut c_void) -> i32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    /// `RtlGenRandom`
    pub fn SystemFunction036(data: *mut u8, size: u32) -> u8;
}

#[link(name = "ole32")]
unsafe extern "system" {
    pub fn CoTaskMemAlloc(size: usize) -> *mut c_void;
}

#[link(name = "shell32")]
unsafe extern "system" {
    pub fn SHGetFolderPathW(
        owner: *mut c_void,
        folder: i32,
        token: *mut c_void,
        flags: u32,
        path: *mut u16,
    ) -> i32;
}
