use std::hint::black_box;

use criterion::{criterion_group, Criterion};

use pattrns::prelude::*;

// ---------------------------------------------------------------------------------------------

fn create_phrase() -> Phrase {
    let beat_time = BeatTimeBase {
        samples_per_sec: 44100,
        beats_per_min: 130.0,
        beats_per_bar: 4,
    };

    let kick_cycle = new_cycle_emitter(
        "bd? [~ bd] ~ ~ bd [~ bd] _ ~ bd [~ bd?] ~ ~ bd [~ bd] [_ bd2] [~ bd _ ~]",
    )
    .unwrap()
    .with_mappings(&[
        ("bd", vec![new_note("c4")]),
        ("bd2", vec![new_note(("c4", None, None, 0.5))]),
    ]);
    let kick_pattern = beat_time.every_nth_beat(16.0).emit(kick_cycle);

    let snare_pattern = beat_time
        .every_nth_beat(2.0)
        .with_offset(BeatTimeStep::Beats(1.0))
        .emit(new_note_emitter("C_5"));

    let hihat_pattern = beat_time
        .every_nth_sixteenth(2.0)
        .emit(new_note_emitter("C_5").mutate({
            let mut step = 0;
            move |event| {
                if let Event::NoteEvents(notes) = event {
                    for note in notes.iter_mut().flatten() {
                        note.volume = 1.0 / (step + 1) as f32;
                        step += 1;
                        if step >= 3 {
                            step = 0;
                        }
                    }
                }
            }
        }));

    let hihat_pattern2 = beat_time
        .every_nth_sixteenth(2.0)
        .with_offset(BeatTimeStep::Sixteenth(1.0))
        .emit(new_note_emitter("C_5").mutate({
            let mut vel_step = 0;
            let mut note_step = 0;
            move |event| {
                if let Event::NoteEvents(notes) = event {
                    for note in notes.iter_mut().flatten() {
                        note.volume = 1.0 / (vel_step + 1) as f32 * 0.5;
                        vel_step += 1;
                        if vel_step >= 3 {
                            vel_step = 0;
                        }
                        note.note = Note::from((Note::C4 as u8) + 32 - note_step);
                        note_step += 1;
                        if note_step >= 32 {
                            note_step = 0;
                        }
                    }
                }
            }
        }));

    let hihat_pattern = Phrase::new(
        beat_time,
        vec![hihat_pattern, hihat_pattern2],
        BeatTimeStep::Bar(4.0),
    );

    let bass_notes = Scale::try_from((Note::C5, "aeolian")).unwrap().notes();
    let bass_pattern = beat_time
        .every_nth_eighth(1.0)
        .with_rhythm([1, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1].to_rhythm())
        .emit(new_note_sequence_emitter(vec![
            new_note((bass_notes[0], None, None, 0.5)),
            new_note((bass_notes[2], None, None, 0.5)),
            new_note((bass_notes[3], None, None, 0.5)),
            new_note((bass_notes[0], None, None, 0.5)),
            new_note((bass_notes[2], None, None, 0.5)),
            new_note((bass_notes[3], None, None, 0.5)),
            new_note((bass_notes[6].transposed(-12), None, None, 0.5)),
        ]));

    let synth_pattern = beat_time
        .every_nth_bar(4.0)
        .emit(new_polyphonic_note_sequence_emitter(vec![
            vec![
                new_note(("C 4", None, None, 0.3)),
                new_note(("D#4", None, None, 0.3)),
                new_note(("G 4", None, None, 0.3)),
            ],
            vec![
                new_note(("C 4", None, None, 0.3)),
                new_note(("D#4", None, None, 0.3)),
                new_note(("F 4", None, None, 0.3)),
            ],
            vec![
                new_note(("C 4", None, None, 0.3)),
                new_note(("D#4", None, None, 0.3)),
                new_note(("G 4", None, None, 0.3)),
            ],
            vec![
                new_note(("C 4", None, None, 0.3)),
                new_note(("D#4", None, None, 0.3)),
                new_note(("A#4", None, None, 0.3)),
            ],
        ]));

    let fx_pattern = beat_time
        .every_nth_seconds(8.0)
        .emit(new_polyphonic_note_sequence_emitter(vec![
            vec![new_note(("C 4", None, None, 0.2)), None, None],
            vec![None, new_note(("C 4", None, None, 0.2)), None],
            vec![None, None, new_note(("F 4", None, None, 0.2))],
        ]));

    Phrase::new(
        beat_time,
        vec![
            PatternSlot::from(kick_pattern),
            PatternSlot::from(snare_pattern),
            PatternSlot::from(hihat_pattern),
            PatternSlot::from(bass_pattern),
            PatternSlot::from(fx_pattern),
            PatternSlot::from(synth_pattern),
        ],
        BeatTimeStep::Bar(8.0),
    )
}

// ---------------------------------------------------------------------------------------------

pub fn create(c: &mut Criterion) {
    let mut group = c.benchmark_group("Rust Phrase");
    group.bench_function("Create", |b| {
        b.iter(|| {
            black_box(create_phrase());
        })
    });
    group.finish();
}

pub fn clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("Rust Phrase");
    let phrase = create_phrase();
    group.bench_function("Clone", |b| {
        b.iter(|| {
            let mut phrase = black_box(phrase.clone());
            phrase.reset();
        })
    });
    group.finish();
}

pub fn run(c: &mut Criterion) {
    let event_count = 2500;
    let mut group = c.benchmark_group("Rust Phrase");
    group.measurement_time(std::time::Duration::from_secs(10));
    let phrase = create_phrase();
    group.bench_function("Run", |b| {
        b.iter_batched(
            || {
                let mut phrase = phrase.clone();
                phrase.reset();
                phrase
            },
            |mut phrase| {
                let sample_time = SampleTime::MAX;
                let mut num_events = 0;
                while let Some(event) = phrase.run_until_time(sample_time) {
                    black_box(event);
                    num_events += 1;
                    if num_events >= event_count {
                        break;
                    }
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });
    group.finish();
}

pub fn seek(c: &mut Criterion) {
    let mut group = c.benchmark_group("Rust Phrase");
    let phrase = create_phrase();
    let samples_per_sec = phrase.time_base().samples_per_sec as SampleTime;
    let seek_step = 10;
    let seek_time = 60 * 60;
    group.bench_function("Seek", |b| {
        b.iter_batched(
            || {
                let mut phrase = phrase.clone();
                phrase.reset();
                phrase
            },
            |mut phrase| {
                let mut sample_time = samples_per_sec;
                while sample_time < seek_time * samples_per_sec {
                    phrase.advance_until_time(sample_time);
                    sample_time += seek_step * samples_per_sec;
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });
    group.finish();
}

// ---------------------------------------------------------------------------------------------

criterion_group! {
    name = pattern;
    config = Criterion::default();
    targets = create, clone, run, seek
}
