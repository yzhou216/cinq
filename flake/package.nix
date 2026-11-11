# SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
#
# SPDX-License-Identifier: GPL-3.0-or-later

{
  perSystem =
    {
      config,
      lib,
      pkgs,
      system,
      crate,
      sourceDateEpoch,
      installBundle,
      ...
    }:
    let
      demo = pkgs.callPackage ./demo.nix { inherit crate; };
      evaluates = drv: (builtins.tryEval drv.drvPath).success;
    in
    {
      packages = {
        default = config.packages.cinq;
        # Named after the system that it runs on, as the builds for other
        # systems are, so that each system's name gives cinq for it
        ${system} = config.packages.cinq;

        cinq = pkgs.callPackage ./cinq.nix { inherit crate sourceDateEpoch installBundle; };
      }
      # Demo wherever ffmpeg evaluates, which it does not on FreeBSD
      // lib.optionalAttrs (evaluates demo) { inherit demo; };
    };
}
