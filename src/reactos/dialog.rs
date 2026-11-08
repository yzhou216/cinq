// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! File dialogs of rfd, whose COM classes ReactOS does not register
//!
//! rfd shows the file dialogs of later Windows, which ReactOS 0.4.16 lacks
//! and later ReactOS registers only when built for Vista's API.  Where
//! `CoCreateInstance` finds no class for them, it makes an object of the
//! program's own, which shows the older dialogs of comdlg32.dll and
//! implements what rfd calls of their interfaces.  The methods that rfd
//! never calls fail, each taking as many arguments as the interface's own,
//! since the callee pops them on i686.

mod item;

use std::{
    cell::RefCell,
    ffi::c_void,
    ptr, slice,
    sync::atomic::{AtomicU32, Ordering},
};

use super::{
    imports::{Resolved, define},
    system::{
        E_CANCELLED, E_NOINTERFACE, E_NOTIMPL, E_UNEXPECTED, GetOpenFileNameW, GetSaveFileNameW,
        Guid, OpenFileName, Proc, REGDB_E_CLASSNOTREG,
    },
};

type Slot = unsafe extern "system" fn();

/// Slot of a vtable for `$function`, whatever its arguments
macro_rules! slot {
    ($function:expr) => {
        unsafe { std::mem::transmute::<*const (), Slot>($function as *const ()) }
    };
}
use slot;

const FILE_OPEN_DIALOG: Guid = Guid(
    0xdc1c_5a9c,
    0xe88a,
    0x4dde,
    [0xa5, 0xa1, 0x60, 0xf8, 0x2a, 0x20, 0xae, 0xf7],
);
const FILE_SAVE_DIALOG: Guid = Guid(
    0xc0b4_e2f3,
    0xba21,
    0x4773,
    [0x8d, 0xba, 0x33, 0x5e, 0xc9, 0x46, 0xeb, 0x8b],
);
const UNKNOWN: Guid = Guid(0, 0, 0, [0xc0, 0, 0, 0, 0, 0, 0, 0x46]);

const OFN_OVERWRITEPROMPT: u32 = 0x2;
const OFN_HIDEREADONLY: u32 = 0x4;
const OFN_NOCHANGEDIR: u32 = 0x8;
const OFN_PATHMUSTEXIST: u32 = 0x800;
const OFN_FILEMUSTEXIST: u32 = 0x1000;
const OFN_EXPLORER: u32 = 0x8_0000;

/// IFileOpenDialog's methods up to GetResults and GetSelectedItems, or
/// IFileSaveDialog's up to ApplyProperties
static DIALOG: [Slot; 32] = [
    slot!(query_interface::<Dialog>),
    slot!(add_ref::<Dialog>),
    slot!(release::<Dialog>),
    slot!(show),
    slot!(set_file_types),
    slot!(fail1),
    slot!(fail1),
    slot!(fail2),
    slot!(fail1),
    slot!(fail1),
    slot!(fail1),
    slot!(fail1),
    slot!(set_folder),
    slot!(fail1),
    slot!(fail1),
    slot!(set_file_name),
    slot!(fail1),
    slot!(set_title),
    slot!(fail1),
    slot!(fail1),
    slot!(get_result),
    slot!(fail2),
    slot!(set_default_extension),
    slot!(fail1),
    slot!(fail1),
    slot!(fail0),
    slot!(fail1),
    slot!(fail1),
    slot!(fail1),
    slot!(fail2),
    slot!(fail1),
    slot!(fail4),
];

define!(
    None::<Proc>,
    20,
    fn CoCreateInstance(
        class: *const Guid,
        outer: *mut c_void,
        context: u32,
        interface: *const Guid,
        object: *mut *mut c_void,
    ) -> i32 {
        create_instance(class, outer, context, interface, object)
    }
);

fn create_instance(
    class: *const Guid,
    outer: *mut c_void,
    context: u32,
    interface: *const Guid,
    object: *mut *mut c_void,
) -> i32 {
    type Create = unsafe extern "system" fn(
        *const Guid,
        *mut c_void,
        u32,
        *const Guid,
        *mut *mut c_void,
    ) -> i32;
    static REAL: Resolved = Resolved::new();
    let real = REAL
        .get("ole32.dll\0", "CoCreateInstance\0")
        .expect("ole32.dll creates objects");
    let real = unsafe { std::mem::transmute::<Proc, Create>(real) };
    let status = unsafe { real(class, outer, context, interface, object) };
    let dialog = [FILE_OPEN_DIALOG, FILE_SAVE_DIALOG]
        .iter()
        .position(|dialog| unsafe { &*class } == dialog);
    match dialog.filter(|_| status == REGDB_E_CLASSNOTREG) {
        Some(save) => Dialog::create(save == 1, interface, object),
        None => status,
    }
}

/// COM object, whose first field is its vtable
trait Object: Sized {
    const INTERFACES: &'static [Guid];

    fn references(&self) -> &AtomicU32;
}

#[repr(C)]
struct Dialog {
    vtable: &'static [Slot; 32],
    references: AtomicU32,
    save: bool,
    state: RefCell<State>,
}

/// What rfd set before it shows the dialog, each ending in NUL, or empty
#[derive(Default)]
struct State {
    filter: Vec<u16>,
    extension: Vec<u16>,
    title: Vec<u16>,
    directory: Vec<u16>,
    file: Vec<u16>,
    chosen: Option<Vec<u16>>,
}

impl Object for Dialog {
    /// IUnknown, IModalWindow, IFileDialog, IFileOpenDialog, and
    /// IFileSaveDialog
    const INTERFACES: &'static [Guid] = &[
        UNKNOWN,
        Guid(
            0xb4db_1657,
            0x70d7,
            0x485e,
            [0x8e, 0x3e, 0x6f, 0xcb, 0x5a, 0x5c, 0x18, 0x02],
        ),
        Guid(
            0x42f8_5136,
            0xdb7e,
            0x439c,
            [0x85, 0xf1, 0xe4, 0x07, 0x5d, 0x13, 0x5f, 0xc8],
        ),
        Guid(
            0xd57c_7288,
            0xd4ad,
            0x4768,
            [0xbe, 0x02, 0x9d, 0x96, 0x95, 0x32, 0xd9, 0x60],
        ),
        Guid(
            0x84bc_cd23,
            0x5fde,
            0x4cdb,
            [0xae, 0xa4, 0xaf, 0x64, 0xb8, 0x3d, 0x78, 0xab],
        ),
    ];

    fn references(&self) -> &AtomicU32 {
        &self.references
    }
}

impl Dialog {
    fn create(save: bool, interface: *const Guid, object: *mut *mut c_void) -> i32 {
        let dialog = Box::into_raw(Box::new(Self {
            vtable: &DIALOG,
            references: AtomicU32::new(1),
            save,
            state: RefCell::default(),
        }));
        unsafe {
            let status = query_interface(dialog, interface, object);
            release(dialog);
            status
        }
    }
}

unsafe extern "system" fn show(this: *mut Dialog, owner: *mut c_void) -> i32 {
    let dialog = unsafe { &*this };
    let mut state = dialog.state.borrow_mut();
    let mut file = state.file.clone();
    file.resize(32_768, 0);
    let flags = if dialog.save {
        OFN_OVERWRITEPROMPT
    } else {
        OFN_FILEMUSTEXIST | OFN_HIDEREADONLY
    };
    let mut names = OpenFileName {
        size: size_of::<OpenFileName>() as u32,
        owner,
        filter: text(&state.filter),
        filter_index: 1,
        file: file.as_mut_ptr(),
        file_size: file.len() as u32,
        directory: text(&state.directory),
        title: text(&state.title),
        flags: flags | OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        extension: text(&state.extension),
        ..unsafe { std::mem::zeroed() }
    };
    let shown = unsafe {
        if dialog.save {
            GetSaveFileNameW(&mut names)
        } else {
            GetOpenFileNameW(&mut names)
        }
    };
    state.chosen = (shown != 0).then(|| {
        file.iter()
            .copied()
            .take_while(|&unit| unit != 0)
            .chain([0])
            .collect()
    });
    if shown != 0 { 0 } else { E_CANCELLED }
}

/// Filter of the older dialogs from rfd's pairs of name and pattern: each
/// ending in NUL, and the whole in another
unsafe extern "system" fn set_file_types(
    this: *mut Dialog,
    count: u32,
    specs: *const [*const u16; 2],
) -> i32 {
    let specs = unsafe { slice::from_raw_parts(specs, count as usize) };
    let filter = specs
        .iter()
        .flatten()
        .flat_map(|&text| unsafe { wide(text) })
        .chain([0])
        .collect();
    unsafe { &*this }.state.borrow_mut().filter = filter;
    0
}

unsafe extern "system" fn set_folder(this: *mut Dialog, folder: *mut c_void) -> i32 {
    if let Some(path) = item::path(folder) {
        unsafe { &*this }.state.borrow_mut().directory = path;
    }
    0
}

unsafe extern "system" fn set_file_name(this: *mut Dialog, name: *const u16) -> i32 {
    unsafe { &*this }.state.borrow_mut().file = unsafe { wide(name) };
    0
}

unsafe extern "system" fn set_title(this: *mut Dialog, title: *const u16) -> i32 {
    unsafe { &*this }.state.borrow_mut().title = unsafe { wide(title) };
    0
}

unsafe extern "system" fn set_default_extension(this: *mut Dialog, extension: *const u16) -> i32 {
    unsafe { &*this }.state.borrow_mut().extension = unsafe { wide(extension) };
    0
}

unsafe extern "system" fn get_result(this: *mut Dialog, result: *mut *mut c_void) -> i32 {
    let chosen = unsafe { &*this }.state.borrow().chosen.clone();
    unsafe { *result = chosen.map_or(ptr::null_mut(), item::new) };
    if unsafe { *result }.is_null() {
        E_UNEXPECTED
    } else {
        0
    }
}

unsafe extern "system" fn query_interface<T: Object>(
    this: *mut T,
    interface: *const Guid,
    object: *mut *mut c_void,
) -> i32 {
    let known = T::INTERFACES.contains(unsafe { &*interface });
    if known {
        unsafe { add_ref(this) };
    }
    unsafe { *object = if known { this.cast() } else { ptr::null_mut() } };
    if known { 0 } else { E_NOINTERFACE }
}

unsafe extern "system" fn add_ref<T: Object>(this: *mut T) -> u32 {
    unsafe { &*this }
        .references()
        .fetch_add(1, Ordering::Relaxed)
        + 1
}

unsafe extern "system" fn release<T: Object>(this: *mut T) -> u32 {
    let left = unsafe { &*this }
        .references()
        .fetch_sub(1, Ordering::AcqRel)
        - 1;
    if left == 0 {
        drop(unsafe { Box::from_raw(this) });
    }
    left
}

unsafe extern "system" fn fail0(_: *mut c_void) -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn fail1(_: *mut c_void, _: usize) -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn fail2(_: *mut c_void, _: usize, _: usize) -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn fail3(_: *mut c_void, _: usize, _: usize, _: usize) -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn fail4(_: *mut c_void, _: usize, _: usize, _: usize, _: usize) -> i32 {
    E_NOTIMPL
}

/// Text that `text` points to, with its NUL
unsafe fn wide(text: *const u16) -> Vec<u16> {
    let length = (0..)
        .take_while(|&offset| unsafe { *text.add(offset) } != 0)
        .count();
    unsafe { slice::from_raw_parts(text, length + 1) }.to_vec()
}

fn text(wide: &[u16]) -> *const u16 {
    if wide.is_empty() {
        ptr::null()
    } else {
        wide.as_ptr()
    }
}
