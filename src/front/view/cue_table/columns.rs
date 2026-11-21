// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Columns of the cue list

use iced::{Theme, widget::text};
use itertools::Itertools;
use strum::{IntoStaticStr, VariantArray};
use unicode_width::UnicodeWidthStr;

use super::{RADIO, STATE_ROOM, TARGET_MOST};
use crate::{
    back::{Cue, Groups, Kind, OutputId, Problem, Workspace},
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
                let hidden = hidden_problem(workspace, groups, cue);
                // The column has a fixed width, so that no output failing changes it
                lines(workspace, cue, next, hidden, &[], false)
                    .into_iter()
                    .map(|(line, _)| line)
                    .join("\n")
            }
        }
    }

    /// Returns whether the list shows the column at all, which a yielding
    /// column that holds no text in any row does not, as it tells nothing.
    /// A list with no cue keeps them all, so that its header names them.
    pub(super) fn shown(self, workspace: &Workspace, groups: &Groups) -> bool {
        let cues = &groups.list().cues;
        !Self::YIELDING.contains(&self)
            || cues.is_empty()
            || cues
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
            Column::Target => Some(width(widest().min(TARGET_MOST))),
            Column::State => Some(width(STATE_ROOM)),
            _ => Some(width(widest())),
        }
    }
}

/// A line of the State column with the style of its color
pub(super) type Line = (String, fn(&Theme) -> text::Style);

/// Returns the lines of the State column for a cue.  A cue that is `next`
/// reads NEXT on the first line, as the row of a collapsed group does that
/// hides the standby cue, so that the mark stays where GO lands whatever else
/// the cue needs to say, which follows on the next line.  A cue whose outputs
/// all `failing` plays on, in silence, which a last line of its own tells in
/// yellow, so that it hides neither the state of a cue that runs nor its time
/// left.
pub(super) fn lines(
    workspace: &Workspace,
    cue: &Cue,
    next: bool,
    hidden: Option<Problem>,
    failing: &[OutputId],
    selected: bool,
) -> Vec<Line> {
    let marked = next && workspace.active_of(cue.id).is_none();
    let mark = marked.then(|| ("NEXT".to_owned(), text::primary as fn(&Theme) -> _));
    let state = state(workspace, cue, hidden, selected);
    let failed = (workspace.problem(cue).is_none() && workspace.sends_to_failing(cue, failing))
        .then(|| ("OUTPUT FAILED".to_owned(), text::warning as fn(&Theme) -> _));
    mark.into_iter()
        .chain((!state.0.is_empty()).then_some(state))
        .chain(failed)
        .collect()
}

/// Describes a cue in the State column, in the color of what it needs.  A
/// file that is still loading stops the cue from playing, but it needs
/// nothing but time.  The list of running cues tells more of each, such as
/// where a paused cue stopped.  A cue that a fade or Panic is fading reads
/// fading, since the time left of its phase says nothing of that.  A cue that
/// sends only to outputs that are gone reads in yellow that it has none,
/// which GO does not skip.  Red turns lighter where the cue is `selected`, so
/// that it stays legible on the selected row.  A collapsed group reads the
/// `hidden` problem of a cue that it hides as that cue would, so that GO does
/// not fire a broken cue out of sight, but gives way to the state of the
/// group while it runs.
pub(super) fn state(
    workspace: &Workspace,
    cue: &Cue,
    hidden: Option<Problem>,
    selected: bool,
) -> Line {
    let problem = |problem: Problem| -> Line {
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
        // The fade leaves no room for the time left, which the fade cue tells
        (None, Some(active)) if active.fade.is_some() => ("fading".to_owned(), text::success),
        (None, Some(active)) => (format::phase(active), text::success),
        (None, None) => match hidden {
            Some(hidden) => problem(hidden),
            None if workspace.sends_nowhere(cue) => ("NO OUTPUT".to_owned(), text::warning),
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
