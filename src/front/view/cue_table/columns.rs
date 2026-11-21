// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Columns of the cue list

use iced::{Theme, widget::text};
use strum::{IntoStaticStr, VariantArray};
use unicode_width::UnicodeWidthStr;

use super::{RADIO, STATE_ROOM, TARGET_ROOM};
use crate::{
    back::{Cue, Groups, Kind, Problem, Workspace},
    front::view::{char_width, format, theme},
};

#[derive(Clone, Copy, PartialEq, Eq, VariantArray, IntoStaticStr)]
pub(super) enum Column {
    #[strum(serialize = "#")]
    Number,
    Type,
    Name,
    Target,
    Pre,
    Action,
    Post,
    Then,
    State,
}

impl Column {
    /// Columns that give way in turn, the first to go first, when the list
    /// is too narrow to leave a name room to be read
    pub(super) const YIELDING: [Column; 5] = [
        Column::Post,
        Column::Pre,
        Column::Then,
        Column::Type,
        Column::Target,
    ];

    pub(super) fn title(self) -> &'static str {
        self.into()
    }

    pub(super) fn text(self, workspace: &Workspace, groups: &Groups, cue: &Cue) -> String {
        match self {
            Column::Number => cue.number.clone(),
            Column::Type => cue.kind.to_string(),
            Column::Name => cue.name.clone(),
            Column::Target => format::target(workspace, groups, cue),
            Column::Pre => format::wait(cue.pre_wait.value().into()),
            Column::Action => format::action(groups, cue),
            // A cue in a group goes on to no cue and ignores its post-wait
            Column::Post | Column::Then if cue.depth > 0 && groups.group_of(cue.id).is_some() => {
                String::new()
            }
            Column::Post => format::post_wait(cue),
            Column::Then => format::advance(cue.advance).to_owned(),
            Column::State => {
                let next = groups.list().standby == Some(cue.id);
                state(
                    workspace,
                    cue,
                    next,
                    hidden_problem(workspace, groups, cue),
                    false,
                )
                .0
            }
        }
    }

    pub(super) fn holds_text(self, workspace: &Workspace, groups: &Groups) -> bool {
        groups
            .list()
            .cues
            .iter()
            .any(|cue| !self.text(workspace, groups, cue).is_empty())
    }

    /// Returns none for the name, which takes the room left
    pub(super) fn width(self, workspace: &Workspace, groups: &Groups) -> Option<f32> {
        let widest = || {
            groups
                .list()
                .cues
                .iter()
                .map(|cue| self.text(workspace, groups, cue).width())
                .fold(self.title().width(), usize::max)
        };
        let width = |columns: usize| columns as f32 * char_width();
        match self {
            Column::Name => None,
            Column::Number => Some(width(widest()) + RADIO),
            Column::Target => Some(width(widest().min(TARGET_ROOM))),
            Column::State => Some(width(STATE_ROOM)),
            _ => Some(width(widest())),
        }
    }
}

/// Describes a cue in the State column, in the color of what it needs.  A
/// file that is still loading stops the cue from playing, but it needs
/// nothing but time.  The list of running cues tells more of each, such as
/// where a paused cue stopped.  A cue that sends only to outputs that are gone
/// reads in yellow that it has none, which GO does not skip.  Red turns
/// lighter where the cue is `selected`, so that it stays legible on the
/// selected row.  A cue that is `next` reads NEXT, as the row of a collapsed
/// group does that hides the standby cue.  A collapsed group reads the
/// `hidden` problem of a cue that it hides as that cue would, so that GO does
/// not fire a broken cue out of sight, but gives way to the state of the group
/// while it runs.
pub(super) fn state(
    workspace: &Workspace,
    cue: &Cue,
    next: bool,
    hidden: Option<Problem>,
    selected: bool,
) -> (String, fn(&Theme) -> text::Style) {
    let problem = |problem: Problem| -> (String, fn(&Theme) -> text::Style) {
        let danger = theme::danger(selected);
        match problem {
            Problem::NotLoaded => ("loading".to_owned(), text::default),
            Problem::NoFile => ("NO FILE".to_owned(), danger),
            Problem::NoTarget | Problem::InTarget => ("NO TARGET".to_owned(), danger),
            Problem::NotLooping => ("NO LOOP".to_owned(), danger),
            Problem::NoDestination => ("NO DESTINATION".to_owned(), danger),
            Problem::NoMessage => ("NO MESSAGE".to_owned(), danger),
            Problem::LoadFailed(_) | Problem::EmptyClip | Problem::EmptyLoop => {
                ("BROKEN".to_owned(), danger)
            }
        }
    };
    match (workspace.problem(cue), workspace.active_of(cue.id)) {
        (Some(own), _) => problem(own),
        (None, Some(active)) if active.paused => ("paused".to_owned(), text::warning),
        (None, Some(active)) => (format::phase(active), text::success),
        (None, None) => match hidden {
            Some(hidden) => problem(hidden),
            None if workspace.sends_nowhere(cue) => ("NO OUTPUT".to_owned(), text::warning),
            None if next => ("NEXT".to_owned(), text::primary),
            None => (String::new(), text::default),
        },
    }
}

/// Returns the problem of the first cue that the collapsed group `cue` hides
/// which has one, or none for any other cue
pub(super) fn hidden_problem<'a>(
    workspace: &Workspace,
    groups: &Groups<'a>,
    cue: &Cue,
) -> Option<Problem<'a>> {
    (cue.kind == Kind::Group && cue.collapsed)
        .then(|| {
            groups
                .members(cue.id)
                .iter()
                .find_map(|member| workspace.problem(member))
        })
        .flatten()
}
