// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Drop-down lists whose menus the keys move through
//!
//! iced's pick list takes no keys, so keys pressed over its open menu act
//! on the cue list behind it.  These lists look like iced's pick list but
//! open a [`menu`] of their own, in which the keys of [`Action::Next`] and
//! [`Action::Previous`] move the highlight, which the menu keeps in view,
//! and Enter picks the highlighted option.  GO and Panic act as everywhere
//! and close the menu, so that the show is in view, and the menu keeps
//! every other key until it closes.
//!
//! The keys that move between fields move to these lists as well, which
//! then show the border of a focused field, and Enter opens the menu, which
//! takes the keys from there, unless Enter is bound to GO or Panic, which
//! act as everywhere.  The list hands the keyboard to its menu as it
//! opens, so that the keys act on the cue list again once the menu closes,
//! as they do after Enter leaves a text field.
//!
//! [`Action::Next`]: crate::front::keymap::Action::Next
//! [`Action::Previous`]: crate::front::keymap::Action::Previous

mod menu;
mod state;
mod widget;

use iced::{Element, Padding, widget::button};

use crate::front::{Message, keymap::Keymap};

/// Padding of the list and of each option of its menu, as iced's pick list
/// pads them
const PADDING: Padding = button::DEFAULT_PADDING;

pub struct Dropdown<'a> {
    names: Vec<String>,
    /// Index of the selected option
    chosen: Option<usize>,
    /// Makes the message of the option at an index, if there is one
    pick: Box<dyn Fn(usize) -> Option<Message> + 'a>,
    placeholder: Option<String>,
    keymap: &'a Keymap,
}

pub fn dropdown<'a, T: ToString + PartialEq + Clone + 'a>(
    options: impl Into<Vec<T>>,
    selected: Option<T>,
    on_select: impl Fn(T) -> Message + 'a,
    keymap: &'a Keymap,
) -> Dropdown<'a> {
    let options = options.into();
    Dropdown {
        names: options.iter().map(T::to_string).collect(),
        chosen: options
            .iter()
            .position(|option| Some(option) == selected.as_ref()),
        pick: Box::new(move |row| options.get(row).cloned().map(&on_select)),
        placeholder: None,
        keymap,
    }
}

impl Dropdown<'_> {
    pub fn placeholder(self, placeholder: impl Into<String>) -> Self {
        Self {
            placeholder: Some(placeholder.into()),
            ..self
        }
    }
}

impl<'a> From<Dropdown<'a>> for Element<'a, Message> {
    fn from(dropdown: Dropdown<'a>) -> Self {
        Element::new(dropdown)
    }
}
