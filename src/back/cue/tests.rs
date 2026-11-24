// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of cues: their levels, names, clips, and delays

mod passing;

use super::*;

/// Returns the delay between a cue firing and it firing the next cue of a
/// list that holds no group
fn next_delay(cue: &Cue) -> Option<f32> {
    cue.next_delay_in(&CueList::new("Main", Vec::new()).groups())
}

fn audio(name: &str, file: &str) -> Cue {
    Cue::new(CueId(1), "1", Kind::Audio, name)
        .file(file)
        .build()
}

#[test]
fn typed_level_reads_with_or_without_its_unit() {
    assert_eq!(parse_level("-6"), Some(-6.0));
    assert_eq!(parse_level(" +3.5 dB "), Some(3.5));
    assert_eq!(parse_level("-60db"), Some(-60.0));
}

#[test]
fn typed_level_past_either_end_or_half_typed_reads_as_none() {
    assert_eq!(parse_level("-"), None);
    assert_eq!(parse_level("-61"), None);
    assert_eq!(parse_level("13"), None);
    assert_eq!(parse_level("NaN"), None);
}

#[test]
fn cue_without_a_number_displays_as_its_name_alone() {
    let numbered = audio("Rain", "rain.wav");
    let unnumbered = Cue::new(CueId(2), " ", Kind::Audio, "Rain").build();
    assert_eq!(numbered.to_string(), "1 Rain");
    assert_eq!(unnumbered.to_string(), "Rain");
}

#[test]
fn chosen_sound_file_names_a_cue_until_the_operator_names_it() {
    let chosen = |cue: Cue| cue.playing("sounds/thunder.wav".to_owned());
    let thunder = Edit::FileAndName {
        file: "sounds/thunder.wav".to_owned(),
        name: "thunder".to_owned(),
    };
    assert_eq!(chosen(audio(&Cue::new_name(Kind::Audio), "")), thunder);
    assert_eq!(chosen(audio("storm", "sounds/storm.ogg")), thunder);
    assert_eq!(
        chosen(audio("Storm build", "sounds/storm.ogg")),
        Edit::File("sounds/thunder.wav".to_owned())
    );
}

fn loaded(cue: Cue) -> Cue {
    Cue {
        media: Media::Loaded {
            duration: 10.0,
            sound: Some(10.0),
        },
        ..cue
    }
}

#[test]
fn clip_runs_from_the_start_to_the_end_or_to_the_end_of_the_file() {
    let whole = loaded(audio("storm", "storm.ogg"));
    let clip = |start: f64, end: f64| {
        Some(Clip {
            start,
            end,
            looping: None,
        })
    };
    assert_eq!(whole.clip(), clip(0.0, 10.0));
    let trimmed = Cue {
        start: 1.5.into(),
        end: 4.0.into(),
        ..whole.clone()
    };
    assert_eq!(trimmed.clip(), clip(1.5, 4.0));
    assert_eq!(trimmed.action(), 2.5);
    let past = Cue {
        end: 30.0.into(),
        ..whole
    };
    assert_eq!(past.clip(), clip(0.0, 10.0));
}

#[test]
fn cue_that_starts_at_or_after_its_end_has_no_clip() {
    let whole = loaded(audio("storm", "storm.ogg"));
    let at_end = Cue {
        start: 4.0.into(),
        end: 4.0.into(),
        ..whole.clone()
    };
    assert_eq!(at_end.clip(), None);
    let past_file = Cue {
        start: 12.0.into(),
        ..whole
    };
    assert_eq!(past_file.clip(), None);
}

#[test]
fn clip_that_loops_goes_back_to_its_start_at_its_end() {
    let clip = Clip {
        start: 2.0,
        end: 5.0,
        looping: Some(Loop {
            start: 2.0,
            end: 5.0,
            released: None,
        }),
    };
    assert_eq!(clip.position(1.0), 3.0);
    assert_eq!(clip.position(4.0), 3.0);
    let once = Clip {
        looping: None,
        ..clip
    };
    assert_eq!(once.position(4.0), 6.0);
}

#[test]
fn audio_cue_that_loops_follows_on_to_nothing() {
    let looping = Cue {
        looping: true,
        advance: Advance::Follow,
        ..loaded(audio("rain", "rain.ogg"))
    };
    assert_eq!(next_delay(&looping), None);
    let continuing = Cue {
        advance: Advance::Continue,
        ..looping
    };
    assert_eq!(next_delay(&continuing), Some(0.0));
}

#[test]
fn silence_is_where_kira_stops_making_sound() {
    assert_eq!(SILENCE, kira::Decibels::SILENCE.0);
}

fn video(start: f32, looping: bool) -> Cue {
    Cue {
        media: Media::Loaded {
            duration: 10.0,
            sound: Some(9.5),
        },
        ..Cue::new(CueId(1), "1", Kind::Video, "Movie")
            .file("movie.mp4")
            .start(start)
            .looping(looping)
            .build()
    }
}

#[test]
fn video_cue_that_loops_ends_with_its_sound_where_that_ends_first() {
    let end = |cue: Cue| cue.clip().map(|clip| clip.end);
    assert_eq!(end(video(0.0, true)), Some(9.5));
    assert_eq!(end(video(0.0, false)), Some(10.0));
    assert!(video(0.0, true).sounds());
}

#[test]
fn video_cue_that_starts_past_the_end_of_its_sound_plays_its_picture_alone() {
    let late = video(9.7, true);
    assert!(!late.sounds());
    let clip = late.clip().expect("the picture plays");
    assert_eq!((clip.start as f32, clip.end), (9.7, 10.0));
}

#[test]
fn picture_cue_shows_its_file_whole_and_follows_on_to_nothing() {
    let picture = Cue {
        media: Media::Loaded {
            duration: 0.0,
            sound: None,
        },
        advance: Advance::Follow,
        ..Cue::new(CueId(1), "1", Kind::Picture, "New Picture cue").build()
    };
    assert_eq!(
        (picture.clip(), picture.shown()),
        (None, Some(Shown::Still))
    );
    assert!(picture.endless() && !picture.sounds());
    assert_eq!(next_delay(&picture), None);
    let named = picture.playing("slides/preshow.png".to_owned());
    assert!(matches!(named, Edit::FileAndName { name, .. } if name == "preshow"));
}

#[test]
fn audio_cue_that_follows_goes_on_when_its_clip_ends() {
    let following = Cue {
        advance: Advance::Follow,
        pre_wait: 1.5.into(),
        post_wait: 9.0.into(),
        start: 2.0.into(),
        ..loaded(audio("rain", "rain.ogg"))
    };
    // Auto-follow ignores the post-wait
    assert_eq!(next_delay(&following), Some(9.5));
}

#[test]
fn audio_cue_that_continues_goes_on_after_its_waits() {
    let continuing = Cue {
        advance: Advance::Continue,
        pre_wait: 1.5.into(),
        post_wait: 2.0.into(),
        ..loaded(audio("rain", "rain.ogg"))
    };
    assert_eq!(next_delay(&continuing), Some(3.5));
}

#[test]
fn audio_cue_that_waits_for_go_goes_on_to_nothing() {
    assert_eq!(next_delay(&loaded(audio("rain", "rain.ogg"))), None);
}

#[test]
fn clip_has_played_as_far_as_its_sound_has_reached_in_its_file() {
    let clip = |looping: bool| Clip {
        start: 2.0,
        end: 5.0,
        looping: looping.then_some(Loop {
            start: 2.0,
            end: 5.0,
            released: None,
        }),
    };
    assert_eq!(clip(false).played_at(4.5, 2.4), 2.5);
    // Three seconds a pass: the sound at 3.0 in the file is a second into a
    // pass, and the clock tells which
    let pass = |position, near| clip(true).played_at(position, near);
    assert_eq!(pass(3.0, 0.5), 1.0);
    assert_eq!(pass(3.0, 3.4), 4.0);
    // Sound just past the loop point, with a clock just short of it, and
    // the other way round
    assert!((pass(2.1, 2.9) - 3.1).abs() < 1e-9);
    assert!((pass(4.9, 3.1) - 2.9).abs() < 1e-9);
    // Each position of the sound maps back to the time it came from
    assert!(
        [0.0, 0.7, 3.0, 4.2, 20.5]
            .iter()
            .all(|&at| (pass(clip(true).position(at), at) - at).abs() < 1e-9)
    );
}

/// Returns a clip from 1 to 9 seconds into its file that loops from 3 to 5,
/// so that it reaches the end of its loop first after 4 seconds and goes
/// round the loop every 2
fn sectioned() -> Clip {
    Clip {
        start: 1.0,
        end: 9.0,
        looping: Some(Loop {
            start: 3.0,
            end: 5.0,
            released: None,
        }),
    }
}

#[test]
fn clip_plays_to_the_end_of_its_loop_then_goes_round_the_loop() {
    let clip = sectioned();
    assert_eq!(clip.position(1.5), 2.5);
    assert_eq!(clip.position(3.5), 4.5);
    assert_eq!(clip.position(4.5), 3.5);
    assert_eq!(clip.position(7.0), 4.0);
    assert_eq!(clip.lasts(), None);
    assert!(
        [0.0, 1.9, 2.0, 3.2, 4.0, 6.9, 21.3].iter().all(|&at| (clip
            .played_at(clip.position(at), at)
            - at)
            .abs()
            < 1e-9)
    );
}

#[test]
fn devamp_lets_a_loop_go_on_at_the_end_of_the_pass_under_way() {
    let clip = sectioned();
    let released = |played| {
        clip.released(played, 0.5)
            .looping
            .and_then(|loop_| loop_.released)
    };
    assert_eq!(released(0.5), Some(4.0));
    assert_eq!(released(4.2), Some(6.0));
    // Too near the end of the pass for the sound to leave the loop there
    assert_eq!(released(5.7), Some(8.0));
    let free = clip.released(4.2, 0.5);
    assert_eq!(free.lasts(), Some(10.0));
    assert_eq!(free.last_pass(), Some(4.0));
    assert_eq!(free.position(5.0), 4.0);
    assert_eq!(free.position(7.5), 6.5);
    assert_eq!(free.played_at(6.5, 7.4), 7.5);
    // Sound still in its last pass while the clock has passed its end
    assert_eq!(free.played_at(4.9, 6.2), 5.9);
    assert_eq!(free.released(4.9, 0.5), free);
}

#[test]
fn clip_makes_passes_through_its_loop_and_then_to_its_end() {
    let clip = sectioned();
    let pass = |from, to, at| Some(Pass { from, to, at });
    assert_eq!(clip.pass(0), pass(1.0, 5.0, 0.0));
    assert_eq!(clip.pass(1), pass(3.0, 5.0, 4.0));
    assert_eq!(clip.pass(7), pass(3.0, 5.0, 16.0));
    let free = clip.released(4.2, 0.5);
    assert_eq!(free.pass(1), pass(3.0, 5.0, 4.0));
    assert_eq!(free.pass(2), pass(5.0, 9.0, 6.0));
    assert_eq!(free.pass(3), None);
    let to_end = Clip { end: 5.0, ..free };
    assert_eq!(to_end.pass(2), None);
    assert_eq!(to_end.lasts(), Some(6.0));
    let once = Clip {
        looping: None,
        ..clip
    };
    assert_eq!(once.pass(0), pass(1.0, 9.0, 0.0));
    assert_eq!(once.pass(1), None);
}

#[test]
fn loop_lies_within_the_clip_of_its_cue() {
    let cue = |loop_start: f32, loop_end: f32| Cue {
        start: 1.0.into(),
        end: 8.0.into(),
        looping: true,
        loop_start: loop_start.into(),
        loop_end: loop_end.into(),
        ..loaded(audio("vamp", "vamp.ogg"))
    };
    let section = |cue: Cue| cue.clip().and_then(|clip| clip.looping);
    let within = |start, end| {
        Some(Loop {
            start,
            end,
            released: None,
        })
    };
    assert_eq!(section(cue(3.0, 5.0)), within(3.0, 5.0));
    assert_eq!(section(cue(0.0, 30.0)), within(1.0, 8.0));
    let whole = Cue {
        loop_start: Seconds::default(),
        loop_end: Seconds::default(),
        ..cue(3.0, 5.0)
    };
    assert_eq!(section(whole), within(1.0, 8.0));
    let backward = cue(5.0, 3.0);
    assert_eq!(backward.clip(), None);
    assert!(backward.loops_nothing());
    assert!(!cue(3.0, 5.0).loops_nothing());
}

#[test]
fn minus_infinity_reads_as_silence() {
    assert_eq!(parse_level("-inf"), Some(SILENCE));
    assert_eq!(parse_level(" -INF dB "), Some(SILENCE));
    assert_eq!(parse_level("inf"), None);
}

#[test]
fn send_level_reads_off_as_no_send() {
    assert_eq!(parse_send("off"), Some(None));
    assert_eq!(parse_send(" Off "), Some(None));
    assert_eq!(parse_send(""), Some(None));
    assert_eq!(parse_send("-6"), Some(Some(-6.0)));
    assert_eq!(parse_send("-inf"), Some(Some(SILENCE)));
    assert_eq!(parse_send("-61"), None);
}

#[test]
fn loop_played_tells_where_the_loop_that_plays_differs_from_the_one_typed() {
    let cue = |loop_start: f32, loop_end: f32| Cue {
        start: 1.0.into(),
        end: 8.0.into(),
        looping: true,
        loop_start: loop_start.into(),
        loop_end: loop_end.into(),
        ..loaded(audio("vamp", "vamp.ogg"))
    };
    assert_eq!(cue(3.0, 5.0).loop_played(), None);
    assert_eq!(cue(3.0, 30.0).loop_played(), Some((3.0, 8.0)));
    assert_eq!(cue(0.5, 5.0).loop_played(), Some((1.0, 5.0)));
    assert_eq!(cue(5.0, 3.0).loop_played(), None);
    let whole = Cue {
        loop_start: Seconds::default(),
        loop_end: Seconds::default(),
        ..cue(3.0, 5.0)
    };
    assert_eq!(whole.loop_played(), None);
}

#[test]
fn a_name_with_line_breaks_reads_on_one_line() {
    let cue = Cue::new(CueId(1), "4", Kind::Audio, "Two\nlines\t name").build();
    assert_eq!(cue.to_string(), "4 Two lines name");
    assert_eq!(Cue::one_line("a\n\n  \nb\r\n"), "a b");
}
