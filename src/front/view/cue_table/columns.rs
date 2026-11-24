// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Columns of the cue list

use std::collections::BTreeSet;

use iced::{Theme, widget::text};
use itertools::Itertools;
use strum::{IntoStaticStr, VariantArray};
use unicode_width::UnicodeWidthStr;

use super::{RADIO, STATE_ROOM, TARGET_MOST};
use crate::{
    back::{Cue, Groups, Kind, OutputId, Problem, Unlit, Workspace},
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
    /// is too narrow to leave a name room to be read.  The Action column goes
    /// last, so that the State column can keep a word of its states and the
    /// name its least beside the number.
    pub(super) const YIELDING: [Column; 6] = [
        Column::Post,
        Column::Pre,
        Column::Then,
        Column::Type,
        Column::Target,
        Column::Action,
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
                // The column has a fixed width, so that no output failing changes it
                shown_lines(workspace, groups, cue, next, &[], &BTreeSet::new(), false)
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

    /// Returns the columns of the font that the longest text of the column
    /// takes, its title included
    pub(super) fn widest(self, workspace: &Workspace, groups: &Groups) -> usize {
        groups
            .list()
            .cues
            .iter()
            .flat_map(|cue| {
                self.text(workspace, groups, cue)
                    .lines()
                    .map(UnicodeWidthStr::width)
                    .collect::<Vec<_>>()
            })
            .fold(self.title().width(), usize::max)
    }

    /// Returns none for the name, which takes the room left
    pub(super) fn width(self, workspace: &Workspace, groups: &Groups) -> Option<f32> {
        let widest = || self.widest(workspace, groups);
        let width = |columns: usize| columns as f32 * char_width();
        match self {
            Column::Name => None,
            Column::Number => Some(width(widest()) + RADIO),
            Column::Target => Some(width(widest().min(TARGET_MOST))),
            Column::State => Some(width(widest().max(STATE_ROOM))),
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
/// left.  A network cue whose destination is among `unreached` tells so in
/// the same way, until a message gets there.
pub(super) fn lines(
    workspace: &Workspace,
    cue: &Cue,
    next: bool,
    hidden: Option<Problem>,
    failing: &[OutputId],
    unreached: &BTreeSet<String>,
    selected: bool,
) -> Vec<Line> {
    let mark = next.then(|| ("NEXT".to_owned(), text::primary as fn(&Theme) -> _));
    let state = state(workspace, cue, hidden, selected);
    let sendable = workspace.problem(cue).is_none();
    let warning = |told: &str| (told.to_owned(), text::warning as fn(&Theme) -> _);
    let failed =
        (sendable && workspace.sends_to_failing(cue, failing)).then(|| warning("OUTPUT FAILED"));
    let unsent =
        (sendable && workspace.sends_to_failed(cue, unreached)).then(|| warning("SEND FAILED"));
    mark.into_iter()
        .chain((!state.0.is_empty()).then_some(state))
        .chain(failed)
        .chain(unsent)
        .collect()
}

/// Describes a cue in the State column, in the color of what it needs.  A
/// file that is still loading stops the cue from playing, but it needs
/// nothing but time.  The list of running cues tells more of each, such as
/// where a paused cue stopped.  A cue that a fade or Panic is fading reads
/// fading, since the time left of its phase says nothing of that.  A cue that
/// sends only to outputs that are gone reads in yellow that it has none,
/// which GO does not skip, and a light cue that sets no instrument or names
/// one that the patch lacks reads so in yellow, which GO does not skip
/// either.  Red turns lighter where the cue is `selected`, so
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
            Problem::NoTarget => ("NO TARGET".to_owned(), danger),
            Problem::WrongTarget => ("BAD TARGET".to_owned(), danger),
            Problem::InTarget => ("IN TARGET".to_owned(), danger),
            Problem::NotLooping => ("NO LOOP".to_owned(), danger),
            Problem::NoDestination => ("NO DESTINATION".to_owned(), danger),
            Problem::BadPort => ("BAD PORT".to_owned(), danger),
            Problem::NoMessage | Problem::OpenQuote => ("NO MESSAGE".to_owned(), danger),
            Problem::LoadFailed(_)
            | Problem::EmptyClip
            | Problem::PastFile
            | Problem::EmptyLoop => ("BROKEN".to_owned(), danger),
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
            None => match workspace.unlit(cue) {
                Some(Unlit::Empty) => ("NO LEVELS".to_owned(), text::warning),
                Some(Unlit::Unpatched(_)) => ("UNPATCHED".to_owned(), text::warning),
                None => (String::new(), text::default),
            },
        },
    }
}

/// Returns the lines of the State column of the cue `cue` as the cue list
/// shows them, which are [`lines`] and, below them, the runs that the
/// collapsed group `cue` hides
pub(super) fn shown_lines(
    workspace: &Workspace,
    groups: &Groups,
    cue: &Cue,
    next: bool,
    failing: &[OutputId],
    unreached: &BTreeSet<String>,
    selected: bool,
) -> Vec<Line> {
    let hidden = hidden_problem(workspace, groups, cue);
    lines(workspace, cue, next, hidden, failing, unreached, selected)
        .into_iter()
        .chain(hidden_runs(workspace, groups, cue))
        .chain(hidden_warnings(workspace, groups, cue, failing, unreached))
        .collect()
}

/// Returns the yellow warnings that the cues of the collapsed group `cue`
/// would tell, such as OUTPUT FAILED, each once, so that folding the group
/// hides neither a silent cue nor a light that sets nothing.  A cue with a
/// problem is told by the problem, and any other cue has none.
fn hidden_warnings(
    workspace: &Workspace,
    groups: &Groups,
    cue: &Cue,
    failing: &[OutputId],
    unreached: &BTreeSet<String>,
) -> Vec<Line> {
    const WARNINGS: [&str; 5] = [
        "OUTPUT FAILED",
        "SEND FAILED",
        "NO OUTPUT",
        "NO LEVELS",
        "UNPATCHED",
    ];
    let members = if cue.kind == Kind::Group && cue.collapsed {
        groups.members(cue.id)
    } else {
        &[]
    };
    members
        .iter()
        .filter(|member| member.kind != Kind::Group && workspace.problem(member).is_none())
        .flat_map(|member| lines(workspace, member, false, None, failing, unreached, false))
        .filter(|(told, _)| WARNINGS.contains(&told.as_str()))
        .unique_by(|(told, _)| told.clone())
        .collect()
}

/// Returns the lines that count the cues of the collapsed group `cue` that
/// run, the paused ones first in yellow, so that folding the group hides no
/// sound or picture that plays.  A group that runs itself tells its own state
/// instead, and any other cue has none.
fn hidden_runs(workspace: &Workspace, groups: &Groups, cue: &Cue) -> Vec<Line> {
    let collapsed = cue.kind == Kind::Group && cue.collapsed;
    let members = if collapsed && workspace.active_of(cue.id).is_none() {
        groups.members(cue.id)
    } else {
        &[]
    };
    let runs = members
        .iter()
        .filter(|member| member.kind != Kind::Group)
        .filter_map(|member| workspace.active_of(member.id))
        .map(|active| active.paused)
        .collect::<Vec<_>>();
    let paused = runs.iter().filter(|&&paused| paused).count();
    [
        (paused, "paused", text::warning as fn(&Theme) -> _),
        (runs.len() - paused, "running", text::success),
    ]
    .into_iter()
    .filter(|&(count, ..)| count > 0)
    .map(|(count, word, style)| (format!("{count} {word}"), style))
    .collect()
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

/// Returns how many cues the collapsed group `cue` hides, as ` (10 cues)`
/// for the end of its name, or none for any other cue.  A list that has no
/// room for the Target column, which tells it, shows the count there.
pub(super) fn counted(workspace: &Workspace, groups: &Groups, cue: &Cue) -> Option<String> {
    (cue.kind == Kind::Group && cue.collapsed)
        .then(|| format!(" ({})", Column::Target.text(workspace, groups, cue)))
}
