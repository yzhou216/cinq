// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Shell items of the file dialogs, which hold a path and nothing else
//!
//! rfd reads the dialog's choice from one, and names the directory that the
//! dialog starts in with one that `SHCreateItemFromParsingName` makes,
//! which ReactOS's shell32.dll lacks.

use std::{ffi::c_void, ptr, sync::atomic::AtomicU32};

use super::{
    Object, Slot, UNKNOWN, add_ref, fail1, fail2, fail3, fail4, query_interface, release, slot,
    wide,
};
use crate::reactos::{
    imports::import,
    system::{CoTaskMemAlloc, E_OUTOFMEMORY, Guid},
};

/// IShellItem's methods
static ITEM: [Slot; 8] = [
    slot!(query_interface::<Item>),
    slot!(add_ref::<Item>),
    slot!(release::<Item>),
    slot!(fail4),
    slot!(fail1),
    slot!(display_name),
    slot!(fail2),
    slot!(fail3),
];

import!(
    "shell32.dll",
    16,
    fn SHCreateItemFromParsingName(
        path: *const u16,
        context: *mut c_void,
        interface: *const Guid,
        item: *mut *mut c_void,
    ) -> i32 {
        let made = new(unsafe { wide(path) });
        unsafe {
            let status = query_interface(made.cast::<Item>(), interface, item);
            release(made.cast::<Item>());
            status
        }
    }
);

#[repr(C)]
struct Item {
    vtable: &'static [Slot; 8],
    references: AtomicU32,
    path: Vec<u16>,
}

impl Object for Item {
    /// IUnknown and IShellItem
    const INTERFACES: &'static [Guid] = &[
        UNKNOWN,
        Guid(
            0x4382_6d1e,
            0xe718,
            0x42ee,
            [0xbc, 0x55, 0xa1, 0xe2, 0x61, 0xc3, 0x7b, 0xfe],
        ),
    ];

    fn references(&self) -> &AtomicU32 {
        &self.references
    }
}

/// Item of `path`, which ends in NUL, with one reference
pub fn new(path: Vec<u16>) -> *mut c_void {
    let item = Item {
        vtable: &ITEM,
        references: AtomicU32::new(1),
        path,
    };
    Box::into_raw(Box::new(item)).cast()
}

/// Path of `item`, if it is one of these
pub fn path(item: *mut c_void) -> Option<Vec<u16>> {
    let item = item.cast::<Item>();
    let ours = !item.is_null() && ptr::eq(unsafe { (*item).vtable }, &ITEM);
    ours.then(|| unsafe { &(*item).path }.clone())
}

/// Path in memory that the caller frees with `CoTaskMemFree`, in whatever
/// form it asks for, since a path is all an item has
unsafe extern "system" fn display_name(this: *mut Item, _form: u32, name: *mut *mut u16) -> i32 {
    let path = &unsafe { &*this }.path;
    let copy = unsafe { CoTaskMemAlloc(size_of_val(path.as_slice())) }.cast::<u16>();
    unsafe { *name = copy };
    if copy.is_null() {
        return E_OUTOFMEMORY;
    }
    unsafe { copy.copy_from_nonoverlapping(path.as_ptr(), path.len()) };
    0
}
