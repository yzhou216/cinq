// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Messages that the update applies: the operator's intent, which the view
//! and the keys send, and the reports of the clock, of the configuration
//! file's watch, and of the work done on threads of its own

mod scrolling;
mod tab;

use std::{ffi::OsString, path::PathBuf, time::Instant};

use iced::{Size, mouse, window};

use super::{
    keymap::{Press, config::Config},
    update::{Measured, Read, Saved},
};
use crate::back::{Command, CueId, Edit, Kind, Place};
pub use scrolling::{Height, Page};
pub use tab::Tab;

/// Text field that keys move the keyboard focus to, since iced itself
/// moves it only on clicks
#[derive(Debug, Clone)]
pub enum Focus {
    /// Name field of the inspector, for editing the selected cue
    Name,
    /// Next field, or the first one when no field has focus
    Next,
    Previous,
    /// No field, which hands the keys back to the cue list
    Cues,
}

/// Operator intent or a report of work done, which becomes a back-end
/// command or an interface change
#[derive(Debug, Clone)]
pub enum Message {
    Go,
    Panic,
    /// Drops the keys typed so far and takes the keyboard out of a field,
    /// without touching playback
    Cancel,
    PauseAll,
    ResumeAll,
    Stop(CueId),
    /// Time of a tick of the playback clock, or of the moment that the
    /// next picture of a video cue comes due
    Tick(Instant),
    Select(CueId),
    /// Moves the selection by a number of cues, downwards when positive
    Step(isize),
    /// Selects the cue on a row counted from one, or the last cue for a row
    /// past the end
    Row(usize),
    /// Scrolls the cue list by a page and moves the selection with it, with
    /// the count typed before the keys, which sets the rows of a half page and
    /// counts the whole pages
    Page(Page, Option<usize>),
    /// Selects the row, counted from zero, where a page scrolled the cue list
    /// to, which has scrolled already, unless the list or the selection that
    /// the key found has changed since, as a key pressed in between would
    /// have changed them
    Landed {
        list: usize,
        from: Option<CueId>,
        row: usize,
    },
    /// What a measure of the cue list or of the help page found
    Measured(Measured),
    /// The mouse may have scrolled the cue list, so that it is measured again
    Remeasure,
    /// A window has drawn, which measures the cue list when no measure of it
    /// is kept, as the window opens and after a change that made it out of
    /// date, since the list is laid out only once the window has drawn
    Drawn(window::Id),
    /// Scrolls the cue list a number of rows, down when positive, and keeps
    /// the selection in view
    Scroll(isize),
    /// Selects the row at a height in view, counted from the top or the bottom
    /// with the count typed before the keys, as a jump
    InView(Height, Option<usize>),
    /// Selects the row, counted from zero, that a measure of the cue list
    /// found, as a jump, unless the list or the selection that the key found
    /// has changed since
    Reached {
        list: usize,
        from: Option<CueId>,
        row: usize,
    },
    /// Scrolls the cue list to put the selected row at a height in view, after
    /// selecting the row that the count typed before the keys names
    Place(Height, Option<usize>),
    StandbySelected,
    SwitchList(usize),
    /// Shows the cue list a number of lists further down the sidebar
    StepList(isize),
    /// Goes a number of places back, among those that jumps have left
    Back(usize),
    /// Goes a number of places forward again after going back
    Forward(usize),
    ShowMode(bool),
    Tab(Tab),
    /// Shows the inspector tab a number of tabs to the right, wrapping
    /// around at either end
    StepTab(isize),
    /// Shows the inspector tab with a number, counted from one, or the last
    /// tab when there are fewer
    ShowTab(usize),
    NewKind(Kind),
    /// Adds a cue next to the selection, on the side that the function
    /// picks
    Add(fn(CueId) -> Place),
    /// Moves the selected cue by a number of places, downwards when
    /// positive
    Move(isize),
    /// Deletes a number of cues from the selection down
    Delete(usize),
    Edit(CueId, Edit),
    LevelTyped(CueId, String),
    /// Asks which file an audio, video, or picture cue plays or shows
    ChooseFile(CueId),
    FileChosen(CueId, PathBuf),
    /// Undoes a number of changes, stopping at the oldest
    Undo(usize),
    /// Redoes a number of changes, stopping at the newest
    Redo(usize),
    Focus(Focus),
    ToggleHelp,
    CloseHelp,
    /// Key that no widget kept, whether typed outside any text field or one
    /// that acts in fields, or a key of GO or Panic that a field took
    /// without typing it, and whether it is a repeat of a key held down
    Typed {
        press: Press,
        repeat: bool,
    },
    /// What loading the files of audio, video, and picture cues found, for
    /// the show of an identity
    Loaded(u64, Vec<Command>),
    /// Time to load again the files that could not be played
    Recheck,
    /// What loading again the files that could not be played found, for
    /// the show of an identity
    Rechecked(u64, Vec<Command>),
    /// Configuration as its file holds it, read once the watch on the file
    /// is in place and again after each change to it
    Reconfigured(Config),
    /// Opens a web page or a file in the program that the desktop opens it
    /// with
    OpenLink(OsString),
    /// Opens the configuration file in the program that the desktop opens
    /// it with, making it first when it is not there yet
    EditConfig(PathBuf),
    LinkOpened(Result<(), String>),
    /// Asks for the audio devices afresh, as their list opens
    ListAudioDevices,
    /// ID and name of each audio device that sound can play on, or why they
    /// could not be listed
    AudioDevicesListed(Result<Vec<(String, String)>, String>),
    /// Plays the sound on the audio device of an ID, or on the default
    /// device without one, and keeps the choice in the configuration file
    ChooseAudioDevice(Option<String>),
    /// Whether the configuration file kept the audio device chosen last
    AudioDeviceKept(Result<(), String>),
    /// Asks which show to open
    Open,
    /// Show read from its file, with the unsaved changes kept for it
    Opened(Box<Read>),
    /// Saves the show in its file, or asks where to save a show that has
    /// none
    Save,
    /// Asks where to save the show, and saves it there
    SaveAs,
    /// Saves the show in a file that the operator chose
    SaveTo(PathBuf),
    Saved(Saved),
    /// Mouse button pressed anywhere in a window
    Clicked,
    /// Back or forward button of the mouse pressed in a window
    SidePressed(window::Id, mouse::Button),
    /// Window changed size, to no size at all while it is minimized
    Resized(window::Id, Size),
    WindowFocused(window::Id),
    /// Toggles the stage window between fullscreen and a window
    ToggleFullscreen,
    /// Window closed, as the operator closes it
    Closed(window::Id),
}
