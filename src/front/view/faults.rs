// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Faults of the outputs that cannot play, which the views of outputs tell

use crate::{
    back::{Fault, OutputId},
    front::App,
};

/// Returns the fault of the output of `id`, if it cannot play
pub(super) fn fault(app: &App, id: OutputId) -> Option<&Fault> {
    app.audio_problems
        .iter()
        .find(|(output, _)| *output == id)
        .map(|(_, fault)| fault)
}
