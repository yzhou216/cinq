<!--
SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>

SPDX-License-Identifier: GPL-3.0-or-later
-->

# Demo shows for cinq

Shows to try cinq with, along with the sounds that they play, which Git LFS
keeps.  Check them out next to the program, as with
`git worktree add ../cinq-demo demo`, and open one, as with
`cinq ../cinq-demo/storm.cinq.toml`.

- `storm.cinq.toml`: a scene in one list, from the preshow music through a
  storm, with fades that bring a cue up and take it out, a clip with a start
  and an end, and an auto-follow.
- `chains.cinq.toml`: knocks that one GO fires, chained by auto-continue with
  post-waits and by auto-follow, then a wait, and a stop cue with a pre-wait.
- `lists.cinq.toml`: a main list and a list of effects, each with a standby
  of its own.

The sounds, in WAV, FLAC, Ogg Vorbis, and MP3, are generated rather than
recorded.

## License

cinq is free software: you can redistribute it and/or modify it under the terms
of the GNU General Public License as published by the Free Software Foundation,
either version 3 of the License, or any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
