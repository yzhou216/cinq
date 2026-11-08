// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Imports of winit, rfd, dirs, sys-locale, cpal, and the windows crate

use std::{ffi::c_void, ptr};

use super::{import, not_implemented};
use crate::reactos::system::{
    CoTaskMemAlloc, E_INVALIDARG, E_NOTIMPL, E_OUTOFMEMORY, Guid, SHGetFolderPathW,
};

/// Known folders that cinq looks up, with the CSIDL of each, by which
/// `SHGetFolderPathW` finds it: RoamingAppData and LocalAppData
const FOLDERS: [(Guid, i32); 2] = [
    (
        Guid(
            0x3eb6_85db,
            0x65f9,
            0x4cf6,
            [0xa0, 0x3a, 0xe3, 0xef, 0x65, 0x72, 0x9f, 0x3d],
        ),
        0x1a,
    ),
    (
        Guid(
            0xf1b3_2785,
            0x6fba,
            0x4fcf,
            [0x9d, 0x55, 0x7b, 0x8e, 0x7f, 0x15, 0x70, 0x91],
        ),
        0x1c,
    ),
];

import!(
    "user32.dll",
    8,
    fn RegisterTouchWindow(window: *mut c_void, flags: u32) -> i32 {
        not_implemented(0)
    }
);
import!(
    "user32.dll",
    16,
    fn GetTouchInputInfo(input: *mut c_void, count: u32, inputs: *mut c_void, size: i32) -> i32 {
        not_implemented(0)
    }
);
import!(
    "user32.dll",
    4,
    fn CloseTouchInputHandle(input: *mut c_void) -> i32 {
        not_implemented(0)
    }
);
import!(
    "user32.dll",
    0,
    fn IsProcessDPIAware() -> i32 {
        0
    }
);
import!(
    "user32.dll",
    8,
    fn SetWindowDisplayAffinity(window: *mut c_void, affinity: u32) -> i32 {
        not_implemented(0)
    }
);
import!(
    "dwmapi.dll",
    8,
    fn DwmEnableBlurBehindWindow(window: *mut c_void, blur: *const c_void) -> i32 {
        E_NOTIMPL
    }
);
import!(
    "dwmapi.dll",
    16,
    fn DwmSetWindowAttribute(
        window: *mut c_void,
        attribute: u32,
        value: *const c_void,
        size: u32,
    ) -> i32 {
        E_NOTIMPL
    }
);
import!(
    "shell32.dll",
    16,
    fn SHGetKnownFolderPath(
        id: *const c_void,
        flags: u32,
        token: *mut c_void,
        path: *mut *mut u16,
    ) -> i32 {
        known_folder(id.cast(), path)
    }
);
import!(
    "shell32.dll",
    16,
    fn SHCreateItemFromParsingName(
        path: *const u16,
        context: *mut c_void,
        interface: *const c_void,
        item: *mut *mut c_void,
    ) -> i32 {
        unsafe { *item = ptr::null_mut() };
        E_NOTIMPL
    }
);
import!(
    "kernel32.dll",
    16,
    fn GetUserPreferredUILanguages(
        flags: u32,
        count: *mut u32,
        languages: *mut u16,
        size: *mut u32,
    ) -> i32 {
        not_implemented(0)
    }
);
import!(
    "mmdevapi.dll",
    20,
    fn ActivateAudioInterfaceAsync(
        path: *const u16,
        interface: *const c_void,
        parameters: *const c_void,
        handler: *mut c_void,
        operation: *mut *mut c_void,
    ) -> i32 {
        unsafe { *operation = ptr::null_mut() };
        E_NOTIMPL
    }
);
import!(
    "combase.dll",
    12,
    fn RoOriginateErrorW(error: i32, length: u32, message: *const u16) -> i32 {
        0
    }
);

/// Path of known folder `id` from the older `SHGetFolderPathW`, in memory
/// that the caller frees with `CoTaskMemFree`
fn known_folder(id: *const Guid, path: *mut *mut u16) -> i32 {
    unsafe { *path = ptr::null_mut() };
    let Some(&(_, csidl)) = FOLDERS.iter().find(|(folder, _)| unsafe { &*id } == folder) else {
        return E_INVALIDARG;
    };
    let mut buffer = [0_u16; 260];
    let status = unsafe {
        SHGetFolderPathW(
            ptr::null_mut(),
            csidl,
            ptr::null_mut(),
            0,
            buffer.as_mut_ptr(),
        )
    };
    if status < 0 {
        return status;
    }
    let length = buffer
        .iter()
        .position(|&unit| unit == 0)
        .map_or(buffer.len(), |end| end + 1);
    let copy = unsafe { CoTaskMemAlloc(length * size_of::<u16>()) }.cast::<u16>();
    if copy.is_null() {
        return E_OUTOFMEMORY;
    }
    unsafe {
        copy.copy_from_nonoverlapping(buffer.as_ptr(), length);
        *path = copy;
    }
    0
}
