<!--
SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>

SPDX-License-Identifier: GPL-3.0-or-later
-->

# Build matrix

Where each build of cinq comes from, as of master `c372af6` (2026-11-12).
Rows are what is built; columns are the machine that runs Nix.  The flake
offers its outputs on x86_64-linux, aarch64-linux, and aarch64-darwin, and
CI builds every one of them on every push and keeps it for download.

| Label | Meaning |
|---|---|
| **Artifact, runs** | CI builds it, uploads it, and runs it on its own system |
| **Artifact** | CI builds and uploads it |
| **Branch** | Built on a scratch branch only, not on master |
| **Blocked [n]** | Cannot be built there; see note n |
| none | Not offered |

## cinq for each system

| Built (rows) by (columns) | x86_64-linux | aarch64-linux | aarch64-darwin | riscv64-linux | Other Linux [5] | x86_64-freebsd |
|---|---|---|---|---|---|---|
| x86_64-linux | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | none | Blocked [2] |
| aarch64-linux | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | none | Blocked [2] |
| riscv64-linux | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | none | Blocked [2] |
| i686, armv6l, armv7l, powerpc64le Linux | none | none | none | Blocked [1] | none | Blocked [2] |
| x86_64-freebsd | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | none | Blocked [2] |
| aarch64-freebsd | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | none | Blocked [2] |
| riscv64-freebsd | Blocked [3] | Blocked [3] | none | Blocked [1] | none | Blocked [2] |
| aarch64-darwin | **Artifact, runs** | **Artifact, runs** | **Artifact, runs** | Blocked [1] | none | Blocked [2] |
| x86_64-darwin [6] | none | none | none | Blocked [1] | none | Blocked [2] |
| x86_64-windows | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | none | Blocked [2] |
| aarch64-windows | **Artifact, runs** | **Artifact, runs** | none | Blocked [1] | none | Blocked [2] |
| i686-windows, for ReactOS [7] | Branch | none | none | Blocked [1] | none | Blocked [2] |

## Packages and documentation

| Built (rows) by (columns) | x86_64-linux | aarch64-linux | aarch64-darwin | riscv64-linux | Other Linux [5] | x86_64-freebsd |
|---|---|---|---|---|---|---|
| deb, rpm, Arch, for the machine's own processor | **Artifact** | **Artifact** | none | Blocked [1] | none | Blocked [2] |
| Darwin `.pkg` (`.#darwin-pkg`) | **Artifact, runs** | **Artifact, runs** | **Artifact, runs**, without the tutorial [4] | Blocked [1] | none | Blocked [2] |
| Tutorial (`.#doc`) | **Artifact** | **Artifact** | none [4] | Blocked [1] | none | Blocked [2] |
| Demo shows (`.#demo`) | **Artifact** | **Artifact** | **Artifact** | Blocked [1] | none | Blocked [2] |

"Runs" means `cinq --version` on the system the build is for: on
ubuntu-latest, ubuntu-24.04-arm, RISE's ubuntu-24.04-riscv, macos-latest,
windows-latest, and windows-11-arm runners, and in FreeBSD 15.1 virtual
machines, the aarch64 one emulated.  Each Darwin `.pkg` is installed on
macos-latest first.  The native builds run their tests while they build.

## CI artifacts

| Workflow | Built on | Artifacts | Count |
|---|---|---|---|
| `native.yml` | x86_64-linux, aarch64-linux, aarch64-darwin | `cinq-<system>.tar` | 3 |
| `cross-linux.yml` | x86_64-linux, aarch64-linux | `cinq-<host>-built-on-<system>.tar` for the other two of x86_64, aarch64, and riscv64 GNU/Linux | 4 |
| `cross-freebsd.yml` | x86_64-linux, aarch64-linux | `cinq-{x86_64,aarch64}-freebsd-built-on-<system>.tar` | 4 |
| `cross-darwin.yml` | x86_64-linux, aarch64-linux | `cinq-aarch64-darwin-built-on-<system>.tar` | 2 |
| `cross-windows.yml` | x86_64-linux, aarch64-linux | `cinq-{x86_64,aarch64}-windows-built-on-<system>.exe` | 4 |
| `packages.yml` | x86_64-linux, aarch64-linux, aarch64-darwin | `cinq-<system>-packages`: the deb, rpm, and Arch packages, the Darwin `.pkg`, `tutorial.pdf`, and `demo/`, as far as the system builds them | 3 |

That makes 20 artifacts per push, which GitHub keeps for 90 days.

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
4. The tutorial needs sway, which runs on GNU/Linux alone, so a `.pkg` built
   on Darwin lacks it and its demo.
5. Other Linux: i686, armv6l, armv7l, and powerpc64le GNU/Linux.  No runner
   builds for them, so the flake no longer offers them.  On armv6l, armv7l,
   and powerpc64le the tutorial could not build anyway, since its
   dependencies need GHC.
6. x86_64-darwin: this Nixpkgs no longer lists the system.  A cross build
   like aarch64-darwin's is untried.
7. i686-windows: the ReactOS port on the scratch branch `reactos`, built as
   `.#i686-windows` on x86_64-linux and run on windows-latest.
