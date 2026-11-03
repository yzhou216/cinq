<!--
SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>

SPDX-License-Identifier: GPL-3.0-or-later
-->

# Demo shows for cinq

Shows to try cinq with, along with the sounds that they play, which Git LFS
keeps.  Install Git LFS first, as with `brew install git-lfs` or the
git-lfs package of your system, and run `git lfs install` once: without it,
the sounds arrive as small pointer files, and cinq calls every audio cue
broken.  Then, in a clone of cinq, check the shows out next to it and open
one:

```sh
git fetch origin demo
git worktree add ../cinq-demo demo
cinq ../cinq-demo/storm.cinq.toml
```

A checkout made before Git LFS was installed gets its sounds with
`git lfs pull` in `../cinq-demo`.

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
