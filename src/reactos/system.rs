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
pub const E_CANCELLED: i32 = 0x8007_04C7_u32 as i32;
pub const E_INVALIDARG: i32 = 0x8007_0057_u32 as i32;
pub const E_NOINTERFACE: i32 = 0x8000_4002_u32 as i32;
pub const E_NOTIMPL: i32 = 0x8000_4001_u32 as i32;
pub const E_OUTOFMEMORY: i32 = 0x8007_000E_u32 as i32;
pub const E_UNEXPECTED: i32 = 0x8000_FFFF_u32 as i32;
pub const REGDB_E_CLASSNOTREG: i32 = 0x8004_0154_u32 as i32;

/// `OPENFILENAMEW` of the older file dialogs
#[repr(C)]
pub struct OpenFileName {
    pub size: u32,
    pub owner: *mut c_void,
    pub instance: *mut c_void,
    pub filter: *const u16,
    pub custom_filter: *mut u16,
    pub custom_filter_size: u32,
    pub filter_index: u32,
    pub file: *mut u16,
    pub file_size: u32,
    pub file_title: *mut u16,
    pub file_title_size: u32,
    pub directory: *const u16,
    pub title: *const u16,
    pub flags: u32,
    pub file_offset: u16,
    pub file_extension: u16,
    pub extension: *const u16,
    pub custom_data: isize,
    pub hook: *const c_void,
    pub template: *const u16,
    pub reserved: *mut c_void,
    pub reserved_size: u32,
    pub flags_ex: u32,
}

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

// Imported, since ReactOS's comdlg32.dll has TLS, which a DLL gets only
// when it loads with the program
#[link(name = "comdlg32")]
unsafe extern "system" {
    pub fn GetOpenFileNameW(names: *mut OpenFileName) -> i32;
    pub fn GetSaveFileNameW(names: *mut OpenFileName) -> i32;
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
