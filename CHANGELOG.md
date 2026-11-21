<!--
SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>

SPDX-License-Identifier: GPL-3.0-or-later
-->

# Changelog

Major changes and improvements in each release of cinq, newest first.

## Unreleased

- A level typed into a field, that of a cue, a send, the new cues of an
  output, or an instrument of a light cue, applies once the field is left, by
  Enter, Tab, a click, GO, or any other key or click.  A cue that plays no
  longer follows each key, as the `2` of `-25` would have made it 28 dB
  louder, and an emptied send no longer silences a cue before a level is typed
  back.  GO fires a cue at the level typed.  The slider still applies as it
  moves.
- Changing the level of an audio or video cue while it plays moves it by the
  change instead of setting the level, so that a cue that a fade cue has
  lowered stays lower, where a nudge of its slider from 0 to -0.5 used to make
  it jump to -0.5 dB.  Moving the level to the bottom and back restores the
  level of the fade, and the list of running cues never shows less than
  -60 dB.  A fade under way that leaves the cue playing goes on, from where the
  change moved the level, instead of ending.
- Changing the level of a fade cue while it fades now sends the fade to the
  new level in the time it has left, and a fade cue that has yet to act fades
  to the new level, where the change used to wait for the next run of the
  fade cue.  Its duration, target, and stop setting still wait for the next
  run.
- Start, pause, and GoTo cues act on other cues, as those of QLab do, and
  take no time.  A start cue fires its target together with the cues that
  chain to it, as GO does, or resumes it where it is paused.  A pause cue
  pauses its target while it runs, wherever it is, together with the cues
  that wait for it to start them.  A GoTo cue moves the standby cue of the
  list of its target to the target, so that the next GO of that list fires
  it.  Each targets any cue but itself, in any list, chosen on the Basics tab
  of the inspector, and a new one targets the cue above it.  The sounds and
  pictures that a start cue fires are made ready ahead, as those of GO are,
  and a picture or video cue that it fires brings up the stage window.  The
  chapter of the tutorial on sequences covers pause and start cues.  cinq
  0.1.5 and older refuse a show with start, pause, or GoTo cues.
- An audio or video cue that loops can loop a section of its file, so that one
  file holds an introduction, a vamp, and an ending: the cue plays from its
  start to its loop end, then goes back to its loop start each time it reaches
  its loop end.  The Loop start and Loop end fields of the Time & Loops tab
  set them, blank for the start and the end of the cue, which loops the whole
  clip as before, and they are editable while the cue loops.  A loop that
  reaches outside the cue counts as the part of it that lies within, and a loop
  that starts at or after its end cannot play: the cue reads BROKEN and GO
  skips it.  The show file keeps `loop_start` and `loop_end` of a cue that
  sets them, looping or not.  cinq 0.1.5 and older ignore `loop_start` and
  `loop_end`, and loop the whole clip.
- Devamp cues, named as in QLab, let an audio or video cue that loops leave its
  loop: the run of the target plays on to the end of a pass through its loop,
  then from the loop end to the end of its cue, and ends there.  It leaves at
  the first end of a pass that lies more than half a second ahead, which the
  sound needs to leave its loop cleanly, so that a loop shorter than about half
  a second can go round once more than planned.  A devamp cue takes no time,
  and its target is any audio or video cue, in any list, chosen on the Basics
  tab.  A target that does not loop reads NO LOOP, and GO skips the cue.  A run
  that a devamp cue has let go on shows the time it has left, and fades, stop
  cues, pause cues, and panics act on it as on any run.  The chapter of the
  tutorial on sequences covers loop sections and devamp cues.  cinq 0.1.5 and
  older refuse a show with devamp cues.
- Group cues, which act as the timeline group of QLab does, hold the cues
  below them that lie one level deeper, and one GO on a group fires all of
  them at once, each after its own pre-wait and those of the groups that hold
  it.  Standby moves past the group, and the group goes on by its own Then
  setting, to the cue below it: auto-follow once the last of its cues ends.
  The Then setting of a cue inside a group has no effect, and the inspector
  shows it disabled.  A group shows as running while any cue in it runs, with
  the time left while the group has an end.  Stop, pause, start, and GoTo
  cues, and the Stop button, act on a group and on every cue in it, while fade
  and devamp cues cannot name one, and a start cue inside the group that it
  starts cannot play.  A new cue below a group goes into it, and takes no
  target from the group.  Pasted cues keep their shape, a group moves, is cut,
  copied, and deleted with the cues it holds, and undo takes each back in one
  step.  `>>` and `<<` in the vi keymap, and `Control+]` and `Control+[` in the
  cua keymap, move a cue into the group above it and out of its group, and take
  counts and motions as `d` and `y` do.  The show file keeps each cue's
  `depth`.  cinq 0.1.5 and older refuse a show with group cues and ignore
  `depth`.
- A group collapses in the cue list, as a fold closes in Vim: the list shows
  it as one row, with a `+` before its name where an open group has a `-`, and
  leaves out the cues that it holds.  `z c`, `z o`, and `z a` collapse, expand,
  and toggle the selected group in the vi keymap, `z M` and `z R` act on every
  group, `ArrowLeft` and `ArrowRight` collapse and expand in the cua keymap,
  and a click on the mark flips a group.  A collapsed group still fires, stops,
  and pauses its cues, and its row reads NEXT while standby lies among them,
  and the problem of the first of them that has one, such as NO FILE.  Every
  key and count that moves through rows counts a collapsed group as one row,
  and a search, a jump, undo, or GO that selects a hidden cue expands the
  groups that hide it, while those that the search line opens to preview a
  match close again when the match moves on or the search is cancelled.  A
  list switched to selects the collapsed group that hides its standby cue.
  Collapsing is no change for undo, show mode allows it, and the show file
  keeps it as `collapsed = true` on the group.
- The transport ends the name and the notes of the standby cue in an ellipsis
  where they run out of room, where they were cut without a sign, and the Show
  mode checkbox moves to the toolbar, so that the notes take the whole width
  under the title: a long note shows 89 characters instead of 56 at 1024x768.
- A standby cue that has a problem reads NEXT and the problem on lines of their
  own in the State column, where the problem used to hide NEXT, and the column
  holds NO DESTINATION on one line.  The column reads `fading` for a cue that a
  fade or Panic is fading, and OUTPUT FAILED in yellow, under its state, for a
  cue that sends only to outputs whose devices failed, which the line under the
  standby cue tells as silent, as it tells a cue that sends to no output.  The
  cue list leaves out a Pre, Post, Then, or Target column that no cue fills,
  lets other columns give way while a name lacks the room that its names need,
  and lets the Target column grow into the room that the names leave, so that
  a long file name wraps as late as it can.
- The sidebar keeps each cue list to one line, cutting a long name short in an
  ellipsis and keeping the number of its cues, and scrolls to the list that
  shows whenever it changes, and back to it as Tab moves on.  A running cue
  keeps its name and its status to a line each, around its bar, with the time
  left and the level first, so that a status cut short loses the words of its
  phase before them.  The sidebar grows to 560 pixels on a wide window, and an
  empty Active or Warnings list leaves out the gap of its empty body.
- The status bar hints a key only where it acts: while a text field has the
  keyboard it no longer offers undo, redo, paste, add, rename, or help, which
  would only type there, and while a closed drop-down list has it, only undo
  and redo of these.  Where the keymap binds no key to cancel, as the cua
  keymap does, it hints Enter to leave a field.  It hints the key of standby
  while the selected cue is not the standby cue, tells no notice while a menu
  is open, so that the keys that pick and close it show, and in the outputs
  view tells changes to the outputs only, so that the keys of that view show.
  The rename of a cue just added outlasts the hints of GO, Panic, and saving
  at 1024x768, the key of the next inspector tab is hinted last of all, and a
  gap keeps the status apart from the hints.  The status counts warnings only
  while there are any, which leaves the hints the room of `0 warnings`.  A key
  that edits in show mode, or while the stage window has the keyboard, now
  says once why it did nothing.
- The inspector tells in words what the color of a field told alone: its header
  says why GO skips the selected cue, such as a missing file or no target, and
  which time reads as no time, with the forms that a time takes, and a refused
  level, send, light level, or address gives its reason beside the field or
  above or under its table.  A field that the cue ignores says why, as
  "(in a group)", "(with auto-follow)", or "(not looping)", and a value that
  cannot change draws on the black of the window, where it reads at 7:1, in a
  box of the height of a field that shows the dim placeholder where the value
  is empty, so that the inspector keeps its height as GO moves the selection.
  Tab no longer stops on a field that cannot change, and it reaches the Loop
  and Stop target checkboxes, which Enter flips and show mode locks.  The sends
  of the Levels tab lie side by side in rows that wrap, so that most of them
  show at once, each with as much of its output name as names need, up to 32
  columns, keeping the end of a longer one.  The inspector grows to a quarter
  of a window taller than 960 pixels, the Basics tab gives the larger share to
  the name and the file, the destination of a network cue has a row of its
  own, a kind with no target has no Target row, and the Light Patch tells that
  it has no instruments, and what an address takes only while an address has
  the keyboard or is refused.
- Tab reaches the Show mode checkbox, which show mode never locks, and Enter
  flips it and gives the keys back to the cue list, so that the keyboard alone
  locks the show before a performance and unlocks it after.  On the Light
  Patch, Tab reaches Add instrument and each Remove, which Enter presses and
  show mode locks.  Tab past the last field, and Shift+Tab before the first,
  leave every field and give the keys back to the cue list, or the outputs, so
  that a checkbox or a closed list no longer keeps them in the cua keymap,
  which has no key to leave a field.  F11 out of fullscreen gives the main
  window the keyboard.
- The status bar hints `Enter: flip` while a checkbox has the keyboard,
  `Enter: open` while a closed drop-down list does, and `Enter: press` while a
  button does, and the key that shows the next cue list, or the outputs after
  the last one, as `l: next list` in vi.  At the end of a list that another
  follows, where GO fires nothing, that hint stays on a short bar until after
  GO and Panic.  While a field has the keyboard, GO, Panic, and the key that
  leaves it outlast the keys that only teach, and while a text field has it,
  the bar hints a key of GO that types nothing there, or none, since Space
  types a space.  The rename of a cue just added outlasts the help hint too.
- The first paragraph of the help page names the key that closes it, the key of
  standby opens the tutorial from the page, and the table of keys takes the
  width of the window up to one and a half times that of the paragraphs, so
  that most descriptions fit one line, while the paragraphs keep their width.
- The outputs view hands the room that the device does not need to the name
  and the state, so that they wrap later in a narrow window, and its notices
  say that a cue sends to no output, as the State column does.
- A path typed into the File field of an audio, video, or picture cue names the
  cue after the file, as the file chooser does, while the cue goes by the name
  it was added with or by the name of its previous file.  The name follows the
  path as it is typed, and a name that was typed stays.  While a field has the
  keyboard, the key that leaves it outlasts a long notice in the status bar.
- A narrow main window, as one that a tiling compositor sets beside the stage
  window on one display, keeps every control and value readable: the toolbar
  wraps onto a second line, the buttons of the transport go under the title,
  the two forms of the inspector stack and its header wraps, the inspector of
  an output scrolls, the sidebar keeps 160 pixels, and each cell of the cue
  list keeps within its column, breaking a word that does not fit, while the
  State column keeps its words whole and the Action column gives way last.
  The outputs view lets State and then Channels give way, the Light Patch its
  Now column, and the help page the names in the configuration file, and an
  open menu stays within the window, shortening its rows.  A column that gave
  way returns only while every name keeps the room it needs, up to 48 columns,
  so that a wider window never wraps a name that a narrower one did not.  A
  notice too long for the status bar ends in an ellipsis, and the search line
  keeps a gap before its note.

## v0.1.5

- `back` and `forward` go back and forward between the places that jumps left,
  as the jump list of Vim does: switching cue lists, `first`, `last`, selecting
  a cue in another cue list, searching, and showing the outputs and leaving
  them are jumps.  The keys are `Control+o` and `Control+i` in the vi keymap
  and `Alt+ArrowLeft` and `Alt+ArrowRight` in the cua keymap, and the status
  bar names them while there is a place to go to.  A jump after going back keeps
  the places ahead, as in Vim, going back or forward in a text field moves the
  keyboard to the cue list, and the back and forward buttons of the mouse do the
  same from the main window, unless the help page is open.
- The status bar says where the keys of back and forward go, as `Control+o:
  back to Main`, `back to cue 3`, or `back to the outputs`, so that a pass
  through an empty cue list on the way to the outputs is no surprise, and it
  cuts the name of a long cue list short.  While the outputs show, it names the
  keys that go back, add an output, and rename one, and the key that goes back
  is the last of them to give way to a narrow bar.
- In the vi keymap, the key of *open* is now `Control+l`, since `Control+o` is
  Vim's way back through the places jumped to, and Vim's `Control+l` only
  redraws the screen, which cinq never needs.  The cua keymap keeps
  `Control+o`.
- `Control+Home` and `Control+End` select the first and last cue as `gg` and
  `G` do, each with a count.  `+` and `-` select the next and the previous cue,
  as in Vim.
- A count typed before the keys of undo and redo turns that many changes, as
  `3u` does in Vim, and stops at the oldest or newest.  A count before
  `next_tab` shows the inspector tab of that number, as `2gt` does, and before
  `previous_tab` goes back that many tabs.
- A key that completes or continues no sequence of keys cancels what was typed
  so far and does nothing itself, as Vim beeps and drops the keys, so that a
  mistyped `dx` no longer cuts a cue, and `gj` and `gk` move the selection down
  and up in the vi keymap, as they do in Vim.  The keys of GO and Panic still
  act at once.
- `Control+c` cancels in the vi keymap, as in Vim, through the new `cancel`
  action, which has no key in the cua keymap, where `Control+c` copies.  It
  drops the keys typed so far, closes the search line with the selection going
  back to where the search began, takes the keyboard out of a text field, and
  closes an open drop-down list without picking, and never stops playback, so
  that no one reaches for `Escape`, which is Panic.  The status bar hints the
  key while keys wait or the search line is open, and a text field with a
  selection still copies it first.
- While the stage window has the keyboard, the keys that change the show or fill
  the clipboard, such as `x`, `Delete`, `o`, `u`, `p`, and the operators, do
  nothing, as the keys of editing a name, of searching, and of the help page
  already did, so that a key held down there cannot change cues out of sight.
  GO, Panic, standby, and the moves still act.
- The help page, the box of the keys that can follow, and the status bar write
  a sequence of keys that type characters joined, as Vim does: `dd`, `gg`,
  `gt`, and `zz`, no longer `d d`, since a newcomer pressed the Space between
  them, which is GO.  A sequence with a named key, such as `z Enter`, keeps its
  spaces, as does `config.toml`, and the help page says that the keys of a
  sequence are typed with no Space between them.
- The keys of undo and redo act while a closed drop-down list has the keyboard,
  as `Control+z` does in the cua keymap, where they did nothing and said
  nothing.  A text field still keeps them, since the text typed there is typed
  into it.
- The keys of Vim that move by a page work in the vi keymap: `Control+d` and
  `Control+u` scroll the cue list half a page and move the selection the same
  number of rows, so that it keeps its row on the screen, and `Control+f`,
  `Control+b`, `PageDown`, and `PageUp` scroll a whole page less two rows and
  select the first row in view going down, or the last going up.  A count before
  `Control+d` or `Control+u` sets how many rows they move, until the window
  changes size or another show opens, and a count before the whole pages counts
  them.  The same keys scroll the help page, and the cua keymap binds `PageDown`
  and `PageUp` to the whole pages.
- `Control+e` and `Control+y` scroll the cue list a row down and up in the vi
  keymap, as in Vim, or as many rows as a count, through the new `scroll_down`
  and `scroll_up` actions.  The selection stays on its cue unless it would leave
  the view, when it moves to the nearest row in view.  The keys of `pick` and
  `close_menu` act only in a menu, so a key may be bound to one of them and to a
  scroll.
- `H`, `M`, and `L` select the first, middle, and last cue row in view in the vi
  keymap, as in Vim, through the new `top_row`, `middle_row`, and `bottom_row`
  actions.  A count before `H` counts rows from the top and before `L` from the
  bottom.  They are jumps for `back` and, unlike the motions `j`, `k`, `gg`, and
  `G`, take no operator.
- `zz`, `zt`, and `zb`, and `z.`, `z Enter`, and `z-`, put the selected row in
  the middle, at the top, or at the bottom of the view in the vi keymap, as in
  Vim, through the new `row_to_middle`, `row_to_top`, and `row_to_bottom`
  actions.  A count selects that row first, as `{count}zt` does.  The selection
  stays on its cue, and the view stops at the ends of the list.
- A move or jump that leaves the selected cue far outside the view, such as `G`,
  `n`, or going back, scrolls the list to put it in the middle, with Vim's
  thresholds: seven rows above the view and 13 rows below it in a view of 23
  rows.  A move to a row near the view still scrolls by the least, as does GO
  moving the standby a row.
- Cues can be copied, cut, and pasted with a clipboard of cinq's own that
  outlasts the show it was filled from: `yy`, `Y`, `p`, and `P` in the vi keymap
  copy and paste, `dd`, `x`, and `Delete` cut, and `Control+c`, `Control+x`,
  `Control+v`, and `Control+V` do the same in the cua keymap.  The new `copy`,
  `cut`, `paste_below`, and `paste_above` actions carry these keys.  The cua
  `Delete` key still deletes without copying, and `delete` has no key in the vi
  keymap, where `cut = []` in `config.toml` now lets no key remove a cue, as
  `delete = []` did.
- A count copies or cuts that many cues and pastes that many times, a paste is
  one change that undo takes back, and a fade pasted together with its target
  targets the new copy, as in QLab.  The status bar tells what was cut or copied
  and names the key that pastes, and undo and redo still call a cut a cut.  A
  paste with nothing to paste says that the clipboard is empty, and the `copy`
  action works in show mode, as Vim yanks in a read-only buffer.
- In the vi keymap, `d` and `y` take a motion, as in Vim: `dj` cuts the selected
  cue and the next, `yk` copies it and the previous, and `dG`, `dgg`, and `d3G`
  reach the last cue, the first, and row 3.  Counts before the operator and the
  motion multiply, so that `2d3j` cuts seven cues, and the box above the status
  bar lists the motions after the operator.  The operator key is the doubled key
  of the binding, so that `cut = ["D D"]` makes `DD` and `Dj` cut, and a cut of
  several cues is one change for undo.
- A count before the keys of `add_below` and `add_above`, such as `3o`, adds
  that many cues in a row, at most a thousand, as one change for undo, and
  selects the last of them, as the cursor of Vim goes to the last line that `3o`
  inserts.
- Undoing the addition of a cue or a paste below a cue selects the cue that was
  selected when it was made, as the cursor of Vim goes back to where a change
  was made, in place of the cue that follows.
- A search line opens with `/` in the vi keymap and `Control+f` in the cua
  keymap, and `search_backward` opens one for a search up the list on a key
  that a keymap in `config.toml` gives it.  `?` and `F1` both show the help page
  in both keymaps, as an exception to Vim's `?`.  As the text is typed, the
  selection moves to the first cue after it that holds the text, in any case,
  in any column that the list shows except the waits and the state, so that
  `/rumble` finds the cue of `rumble.ogg`, as Vim's search matches a whole
  line.  While the outputs show, it searches their name, device, channels, and
  new cues, so that `/Dante` finds the outputs on that device.
- `Enter` accepts a search, and on an empty line repeats the last one.
  `Escape`, `Backspace` on an empty line, a click, going back or forward,
  moving to another field, and opening a show close the line.  `n` and `N`, or
  `F3` and `Shift+F3`, find the next match in the direction of the last search
  and in the other, a count goes round the matches at most once, and the status
  bar says when a step wrapped around or found none.
- Searches are jumps, so that `back` returns to where a search began or to the
  cue before a step to the next match.  The search line takes the keys of Vim's
  command line: `Control+w` deletes the word before the cursor, `Control+u`
  everything before it, and `ArrowUp` and `ArrowDown` recall the older and newer
  of the last fifty searches accepted that start with the text typed.  The keys
  act on the end of the text.
- The vi keymap enters the name of the selected cue, or of the output, as Vim
  enters text: `i`, `a`, and `A` put the caret at its end, `I` at its start, and
  `cc` and `S` select all of it, so that typing replaces it.  The cua keymap
  selects the whole name with `F2`, as desktop file managers do, and in both
  keymaps `Tab` and `Shift+Tab` select the text of the field that they reach,
  while a click puts the caret where it fell.  The new `edit_start` action has
  no key in the cua keymap, `change_name` has `F2`, and `c` is no operator, so
  only `cc` begins a change.
- `Tab` from the cue list or the outputs goes first to the first field of the
  inspector, so that the settings of the selected cue are one key away.  It then
  goes on through the rest of the inspector, the toolbar, and the sidebar, and
  round again, and `Shift+Tab` goes the other way.
- `Tab` onto a send on the Levels tab of a cue scrolls it into view, with the
  name of its output above it, where it stayed out of sight below or above the
  part of the sends that showed.  `Tab` onto a list of the sidebar does the
  same, with its title, where a warning in a low window pushed the Default
  device list out of sight.
- Enter in the empty File field of an audio, video, or picture cue opens the
  dialog that chooses the file, which Tab never reaches since it skips the
  Choose button, and the status bar hints `Enter: choose a file` while the
  empty field has the keyboard.
- Each visit to a field, such as the name of a cue or of an output, is one step
  of undo, where two visits in a row, each ended by leaving the field, undid as
  one.  A click, or a move to a field, ends a visit as well.
- In an open drop-down list in the vi keymap, `Control+y` picks the highlighted
  option and `Control+e` closes the list without picking, as in Vim's popup
  menu, and `F4` closes it in the cua keymap, through the new `pick` and
  `close_menu` actions.  Closing with these keys does not also panic, as
  `Escape` does, and after a choice or a close by key the keyboard stays on the
  list, so that `Tab` goes on to the next field.  `gg` and `G`, `Home` and
  `End`, or the keys of `first` and `last`, go to the first and last option, and
  `PageDown` and `PageUp` move by the options in view.
- While a drop-down list is open, the status bar hints the keys that pick and
  close it, as `Control+y: pick | Control+e: close` in the vi keymap and
  `Enter: pick | F4: close` in the cua keymap, and gives way when the bar is
  short.  While a text field or a drop-down list has the keyboard, it hints the
  keys of the next field and, in the vi keymap, of leaving it.
- The help page and the table of cinq(5) list the actions in groups, GO, Panic,
  and standby first, then the moves, then the edits, then the files and the
  window, in place of the order in which actions were added.
- The help page leaves out the actions that the keymap in use binds to no key,
  such as `delete` in the vi keymap, so that its table is shorter.
- The help page opens with a short paragraph of first steps, above the table of
  keys, that names the keys of GO and Panic, of moving the selection and of
  standby, and of adding, naming, and undoing, in the keymap in use, since the
  table begins with the keys of scrolling and a newcomer found `o` and `cc` only
  on its second page.
- The help page links to the tutorial below its first steps, and following the
  link opens the PDF in the program that the desktop opens it with.  cinq looks
  for it in the XDG data directories, such as `/usr/share`, where the packages
  for GNU/Linux put it, in `/usr/local/share` on Darwin, and beside `cinq.exe`
  on Windows, and says where it looked when it finds none.  Why a link failed to
  open shows below the page, in view however far the page has scrolled, where it
  showed near the end of the page.
- The GO button looks disabled, as a disabled button does, while no cue that GO
  would fire can play, and the line under the name of the standby cue says why
  in words, as `GO fires nothing: cue 1 (no file set)`.  The notes and that line
  give way at a word where they meet the controls, no longer in the middle of
  one.
- After a cue is added, the status bar hints the key that renames it, as `Added
  cue 1 | cc: rename | u: undo` in the vi keymap and `F2: rename` in the cua
  keymap, where only the key of undo followed, and the new cue kept the name
  that a newcomer then found no key for.
- The list of cue types beside Add cue carries the label `New cue:`, since it
  only sets the type of the next cue that Add cue makes, and a newcomer took it
  for the type of the selected cue.
- Sounds play through a mixer of cinq's own, which opens each audio device on a
  thread of its own, so that a sound can reach any channel of a device and a
  slow device, as a Dante Virtual Soundcard that restarts, holds up no window,
  clock, GO, or other device.  An output shows no warning until its device has
  answered, and reads not answering, in yellow, once five seconds have passed
  without an answer.  A device whose stream stops says so among the warnings,
  where the sound once moved to the default device.
- A show has outputs, the named places that its cues send their sound to, each
  on one channel or two of an audio device, such as the house speakers on the
  first two channels of a desk and the subwoofer on its third.  Each audio and
  video cue sends to the outputs it names, each at a level of its own, and
  moving an output to other channels or another device moves every cue that
  sends to it.  Show files hold them in `output` and in the `sends` of each
  cue, as cinq-show(5) describes.  cinq 0.1.4 and older open such a show
  without them, and drop them when they save it.
- A show without outputs plays on the first two channels of the default device.
  An output that cannot play says why among the warnings, and the cues play on
  the others.
- The Outputs entry below the cue lists in the sidebar shows the outputs of the
  show in place of the cue list, each with its device, its channels, and in a
  few words why it cannot play where it cannot: in yellow where cinq tries its
  device again by itself, and in red where the output must change.  The Device
  column keeps at least 20 columns, the others giving way to it.  The outputs on
  a device that fails share one warning in the sidebar, which shows them when
  pressed.
- The inspector edits the selected output, whose Device list reads not found,
  in yellow, while its device is not there.  A name or a level applies as it is
  typed, and channels once Enter, which leaves the field as in every other, is
  pressed or the field is left, since a device opens again for them.
- The Audio device list of the sidebar is now Default device, since it chooses
  the device of the outputs that name none, and its first option reads System
  default.
- The Levels tab of an audio or video cue sets the level at which the cue sends
  to each output of the show, and a field left empty sends nothing there.  The
  fields scroll, so that every output stays in reach.  The name of an output
  that cannot play says what is wrong in words, as `House L (not found)` or
  `Lobby (past its channels)`, in yellow, or in red where the output must
  change, beside how many of those that the cue sends to cannot.
- The tutorial has a chapter on outputs, in which the reader sends the sound of
  a cue to each speaker of the computer in turn, and its first chapter names the
  outputs and the default device among what the sidebar shows.
- A device that is not there, or that stops while it plays, as one unplugged or
  a Dante Virtual Soundcard that restarts, opens again on its own within two
  seconds of its return, and so does a device whose sample rate changes, as
  Dante Controller can change that of the soundcard.  The cues that play on it
  pick up there from where they are, at the new rate where it changed.
- While a device is away, the cues on it wait for it until it tells that it
  stopped, or until it has asked for no sound for three seconds, then go on by
  the clock of the show, so that the cues that follow them still fire on time.
  A device that has just opened may take up to ten seconds for its first call
  for sound, and the clocks of the cues on it wait at the start until then.
- A device that has fewer channels than an output names opens again once it
  offers them, and is looked at ever more seldom, up to every thirty seconds,
  until what it offers changes.  Changing an output opens again only the
  devices that the change touches, so that the sound on the others plays on.
- On GNU/Linux and the BSDs, cinq also plays on the device of a running JACK
  server, such as PipeWire's, as `jack:output`, whose channels each reach the
  playback port of their number.  PulseAudio spreads a stream over every channel
  of a device that has more, and the ALSA plugin of PipeWire takes the channels
  past the second for those of surround sound, both of which JACK leaves alone.
  Building on GNU/Linux needs JACK, unless the build leaves out the default
  features, and the device lists leave out the null device of ALSA, which takes
  sound and plays none.
- Audio devices that take samples only as integers of 32, 24, or 16 bits, such
  as the `hw` devices of most sound cards on ALSA, now play, where cinq played
  floating-point samples alone.  A mix past full scale clips there instead of
  wrapping into a crack.
- On Windows, the build that Nix makes also plays on ASIO drivers, such as that
  of the Dante Virtual Soundcard, which offers all of its channels as one device
  where its WDM mode offers them in pairs.  The ASIO SDK of Steinberg goes into
  that build under the GPL version 3.
- On Windows, the thread that plays sound on WASAPI devices, as the WDM mode of
  the Dante Virtual Soundcard offers, asks Windows to schedule it for audio, as
  audio programs do there, so that a busy machine is less likely to break up the
  sound.
- Going back from the outputs, or leaving them with the key of the previous cue
  list or its entry in the sidebar, shows the cue list that showed before them
  with the cue that was selected there and its inspector tab.  Where no jump
  recorded a place, as after opening a show, it shows the cue list that was
  showing, and any other cue list shows with its standby cue.
- The keys that move through the rows of the cue list, such as `j`, `k`, `gg`,
  and `G`, the keys of half and whole pages, the keys that scroll by a row or
  select a row in view, and the keys that place the selected row, do the same to
  the outputs while they show, and the key of the next cue list reaches them
  past the last one.  The keys that act on cues alone, such as standby, do
  nothing there, so that none of them changes a cue list out of sight.  The
  outputs show their titles above the rows, as the cue list does.
- The keys that edit cues edit outputs while they show: `o` and `O` add one, and
  a count adds that many, up to 64, `dd`, `x`, and the motions such as `dj` cut
  them, `yy` and `Y` copy them, `p` and `P` paste them, `Alt+j` and `Alt+k` move
  one, and the keys of the name enter it.  Outputs have a register of their own
  beside that of cues, and a show keeps at least one output.
- A cut output pasted back into the same show takes back its identity, so that
  the cues that sent to it send to it again, while a copy is a new output that
  no cue sends to, and a copy of `House R 2` is `House R 3`.  The status bar
  tells first where the cues that name no sends play when the first output
  changes, and names the output that moved or was pasted.  Undo and redo select
  the output that they changed, except that undoing an addition or a paste
  selects the output that was selected when it was made, and undoing a change to
  a cue shows the cue list, as a jump that going back returns from.
- A cue pasted into another show sends to the outputs of the same names there,
  and plays on the first output where that show has none of them.
- A cue that sends only to outputs that a cut or a delete took away plays on
  no output, which the status bar counts with the change, as in `1 cue now
  sends nowhere`.  The State of such a cue reads `NO OUTPUT` in yellow, and the
  sidebar lists it among the warnings, until undo or a paste brings the outputs
  back.  A cue that names no sends still plays on the first output.
- The box that lists the keys that can follow those typed, such as the ones
  after `d`, names outputs where the outputs show, as in "Cut the selected
  output and the next", and cues where a cue list shows.  The help page, which
  lists each action once, names both.
- The fields of an output in the inspector are as wide as what they hold, and a
  line of help beside each says what it takes and what it does.  The line turns
  red and tells why where a value is not applied, as for a name that another
  output has, channels that are neither one nor two, or a level out of range,
  and an output whose device is not there also tells what to do.  Where the
  keyboard leaves a field that holds a value that was not applied, the status
  bar tells why, as `Not applied: another output has this name`, since the field
  shows the old value again.
- The default option of the Device list of an output names the choice of the
  Default device list in the sidebar, as in `Default device (System default)`,
  and while the highlight of the open menu rests on that option, by the mouse or
  the keys, the Default device list in the sidebar turns the blue of the
  highlight, which shows where the default is set.  An output on the Default
  device whose device is not there keeps its name in its list, as in `Default
  device (Studio), not found`, and its help says to connect it or to set the
  default device in the sidebar.
- The sidebar lists the warnings above the Default device list, which a show
  sets once, and at most three of them, then a line such as `and 5 more`, so
  that at 1024x768 the whole button of a warning shows and the Default device
  list stays in reach without a scrollbar.  The warnings count in the title and
  in the status bar is still the whole count.
- The outputs table drops its New cues column once the Device column cannot
  show its widest name on one line, as at 1024x768 beside a Dante Virtual
  Soundcard, and keeps the column in a wider window, where it fits.  The
  inspector keeps the value as New cues send.
- A long name of the show gives way first in the status bar: it is cut short at
  its end with an ellipsis, down to 16 columns, before any key hint goes, so
  that the mode and the warning count stay, and the keys of GO and Panic as well
  while no notice shows.
- The border of a field or a list that has the keyboard is two pixels thick,
  in the same blue, where it was one, so that it stands out from the fields at
  rest at a glance.  A field that holds a rejected value keeps its red border,
  thick while it has the keyboard.
- Red text on the selected row of the outputs table and the cue table is a
  lighter red, with a contrast of 4.9 to 1 against the row where the red of the
  rest of the window had 3.9 to 1, below the 4.5 to 1 that text needs.
- The text of a drop-down list that lies across the lower edge of the sidebar,
  as the Default device list does at 1024 by 768 pixels with a notice in the
  status bar, no longer draws below the sidebar, over the inspector, where the
  software renderer let it spill.
- Light cues fade the instruments of a light patch to levels of their
  own.  An instrument keeps its level until a later light cue names it, as
  in QLab.  The show file holds the patch, which the inspector edits on its
  Light Patch tab.  cinq sends the levels as Art-Net and as sACN on every
  network that the computer is on.  Universes count from 1, as in sACN.
  Each instrument has one level, its intensity.  cinq 0.1.4 and older
  refuse a show with light cues, and drop the patch when they save one.
- On Darwin, `cinq.app` says why it reaches the local network, and it is
  signed under the identifier of its bundle, so that Allow takes effect.
- The tutorial has a chapter on lights, which runs the light cues of a new
  demo show, `lights.cinq.toml`.
- Network cues send an OSC message as they start, over TCP or UDP, as
  those of QLab do.  A GO can then fire a cue of a lighting console, or of
  any other device or program that takes OSC, alongside sound and video.
  The inspector edits the destination and the message of a network cue on
  its Basics tab.  The chapter of the tutorial on lights covers them too,
  with a cue that sends its message to a program that prints it.  cinq 0.1.4
  and older refuse a show with network cues.

## v0.1.4

- While the problems of `config.toml` take a screen of their own on a
  terminal, what cinq and its libraries print meanwhile, such as the
  messages of the sound system, waits and comes out on the main screen once
  the problems clear, or before the problems left when cinq ends, in place
  of being lost when the terminal switches back.  On Windows, only cinq's
  own messages wait.
- A drop-down list too narrow for its options cuts the chosen one short
  with an ellipsis, and its menu grows as wide as the longest option.
- `nix build .#demo` builds the demo shows with the sounds, picture, and
  film that they play, which `doc/demo/build.sh` generates with ffmpeg, in
  place of the `demo` branch and its files in Git LFS.
- A tutorial in eight chapters runs the storm show of the demo.
  `nix build .#doc` builds it into `share/doc/cinq/tutorial.pdf`, with
  screenshots that it takes of cinq in a headless sway session, and puts
  the demo beside it in `share/doc/cinq/demo`.  Systems other than
  GNU/Linux build it on a GNU/Linux builder, such as `nix.linux-builder` in
  nix-darwin.  The deb, rpm, Arch, and Darwin packages carry both.
- On GNU/Linux and on Apple silicon, `nix build .#darwin-pkg` builds an
  installer package for Darwin on Apple silicon, which puts `cinq.app` in
  `/Applications` and the `cinq` command, its manual pages, its
  completions, and the tutorial with its demo in `/usr/local`.  The
  package is unsigned.
- The clock of an audio cue follows how far its sound has played, as that
  of a video cue does, so that its fades, its end, and the cues that follow
  it stay with what is heard.  A sound that drifts from the clock of the
  show, or a device that waits before it plays, no longer leaves the cue to
  end before its sound and cut off its last moments.
- The sound of audio and video cues can play on another audio device than
  the default device of the system.  The Audio device list in the sidebar
  chooses one, and `config.toml` keeps the choice in `audio_device`, by an
  ID that `cinq --audio-devices` lists along with the name of each device.
  A choice applies at once, and the sounds that play go on from where they
  are on the new device once it starts.  The list asks for the devices each
  time it opens, so that a device plugged in since shows.

## v0.1.3

- On Windows, the unsaved changes of shows with the same path on different
  drives, or on different network shares, no longer share one kept file and
  take each other's place.  The drive or share now leads the kept file's
  path, so changes that an earlier version kept for a show on Windows do
  not come back.  Other systems keep their kept files as they were.
- cinq plays up to 1024 sounds at once, where it took the 129th for a
  broken file and kept checking that file again.
- Stopping a cue also cancels the cues of its chain that it would have
  started, even when a cue between them had been fired again or could not
  play.
- GO on a cue that is already running restarts it and cancels the cues that
  its earlier run had yet to start, as QLab does, in place of leaving them
  to play on.
- The help page fills the main window in place of a box over it, and closing
  it gives the window back as it was, with the cue list scrolled where it
  was and the keyboard in the same field.  A field of the main window no
  longer takes the keys typed while it shows.  Besides the keys of the page,
  only those of GO and Panic act.  The text stays in a column at most 960
  pixels wide.  The key of Help opens the page from the main window only,
  since a fullscreen stage window may cover the main window, and does
  nothing while the stage window has the keyboard.  The key of Help also
  opens the page while the menu of a drop-down list is open, and the menu is
  open on the same option when the page closes.
- Opening a show right after switching away from it, while cinq is still
  keeping its unsaved changes, brings back the newest of them and no longer
  older ones.
- A failure to keep unsaved changes now shows on a line of its own among the
  warnings, and no longer replaces the problem of opening or saving a show.
  It also takes back the status note that changes were kept.
- On the stage window, the pictures of earlier cues show around a picture
  of another shape than the window, as in QLab.  The space around it was
  black until the picture began to fade, when they appeared there at once.
- cinq makes ready the sounds that start within ten seconds, instead of those
  of every cue that one GO fires, so that a long chain of auto-continue cues no
  longer opens all its sound files at once.  `ready_ahead` in `config.toml`
  sets that number of seconds, 0 or more and 10 unless the file says
  otherwise, so that owners of small systems can open fewer files at once.
  A change applies as soon as the file is saved.  Sounds that start together
  are always made ready together.
- The clock of a video cue follows how far its sound has played, in place of
  the clock of the show, so that its picture and sound no longer drift apart
  over a long video, as they did by the difference between the clock of the
  audio device and that of the show.  A video without sound, or one whose
  sound has ended, still follows the clock of the show.
- A damaged video file now fails to load with a reason, where it could
  freeze or end cinq, and the files loaded after it load as they would.
- Damaged tags, chapters, or cover art in a video file no longer stop it
  from playing.  cinq reads no tags or cover art, and skips a damaged track
  other than the picture's, which it reports once on standard error.
- QuickTime `.mov` files with AAC sound, which cinq used to refuse, now
  load.

## v0.1.2

- On Windows, a terminal that starts cinq shows what cinq prints, such as
  the problems of `config.toml` and the output of `--help`, and waits until
  cinq closes.  Explorer still starts cinq without a console window.
- Video cues decode with rust_h264, a decoder written in Rust, in place of
  Cisco's openh264, so that building cinq compiles no C++, its program no
  longer needs the C++ library, and it builds on riscv64.  A sample that
  fails to decode now costs no picture but its own.
- On a terminal, the problems of `config.toml` take a screen of their own
  while any are left, as a pager's text does, so that fixing the file brings
  back the prompt and what came before it instead of a cleared screen.
  Problems still left when cinq ends stay on the terminal.
- The Nix packages for other systems go by the systems that they run on, as
  `.#aarch64-darwin`, `.#x86_64-windows`, and `.#aarch64-windows`, in place of
  `.#aarch64-darwin-cross`, `.#x86_64-windows-cross`, and
  `.#aarch64-windows-cross`.
- A picture fading out no longer jumps back to whole when a panic, or
  another fade that stops its cue, takes over; it fades on from where it
  was.
- Video 1280 pixels wide that is cropped below 720 lines, such as
  letterboxed high definition, shows in the colors of high definition.
- On Darwin, cinq follows the changes to `config.toml` in a directory
  reached through a symbolic link, such as one under `/tmp`.
- On FreeBSD and the other BSDs, cinq follows the changes that an editor
  writes into `config.toml` in place, and not only those that rename another
  file over it.
- A cue that GO fires right after a sound or video that could not play
  ended the last running cue no longer starts ahead by the time spent idle.
- Choosing again the file that a running audio or video cue plays, or
  typing its name with a space around it, leaves the cue playing.
- cinq formats `config.toml` each time it is saved, as taplo formats TOML by
  default, but never reorders keys or the items of arrays and inline tables.
  A file that is not TOML stays as it is.
- On GNU/Linux, Nix cross-builds cinq for FreeBSD, as `.#x86_64-freebsd` and
  `.#aarch64-freebsd`, and for GNU/Linux on riscv64, as `.#riscv64-linux`.

## v0.1.1

- Picture cues show a PNG or JPEG on the stage window until a fade, a stop
  cue, or Panic ends them.
- The stage window opens as a window, which the operator can move to the
  display of the audience, and each GO that fires a picture or video cue
  makes it fullscreen on that display.  The fullscreen key turns it back
  into a window until the next such GO.
- Problems in `config.toml` show on standard error each time that cinq
  reads the file, at startup and on each change, with the lines around each
  problem and its place marked, and the sidebar gives the line of each.  On
  a terminal, only the problems of the latest reading stay on screen, so
  that fixing the file clears it, and a file without problems prints
  nothing.
