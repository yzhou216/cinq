<!--
SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>

SPDX-License-Identifier: GPL-3.0-or-later
-->

# Build matrix

Where each build of cinq comes from, as of master `17eb477` (2026-11-12).
Rows are what is built; columns are the machine that runs Nix.  The flake
offers its outputs, native and cross, on every system that Nixpkgs lists.
CI verifies x86_64-linux, aarch64-linux, and aarch64-darwin, builds every
output there on every push, and keeps each for download.  Everything else
that the flake offers is untested, and a build that fails there should come
back as a report.

| Label | Meaning |
|---|---|
| **Artifact, runs** | CI builds it, uploads it, and runs it on its own system |
| **Artifact** | CI builds and uploads it |
| Flake | The flake offers it; CI does not build it, so reports are welcome |
| Branch | Built on a scratch branch only, not on master |
| Blocked [n] | The flake offers it, but it cannot be built there yet; see note n |
| none | Not offered |

## cinq for each system

| Built (rows) by (columns) | x86_64-linux | aarch64-linux | aarch64-darwin | riscv64-linux | Other Linux [5] | x86_64-freebsd |
|---|---|---|---|---|---|---|
| x86_64-linux | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | Flake | none |
| aarch64-linux | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | Flake | none |
| riscv64-linux | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | Flake | none |
| i686, armv6l, armv7l, powerpc64le Linux | none | none | none | none | Flake, natively | none |
| x86_64-freebsd | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | Flake | Blocked [2] |
| aarch64-freebsd | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | Flake | none |
| riscv64-freebsd | Blocked [3] | Blocked [3] | none | none | none | none |
| aarch64-darwin | **Artifact, runs** | **Artifact, runs** | **Artifact, runs** | Blocked [1] | Flake | none |
| x86_64-darwin [6] | none | none | none | none | none | none |
| x86_64-windows | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | Flake | none |
| aarch64-windows | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | Flake | none |
| i686-windows, for ReactOS [7] | Branch | none | none | none | none | none |

Cross builds come from GNU/Linux machines, and FreeBSD offers only its
native build.

## Packages and documentation

| Built (rows) by (columns) | x86_64-linux | aarch64-linux | aarch64-darwin | riscv64-linux | Other Linux [5] | x86_64-freebsd |
|---|---|---|---|---|---|---|
| deb, rpm, Arch, for the machine's own processor | **Artifact** | **Artifact** | none | Blocked [1] | Flake | none |
| Darwin `.pkg` (`.#darwin-pkg`) | **Artifact, runs** | **Artifact, runs** | **Artifact, runs** [4] | Blocked [1] | Flake | none |
| Tutorial (`.#doc`) | **Artifact** | **Artifact** | **Artifact**, aarch64-linux's [4] | Blocked [1] | Flake [5] | Flake, x86_64-linux's [4] |
| Demo shows (`.#demo`) | **Artifact** | **Artifact** | **Artifact** | Blocked [1] | Flake | none [8] |

"Runs" means `cinq --version` on the system the build is for: on
ubuntu-latest, ubuntu-24.04-arm, RISE's ubuntu-24.04-riscv, macos-latest,
windows-latest, and windows-11-arm runners, and in FreeBSD 15.1 virtual
machines, the aarch64 one emulated.  Each Darwin `.pkg` is installed on a
fresh macos-latest runner first.  The native builds run their tests while
they build.

## CI artifacts

| Workflow, as the Actions tab names it | Built on | Artifacts | Count |
|---|---|---|---|
| Native builds (`native.yml`) | x86_64-linux, aarch64-linux, aarch64-darwin | `cinq-<system>.tar` | 3 |
| Cross builds for GNU/Linux (`cross-linux.yml`) | x86_64-linux, aarch64-linux | `cinq-<host>-built-on-<system>.tar` for the other two of x86_64, aarch64, and riscv64 GNU/Linux | 4 |
| Cross builds for FreeBSD (`cross-freebsd.yml`) | x86_64-linux, aarch64-linux | `cinq-{x86_64,aarch64}-freebsd-built-on-<system>.tar` | 4 |
| Cross builds for Darwin (`cross-darwin.yml`) | x86_64-linux, aarch64-linux | `cinq-aarch64-darwin-built-on-<system>.tar` | 2 |
| Cross builds for Windows (`cross-windows.yml`) | x86_64-linux, aarch64-linux | `cinq-{x86_64,aarch64}-windows-built-on-<system>.exe` | 4 |
| Packages (`packages.yml`) | x86_64-linux, aarch64-linux, aarch64-darwin | `cinq-<system>-packages`: the deb, rpm, and Arch packages, the Darwin `.pkg`, `tutorial.pdf`, and `demo/`, as far as the system builds them | 3 |

That makes 20 artifacts per push, which GitHub keeps for 90 days.
`cinq-aarch64-darwin.tar` is Nixpkgs' native build, which links libiconv
from `/nix/store` and so runs only on a Mac with that path; the Darwin
builds cross-built on GNU/Linux and the `.pkg` run on any Mac.

## Notes

1. riscv64-linux: Nix cannot start there, since patchelf put the bootstrap
   gcc's first segment below `mmap_min_addr` (NixOS/nixpkgs#514892).  Cargo
   builds cinq natively there outside Nix.
2. x86_64-freebsd: Nixpkgs' FreeBSD bootstrap does not finish natively yet
   (NixOS/nixpkgs#584846, then later blockers).  The cross builds from
   GNU/Linux need the flake's two workarounds, for NixOS/nixpkgs#584780 and
   NixOS/nixpkgs#584781.
3. riscv64-freebsd: Nixpkgs refuses the platform, and no FreeBSD package
   repository has Rust for it.
4. The tutorial takes its screenshots in sway, which runs on GNU/Linux
   alone.  Elsewhere `.#doc` is that of GNU/Linux on the same processor,
   which Nix builds on a GNU/Linux builder, such as the QEMU virtual machine
   of `nix.linux-builder` in nix-darwin, and every package carries it.
   GitHub's Mac runners cannot start that virtual machine, so CI's Darwin
   job waits for the aarch64-linux job, which pushes the tutorial to the
   Cachix cache `yzhou216`, and takes it from there.  Pull requests from
   forks get no Cachix token, so their GNU/Linux package jobs cannot push it
   and fail.
5. Other Linux: i686, armv6l, armv7l, and powerpc64le GNU/Linux.  No runner
   builds for them, so they stay untested.  The tutorial does not evaluate
   on armv6l, armv7l, or powerpc64le, since its dependencies need GHC, so
   those take that of x86_64-linux through a builder, as in note 4.
6. x86_64-darwin: this Nixpkgs no longer lists the system.  A cross build
   like aarch64-darwin's is untried.
7. i686-windows: the ReactOS port on the scratch branch `reactos`, built as
   `.#i686-windows` on x86_64-linux and run on windows-latest.
8. The demo takes ffmpeg, which does not evaluate on FreeBSD.
