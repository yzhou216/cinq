// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fields of the inspector, each a labelled input for one setting

use iced::{
    Element, Fill,
    widget::{row, text_input},
};
use strum::{IntoStaticStr, VariantArray};

use super::locked::locked;
use crate::{
    back::{Advance, Cue, Destination, Edit, Kind, osc_packet},
    front::{
        App, Message, Setting,
        view::{FILE, NAME, SPACING, button, dropdown, format, reached, theme},
    },
};

#[derive(Clone, Copy, PartialEq, IntoStaticStr)]
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
    /// Message of a network cue
    #[strum(serialize = "Message")]
    Osc,
}

impl Field {
    pub(super) fn label(self, cue: &Cue) -> &'static str {
        match self {
            Field::Target if cue.kind.has_file() => "File",
            Field::Target if cue.kind == Kind::Network => "Destination",
            _ => self.into(),
        }
    }

    /// Returns whether the cue has a row for this field, which a kind with
    /// no file and no target has no use for
    pub(super) fn has_row(self, cue: &Cue) -> bool {
        !matches!(self, Field::Target)
            || !matches!(
                cue.kind,
                Kind::Light | Kind::Group | Kind::Wait | Kind::Memo
            )
    }

    /// Returns why the cue ignores the value of this field, such as the
    /// post-wait of an auto-follow cue, the loop start of a cue that does not
    /// loop, or the post-wait and Then setting of a cue in a group, which goes
    /// on to no cue
    fn ignored(self, cue: &Cue, inside: bool) -> Option<&'static str> {
        match self {
            Field::PostWait | Field::Then if inside => Some("in a group"),
            Field::PostWait if cue.advance == Advance::Follow => Some("with auto-follow"),
            Field::LoopStart | Field::LoopEnd if !cue.looping => Some("not looping"),
            _ => None,
        }
    }

    /// Returns the input for this field, or its value in a box that the
    /// keyboard skips when the show cannot be edited or the cue ignores the
    /// value, with the reason that it does
    pub(super) fn input<'a>(self, app: &'a App, cue: &'a Cue) -> Element<'a, Message> {
        let id = cue.id;
        let editable = app.editable();
        let inside = app.current().group_of(id).is_some();
        let ignored = self.ignored(cue, inside);
        let has_file = cue.kind.has_file();
        let field = |value: &str, edit: fn(String) -> Edit| -> Element<'a, Message> {
            if !editable || ignored.is_some() {
                return locked(value, self.placeholder(cue), ignored, Fill);
            }
            let input = text_input(self.placeholder(cue), value)
                .on_input(move |text| match self {
                    // A path applies once the field is left, as a level does
                    Field::Target if has_file => Message::SettingTyped(Setting::File(id), text),
                    _ => Message::Edit(id, edit(text)),
                })
                .style(theme::field(self.valid(cue)));
            // Enter in the empty File field opens the chooser, since Tab skips
            // the Choose button, and leaves the field once a file is named
            reached(match self {
                Field::Name => input.id(NAME),
                Field::Target if has_file => input
                    .id(FILE)
                    .on_submit_maybe(value.is_empty().then_some(Message::ChooseFile(id))),
                _ => input,
            })
        };
        match self {
            Field::Number => field(&cue.number, Edit::Number),
            Field::Name => field(&cue.name, Edit::Name),
            Field::Target => match cue.kind {
                Kind::Audio | Kind::Video | Kind::Picture => row![
                    field(&typed_file(app, cue), Edit::File),
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
                Kind::Network => field(&cue.destination, Edit::Destination),
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
            Field::Then => locked(&cue.advance.to_string(), "", ignored, Fill),
            Field::Start => field(cue.start.text(), |text| Edit::Start(text.into())),
            Field::End => field(cue.end.text(), |text| Edit::End(text.into())),
            Field::LoopStart => field(cue.loop_start.text(), |text| Edit::LoopStart(text.into())),
            Field::LoopEnd => field(cue.loop_end.text(), |text| Edit::LoopEnd(text.into())),
            Field::Osc => field(&cue.message, Edit::Message),
        }
    }

    /// Returns the field that takes this one's place in the cue: a network
    /// cue has no duration, so its notes take that row, and its message,
    /// which is as long as a name, takes the row of the notes in the wide
    /// form
    pub(super) fn of(self, cue: &Cue) -> Self {
        match self {
            Field::Notes if cue.kind == Kind::Network => Field::Osc,
            Field::Duration if cue.kind == Kind::Network => Field::Notes,
            _ => self,
        }
    }

    /// Returns whether the field holds a time that reads as no time
    fn unreadable(self, cue: &Cue) -> bool {
        match self {
            Field::PreWait => !cue.pre_wait.is_valid(),
            Field::Duration => cue.kind.has_duration() && !cue.duration.is_valid(),
            Field::PostWait => !cue.post_wait.is_valid(),
            Field::Start => !cue.start.is_valid(),
            Field::End => !cue.end.is_valid(),
            Field::LoopStart => !cue.loop_start.is_valid(),
            Field::LoopEnd => !cue.loop_end.is_valid(),
            _ => false,
        }
    }

    /// Returns whether the text of the field reads as a value, which only
    /// a time, a destination, or a message can fail to
    fn valid(self, cue: &Cue) -> bool {
        match self {
            Field::Target if cue.kind == Kind::Network => {
                Destination::parse(&cue.destination).is_some()
            }
            Field::Osc => osc_packet(&cue.message).is_some(),
            _ => !self.unreadable(cue),
        }
    }

    fn placeholder(self, cue: &Cue) -> &'static str {
        match self {
            Field::Target if cue.kind == Kind::Audio => "path of a sound file",
            Field::Target if cue.kind == Kind::Video => "path of a video file",
            Field::Target if cue.kind == Kind::Picture => "path of a picture file",
            Field::Target if cue.kind == Kind::Network => "host:port or udp://host:port",
            Field::Osc => "/address arguments",
            Field::PreWait | Field::Duration | Field::PostWait | Field::Start => "0",
            Field::End => "end of file",
            Field::LoopStart => "start of cue",
            Field::LoopEnd => "end of cue",
            _ => "",
        }
    }
}

/// Returns the sentence that tells which of the time fields in `columns`
/// reads as no time, if one does, since such a time counts as zero, except
/// for an end or a loop time, which is then left empty.  A field that the
/// cue ignores is passed over, as `inside` a group or not, since it does
/// nothing and the view shows it greyed out.
pub(super) fn unread(cue: &Cue, inside: bool, columns: &[&[Field]]) -> Option<String> {
    columns
        .iter()
        .copied()
        .flatten()
        .filter(|field| field.ignored(cue, inside).is_none())
        .find(|field| field.unreadable(cue))
        .map(|field| {
            let reading = match field {
                Field::End | Field::LoopStart | Field::LoopEnd => "reads as empty",
                _ => "counts as zero",
            };
            format!(
                "{} {reading}: use seconds or minutes:seconds",
                field.label(cue)
            )
        })
}

/// Shows a value in a box that the keyboard skips, so it lines up with
/// editable ones
fn shown<'a>(value: &str) -> Element<'a, Message> {
    locked(value, "", None, Fill)
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
    let placeholder = target_placeholder(targets.is_empty());
    dropdown(
        targets,
        selected,
        move |target: &Cue| Message::Edit(id, Edit::Target(Some(target.id))),
        &app.config.keymap,
    )
    .placeholder(placeholder)
    .into()
}

/// Returns what the target menu says while nothing is chosen, which is not
/// an invitation to choose where the list holds no cue to choose
fn target_placeholder(none_to_choose: bool) -> &'static str {
    if none_to_choose {
        "no cue to choose"
    } else {
        "choose a cue"
    }
}

/// Returns the path that the File field of `cue` shows: what was typed into
/// it while the typing waits for the field to be left, or else the file
fn typed_file(app: &App, cue: &Cue) -> String {
    match &app.typed {
        Some((Setting::File(typed), text)) if *typed == cue.id => text.clone(),
        _ => cue.file.clone(),
    }
}

#[cfg(test)]
mod tests;
