// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fields of the inspector, each a labelled input for one setting

use iced::{
    Element,
    widget::{row, text_input},
};
use strum::{IntoStaticStr, VariantArray};

use crate::{
    back::{Advance, Cue, Destination, Edit, Kind, osc_packet},
    front::{
        App, Message,
        view::{FILE, NAME, SPACING, button, dropdown, format, theme},
    },
};

#[derive(Clone, Copy, IntoStaticStr)]
pub(super) enum Field {
    Number,
    Name,
    Target,
    Notes,
    #[strum(serialize = "Pre-wait")]
    PreWait,
    Duration,
    #[strum(serialize = "Post-wait")]
    PostWait,
    Then,
    Start,
    End,
    #[strum(serialize = "Loop start")]
    LoopStart,
    #[strum(serialize = "Loop end")]
    LoopEnd,
}

impl Field {
    pub(super) fn label(self, cue: &Cue) -> &'static str {
        match self {
            Field::Target if cue.kind.has_file() => "File",
            Field::Target if cue.kind == Kind::Network => "Message",
            _ => self.into(),
        }
    }

    /// Returns the input for this field, disabled when the value does not
    /// apply to the cue, such as the post-wait of an auto-follow cue, which
    /// ignores it, the loop start of a cue that does not loop, or the post-wait
    /// and Then setting of a cue in a group, which goes on to no cue
    pub(super) fn input<'a>(self, app: &'a App, cue: &'a Cue) -> Element<'a, Message> {
        let id = cue.id;
        let editable = app.editable();
        let inside = app.current().group_of(id).is_some();
        let applies = match self {
            Field::PostWait => cue.advance != Advance::Follow && !inside,
            Field::LoopStart | Field::LoopEnd => cue.looping,
            _ => true,
        };
        let field = |value: &str, edit: fn(String) -> Edit| -> Element<'a, Message> {
            let input = text_input(self.placeholder(cue), value)
                .on_input_maybe(
                    (editable && applies).then_some(move |text| Message::Edit(id, edit(text))),
                )
                .style(theme::field(self.valid(cue)));
            // Enter in the empty File field opens the chooser, since Tab skips
            // the Choose button, and leaves the field once a file is named
            match self {
                Field::Name => input.id(NAME),
                Field::Target if cue.kind.has_file() => input.id(FILE).on_submit_maybe(
                    (editable && value.is_empty()).then_some(Message::ChooseFile(id)),
                ),
                _ => input,
            }
            .into()
        };
        match self {
            Field::Number => field(&cue.number, Edit::Number),
            Field::Name => field(&cue.name, Edit::Name),
            Field::Target => match cue.kind {
                Kind::Audio | Kind::Video | Kind::Picture => row![
                    field(&cue.file, Edit::File),
                    button("Choose").on_press_maybe(editable.then_some(Message::ChooseFile(id))),
                ]
                .spacing(SPACING)
                .into(),
                Kind::Fade | Kind::Start | Kind::Stop | Kind::Pause | Kind::Devamp | Kind::GoTo
                    if editable =>
                {
                    target_menu(app, cue)
                }
                Kind::Fade | Kind::Start | Kind::Stop | Kind::Pause | Kind::Devamp | Kind::GoTo => {
                    shown(
                        &app.workspace
                            .target(cue)
                            .map_or_else(String::new, Cue::to_string),
                    )
                }
                Kind::Network => network(cue, editable),
                Kind::Light | Kind::Group | Kind::Wait | Kind::Memo => shown(""),
            },
            Field::Notes => field(&cue.notes, Edit::Notes),
            Field::PreWait => field(cue.pre_wait.text(), |text| Edit::PreWait(text.into())),
            Field::Duration if cue.kind.has_duration() => {
                field(cue.duration.text(), |text| Edit::Duration(text.into()))
            }
            Field::Duration => shown(&format::action(&app.current().groups(), cue)),
            Field::PostWait => field(cue.post_wait.text(), |text| Edit::PostWait(text.into())),
            Field::Then if editable && !inside => dropdown(
                Advance::VARIANTS,
                Some(cue.advance),
                move |advance| Message::Edit(id, Edit::Advance(advance)),
                &app.config.keymap,
            )
            .into(),
            Field::Then => shown(&cue.advance.to_string()),
            Field::Start => field(cue.start.text(), |text| Edit::Start(text.into())),
            Field::End => field(cue.end.text(), |text| Edit::End(text.into())),
            Field::LoopStart => field(cue.loop_start.text(), |text| Edit::LoopStart(text.into())),
            Field::LoopEnd => field(cue.loop_end.text(), |text| Edit::LoopEnd(text.into())),
        }
    }

    /// Returns whether the text of the field reads as a value, which only
    /// a time can fail to
    fn valid(self, cue: &Cue) -> bool {
        match self {
            Field::PreWait => cue.pre_wait.is_valid(),
            Field::Duration => cue.duration.is_valid(),
            Field::PostWait => cue.post_wait.is_valid(),
            Field::Start => cue.start.is_valid(),
            Field::End => cue.end.is_valid(),
            Field::LoopStart => cue.loop_start.is_valid(),
            Field::LoopEnd => cue.loop_end.is_valid(),
            _ => true,
        }
    }

    fn placeholder(self, cue: &Cue) -> &'static str {
        match self {
            Field::Target if cue.kind == Kind::Audio => "path of a sound file",
            Field::Target if cue.kind == Kind::Video => "path of a video file",
            Field::Target if cue.kind == Kind::Picture => "path of a picture file",
            Field::PreWait | Field::Duration | Field::PostWait | Field::Start => "0",
            Field::End => "end of file",
            Field::LoopStart => "start of cue",
            Field::LoopEnd => "end of cue",
            _ => "",
        }
    }
}

/// Returns the destination and the message of a network cue side by side,
/// sharing the row evenly, so that a narrow window leaves both readable
fn network<'a>(cue: &Cue, editable: bool) -> Element<'a, Message> {
    let id = cue.id;
    let destination = text_input("host:port", &cue.destination)
        .on_input_maybe(editable.then_some(move |text| Message::Edit(id, Edit::Destination(text))))
        .style(theme::field(Destination::parse(&cue.destination).is_some()));
    let message = text_input("/address arguments", &cue.message)
        .on_input_maybe(editable.then_some(move |text| Message::Edit(id, Edit::Message(text))))
        .style(theme::field(osc_packet(&cue.message).is_some()));
    row![destination, message].spacing(SPACING).into()
}

/// Shows a value in a disabled field, so it lines up with editable ones
fn shown<'a>(value: &str) -> Element<'a, Message> {
    text_input("", value).into()
}

fn target_menu<'a>(app: &'a App, cue: &'a Cue) -> Element<'a, Message> {
    let id = cue.id;
    let targets: Vec<&Cue> = app
        .workspace
        .cues()
        .filter(|other| cue.can_target(other))
        .collect();
    let selected = targets
        .iter()
        .find(|target| Some(target.id) == cue.target)
        .copied();
    dropdown(
        targets,
        selected,
        move |target: &Cue| Message::Edit(id, Edit::Target(Some(target.id))),
        &app.config.keymap,
    )
    .placeholder("choose a cue")
    .into()
}
