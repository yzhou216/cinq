// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the notices, undo and redo, and the keys that enter the name, for outputs

use super::{ALL, clipped, locked, names, on, typed, without};
use crate::{
    back::{PatchEdit, Place},
    front::{
        App, Caret, Focus, Message, Setting, Tab,
        keymap::Action,
        update::tests::{
            after,
            history::{cue, place},
            noticed,
            output_rows::at,
            paging::letter,
            press, show, told,
        },
    },
};

#[test]
fn a_cut_of_the_first_output_tells_where_cues_with_no_sends_play_now() {
    let cut = typed(on(1), "dd");
    assert_eq!(
        clipped(&cut),
        Some(("Cues without sends play on House R, cut House L", true))
    );
    // Not when the first output stays or no cue is left without sends
    let kept = typed(on(2), "dd");
    assert_eq!(clipped(&kept), Some(("Cut the output House R", true)));
}

#[test]
fn a_change_of_the_first_output_tells_it_in_the_notice_of_the_change() {
    let moved = after(on(1), [Message::Move(1)]);
    assert_eq!(
        noticed(&moved),
        "Cues without sends play on House R, moved House L"
    );
    let added = after(on(1), [Message::Add(Place::Before, 1)]);
    assert_eq!(
        noticed(&added),
        "Cues without sends play on Output 1, added Output 1"
    );
    let deleted = after(on(1), [Message::Delete(1)]);
    assert_eq!(
        noticed(&deleted),
        "Cues without sends play on House R, deleted House L"
    );
    let calm = after(on(3), [Message::Move(1)]);
    assert_eq!(noticed(&calm), "Moved the output Sub");
    let counted = after(on(3), [Message::Add(Place::After, 3)]);
    assert_eq!(noticed(&counted), "Added 3 outputs");
}

#[test]
fn undo_and_redo_select_the_output_that_the_change_touched() {
    let cut = typed(on(3), "dd");
    let undone = typed(cut, "u");
    assert_eq!((names(&undone), at(&undone)), (ALL.to_vec(), 3));
    let redone = after(undone, [Message::Redo(1)]);
    assert_eq!((names(&redone), at(&redone)), (without(&[2]), 3));
    // Restored output is selected, wherever the keys left the selection
    let far = typed(typed(on(3), "dd"), "5j");
    assert_eq!(at(&typed(far, "u")), 3);
    // Move selects the output that moved, both ways
    let moved = after(on(3), [Message::Move(2)]);
    let back = after(moved, [Message::Undo(1)]);
    assert_eq!((names(&back), at(&back)), (ALL.to_vec(), 3));
    let again = after(back, [Message::Redo(1)]);
    assert_eq!(at(&again), 5);
    // Counts go that many changes back
    let two = after(
        on(1),
        [Message::Move(1), Message::Move(1), Message::Undo(2)],
    );
    assert_eq!(names(&two), ALL);
}

#[test]
fn undo_of_a_change_to_a_cue_leaves_the_outputs_for_the_cue_list_as_a_jump() {
    let shown = after(
        show(cue(2)),
        [Message::Add(Place::After, 1), Message::ShowOutputs],
    );
    assert!(shown.outputs_shown);
    let undone = typed(shown, "u");
    assert!(!undone.outputs_shown);
    assert_eq!(undone.workspace.cues().count(), 14);
    assert_eq!(place(&undone).0, 0);
    // Control+o returns to the outputs
    let back = after(undone, [Message::Back(1)]);
    assert!(back.outputs_shown);
    // Redo does the same for a change that only cues made
    let redone = after(back, [Message::Redo(1)]);
    assert!(!redone.outputs_shown);
    assert_eq!(redone.workspace.cues().count(), 15);
}

#[test]
fn undo_and_redo_in_show_mode_leave_the_outputs_as_they_are() {
    let app = after(locked(on(3)), [Message::Undo(1), Message::Redo(1)]);
    assert!(app.outputs_shown);
    assert_eq!(names(&app), ALL);
}

#[test]
fn the_keys_that_enter_the_name_focus_the_field_of_the_output() {
    let started = |app: App, caret| app.handled(Message::Focus(Focus::Name(caret))).1.units();
    for caret in [Caret::End, Caret::Start, Caret::All] {
        assert!(started(on(2), caret) > 0, "{caret:?}");
        assert_eq!(started(locked(on(2)), caret), 0, "{caret:?}");
    }
    let hidden = App {
        output: None,
        ..on(2)
    };
    assert_eq!(started(hidden, Caret::End), 0);
}

#[test]
fn f2_of_the_cua_keymap_selects_the_name_of_the_output_and_tab_selects_the_next_field() {
    // F2 is bound to this action by the cua keymap, as its own tests show
    assert!(matches!(
        Action::ChangeName.message(None),
        Message::Focus(Focus::Name(Caret::All))
    ));
    // Moving by key starts the focus and then the selection of the text
    for focus in [Focus::Next, Focus::Previous] {
        assert!(
            on(2).handled(Message::Focus(focus.clone())).1.units() > 1,
            "{focus:?}"
        );
    }
}

#[test]
fn a_cut_by_keys_is_told_as_a_cut_by_the_change_and_a_delete_as_a_delete() {
    assert_eq!(told(&typed(on(3), "dd")), "cut the output Sub");
    assert_eq!(told(&typed(on(3), "2dj")), "cut 3 outputs");
    assert_eq!(
        told(&after(on(3), [Message::Delete(1)])),
        "deleted the output Sub"
    );
    // Leaving the outputs does not turn the cut into a delete
    let left = after(typed(on(3), "dd"), [Message::Back(1)]);
    assert_eq!(told(&left), "cut the output Sub");
}

#[test]
fn a_paste_by_keys_is_told_as_a_paste_and_an_add_as_an_add() {
    assert_eq!(told(&typed(on(3), "ddp")), "pasted the output Sub");
    assert_eq!(told(&typed(on(3), "yy2p")), "pasted 2 outputs");
    assert_eq!(
        told(&after(on(3), [Message::Add(Place::After, 1)])),
        "added the output Output 1"
    );
}

/// Types the keys as the window delivers them, one message each, which the
/// update cycle settles typed text for, unlike [`typed`]
fn keys(app: App, keys: &str) -> App {
    keys.chars().fold(app, |app, key| {
        let press = press(letter(&key.to_string()));
        app.next(Message::Typed {
            press,
            repeat: false,
        })
        .0
    })
}

#[test]
fn channels_typed_before_a_cut_apply_to_the_output_that_is_cut_and_pasted_back() {
    let sub = on(3).workspace.outputs()[2].id;
    let typing = after(
        on(3),
        [Message::SettingTyped(
            Setting::Channels(sub),
            "5-6".to_owned(),
        )],
    );
    assert_eq!(typing.workspace.outputs()[2].channels.to_string(), "1-2");
    // The key that cuts settles the text, and nothing of it is left to
    // apply to the output that has gone
    let cut = keys(typing, "dd");
    assert_eq!(cut.typed, None);
    assert_eq!(names(&cut), without(&[2]));
    let back = keys(cut, "P");
    let pasted = back
        .workspace
        .output(sub)
        .expect("the output took its identity back");
    assert_eq!(pasted.channels.to_string(), "5-6");
}

#[test]
fn undoing_an_addition_or_a_paste_selects_the_output_that_it_was_made_next_to() {
    let undone = |app: App, keys: &str| at(&typed(typed(app, keys), "u"));
    // Below and above, as `o u`, `O u`, `p u`, and `P u` of Vim end, with
    // counts, and at the first and the last place
    assert_eq!(undone(on(3), "o"), 3);
    assert_eq!(undone(on(3), "O"), 3);
    assert_eq!(undone(on(3), "3o"), 3);
    assert_eq!(undone(on(1), "O"), 1);
    assert_eq!(undone(on(10), "o"), 10);
    assert_eq!(undone(typed(on(3), "yy"), "p"), 3);
    assert_eq!(undone(typed(on(3), "yj"), "2P"), 3);
    // Wherever the selection went before the undo
    let moved = typed(typed(on(3), "o"), "4j");
    assert_eq!(at(&typed(moved, "u")), 3);
    // Redo selects what was added again, and undo goes back to the anchor
    let redone = after(typed(typed(on(3), "o"), "u"), [Message::Redo(1)]);
    assert_eq!(at(&redone), 4);
    assert_eq!(at(&typed(redone, "u")), 3);
    // Undo of a delete still selects what it brought back
    assert_eq!(at(&typed(typed(on(3), "dd"), "u")), 3);
}

#[test]
fn undo_and_redo_show_the_place_that_a_patch_or_output_change_touched() {
    // Light patch edit shows its tab, though the inspector was elsewhere
    let patched = after(
        show(cue(2)),
        [Message::Patch(PatchEdit::Add), Message::Tab(Tab::Basics)],
    );
    let undone = after(patched, [Message::Undo(1)]);
    assert_eq!(undone.tab, Tab::Patch);
    let redone = after(
        after(undone, [Message::Tab(Tab::Levels)]),
        [Message::Redo(1)],
    );
    assert_eq!(redone.tab, Tab::Patch);
    // Output edit made with the outputs in sight shows them again from the cue list
    let moved = after(on(3), [Message::Move(2)]).showing_cues();
    assert!(!moved.outputs_shown);
    let undone = after(moved, [Message::Undo(1)]);
    assert!(undone.outputs_shown);
    assert_eq!((names(&undone), at(&undone)), (ALL.to_vec(), 3));
}

#[test]
fn a_count_that_reaches_a_change_to_the_outputs_shows_them_as_one_change_does() {
    let made = after(
        show(cue(2)),
        [Message::ShowOutputs, Message::Add(Place::After, 1)],
    )
    .showing_cues();
    let made = after(made, [Message::Add(Place::After, 1)]);
    let undone = after(made, [Message::Undo(2)]);
    assert!(undone.outputs_shown);
    assert_eq!(undone.workspace.cues().count(), 14);
    assert_eq!(undone.workspace.outputs().len(), 1);
    assert_eq!(undone.output, Some(undone.workspace.outputs()[0].id));
    // Control+o leaves them for the cue list, where the count began
    let back = after(undone, [Message::Back(1)]);
    assert!(!back.outputs_shown);
    // Redo does the same on its way, and ends where the last change lies
    let redone = after(back, [Message::Redo(2)]);
    assert!(!redone.outputs_shown);
    assert_eq!(redone.workspace.cues().count(), 15);
    assert_eq!(redone.workspace.outputs().len(), 2);
    assert_eq!(redone.turns, 2);
}
