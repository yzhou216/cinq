// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of finding the widget that has the keyboard

use futures::{FutureExt, StreamExt};
use iced::Rectangle;
use iced_runtime::{Action, task::into_stream};

use super::{super::focus_order::move_focus, *};
use crate::front::update::tests::Field;

/// Visits one widget per entry of `fields`, as iced does for each window in
/// turn, each with the identity it names, and returns what `holding` finds
fn found(mut holding: Holding, fields: &[(Option<&'static str>, bool)]) -> bool {
    fields.iter().for_each(|&(id, focused)| {
        let id = id.map(Id::new);
        holding.focusable(id.as_ref(), Rectangle::default(), &mut Field::new(focused));
    });
    matches!(holding.finish(), Outcome::Some(true))
}

#[test]
fn a_focused_widget_anywhere_means_the_keyboard_is_held() {
    let holding = Holding::anywhere;
    assert!(found(
        holding(),
        &[(None, false), (None, true), (None, false)]
    ));
    assert!(!found(holding(), &[(None, false), (None, false)]));
    assert!(!found(holding(), &[]));
}

#[test]
fn only_the_focused_widget_of_the_named_field_counts_for_that_field() {
    let holding = || Holding::in_field("file");
    assert!(found(
        holding(),
        &[(Some("name"), false), (Some("file"), true)]
    ));
    assert!(!found(
        holding(),
        &[(Some("name"), true), (Some("file"), false)]
    ));
    assert!(!found(holding(), &[(None, true)]));
}

/// Window of two fields, as widgets that keep their focus between operations
struct Window(Vec<(Option<Id>, Field)>);

impl Window {
    /// Window with a Name field, which has the keyboard, and a File field
    fn new() -> Self {
        Self(vec![
            (Some(Id::new("name")), Field::new(true)),
            (Some(Id::new(view::FILE)), Field::new(false)),
        ])
    }

    /// Runs an operation as iced does, visiting the fields once for each link
    /// of its chain
    fn operate(&mut self, mut operation: Box<dyn Operation>) {
        loop {
            operation.traverse(&mut |operation| {
                self.0.iter_mut().for_each(|(id, field)| {
                    operation.focusable(id.as_ref(), Rectangle::default(), field);
                });
            });
            match operation.finish() {
                Outcome::Chain(next) => operation = next,
                _ => break,
            }
        }
    }

    /// Runs `task` to its end, and returns the messages that it gave.  As in
    /// iced, the operations that the task has ready run together, in order,
    /// and what they find reaches the task only afterwards, which takes a few
    /// polls of its stream.
    fn run(mut self, task: Task<Message>) -> Vec<Message> {
        let Some(mut stream) = into_stream(task) else {
            return Vec::new();
        };
        let mut messages = Vec::new();
        let mut ended = false;
        let mut quiet = 0;
        while !ended && quiet < POLLS_TO_SETTLE {
            let seen = messages.len();
            let mut operations = Vec::new();
            while let Some(next) = stream.next().now_or_never() {
                match next {
                    Some(Action::Output(message)) => messages.push(message),
                    Some(Action::Widget(operation)) => operations.push(operation),
                    Some(_) => {}
                    None => {
                        ended = true;
                        break;
                    }
                }
            }
            quiet = if operations.is_empty() && messages.len() == seen {
                quiet + 1
            } else {
                0
            };
            operations
                .into_iter()
                .for_each(|operation| self.operate(operation));
        }
        messages
    }
}

/// Polls of a task that give nothing, after which it has nothing more to say
const POLLS_TO_SETTLE: usize = 4;

/// What the status bar learns last, which is what it shows
fn last(messages: &[Message]) -> (Option<bool>, Option<bool>) {
    messages
        .iter()
        .fold((None, None), |(held, file), message| match message {
            Message::Held(now) => (Some(*now), file),
            Message::FileHeld(now) => (held, Some(*now)),
            _ => (held, file),
        })
}

#[test]
fn the_answer_after_tab_is_for_the_field_that_tab_reached() {
    let tab = || move_focus::<Message>(false);
    assert_eq!(
        last(&Window::new().run(held_after(tab()))),
        (Some(true), Some(true))
    );
    // Asked beside the move instead, each answer stays one Tab behind
    assert_eq!(
        last(&Window::new().run(Task::batch([tab(), held()]))),
        (Some(true), Some(false))
    );
}

#[test]
fn only_a_text_field_counts_when_the_keyboard_is_asked_for_in_text() {
    let visit = |text: bool| {
        let mut holding = Holding::in_text();
        let mut field = Field::new(true);
        if text {
            holding.text_input(None, Rectangle::default(), &mut field);
        }
        holding.focusable(None, Rectangle::default(), &mut field);
        matches!(holding.finish(), Outcome::Some(true))
    };
    assert!(visit(true));
    assert!(!visit(false), "a drop-down list offers no text input");
}

#[test]
fn any_address_of_the_light_patch_counts_for_the_help_on_addresses() {
    let holding = || Holding::in_field(view::ADDRESS);
    assert!(found(
        holding(),
        &[
            (Some("address"), false),
            (Some("address"), true),
            (Some("address"), false)
        ]
    ));
    assert!(!found(
        holding(),
        &[(Some("address"), false), (Some("name"), true)]
    ));
}

#[test]
fn a_check_box_is_told_from_a_drop_down_list_by_its_identity() {
    let holding = || Holding::in_field(view::CHECK);
    assert!(found(
        holding(),
        &[(Some("check"), false), (Some("check"), true)]
    ));
    assert!(!found(holding(), &[(None, true), (Some("check"), false)]));
}

#[test]
fn a_button_is_told_from_a_drop_down_list_by_its_identity() {
    let holding = || Holding::in_field(view::BUTTON);
    assert!(found(
        holding(),
        &[(Some("button"), false), (Some("button"), true)]
    ));
    assert!(!found(holding(), &[(None, true), (Some("button"), false)]));
}
