use std::{
    path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use simplelog::*;

use pattrns::{
    player::phonic::{
        fundsp::{audiounit::AudioUnit, shared::Shared},
        generators, DefaultOutputDevice, Generator, GeneratorPlaybackHandle,
        GeneratorPlaybackOptions,
    },
    prelude::*,
};

// -------------------------------------------------------------------------------------------------

#[cfg(feature = "dhat-profiler")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

// -------------------------------------------------------------------------------------------------

#[allow(non_snake_case)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "dhat-profiler")]
    let profiler = dhat::Profiler::builder().trim_backtraces(Some(100)).build();

    // init logging
    TermLogger::init(
        log::STATIC_MAX_LEVEL,
        ConfigBuilder::default().build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap_or_else(|err| {
        log::error!("init_logger error: {err:?}");
    });

    // create player
    let mut player = Player::new(DefaultOutputDevice::open()?, None)?;
    player.set_show_events(true);

    // instrument indices
    const KICK: usize = 0;
    const SNARE: usize = 1;
    const HIHAT: usize = 2;
    const BASS: usize = 3;
    const SYNTH: usize = 4;
    const FX: usize = 5;

    // create and add instruments
    fn add_sampler(
        player: &mut Player,
        pattern_index: usize,
        file_name: &str,
        voice_count: usize,
    ) -> Result<phonic::GeneratorPlaybackHandle, phonic::Error> {
        let sample_path = path::PathBuf::from(format!("./examples/assets/{file_name}"));
        let sampler = generators::Sampler::from_file(
            sample_path,
            GeneratorPlaybackOptions::default().voices(voice_count),
            player.channel_count(),
            player.sample_rate(),
        )?
        .with_ahdsr(generators::AhdsrParameters::new(
            Duration::ZERO,
            Duration::ZERO,
            Duration::ZERO,
            1.0,
            Duration::from_millis(350),
        )?)?;
        player.set_generator(pattern_index, sampler.into_box(), None)
    }

    fn add_synth<F>(
        player: &mut Player,
        pattern_index: usize,
        name: &str,
        voice_factory: F,
        voice_count: usize,
    ) -> Result<GeneratorPlaybackHandle, phonic::Error>
    where
        F: Fn(Shared, Shared, Shared, Shared) -> Box<dyn AudioUnit> + Send + Sync + 'static,
    {
        let output_sample_rate = player.sample_rate();
        let generator = generators::FunDspGenerator::new(
            name, //
            voice_factory,
            GeneratorPlaybackOptions::default().voices(voice_count),
            output_sample_rate,
        )?;
        player.set_generator(pattern_index, generator.into_box(), None)
    }

    add_sampler(&mut player, KICK, "kick.wav", 2)?;
    add_sampler(&mut player, SNARE, "snare.wav", 2)?;
    add_sampler(&mut player, HIHAT, "hihat.wav", 2)?;
    // let BASS = add_sampler("bass.wav", 4)?;
    add_synth(
        &mut player,
        BASS,
        "bass_synth",
        {
            use phonic::fundsp::prelude32::*;
            |gate: Shared, freq: Shared, vol: Shared, panning: Shared| -> Box<dyn AudioUnit> {
                // Apply gate via an ADSR envelope
                let envelope = var(&gate) >> adsr_live(0.01, 0.1, 0.7, 1.0);
                // Create 3 sine modulators
                let modulator_ratio1 = 1.98;
                let modulator_ratio2 = 3.01;
                let modulator_ratio3 = 5.97;
                let mod_index1 = 3.5;
                let mod_index2 = 2.2;
                let mod_index3 = 1.8;
                let modulator1 = (var(&freq) * modulator_ratio1) >> sine();
                let modulator2 = (var(&freq) * modulator_ratio2) >> sine();
                let modulator3 = (var(&freq) * modulator_ratio3) >> sine();
                let mod_amount1 = modulator1 * var(&freq) * mod_index1;
                let mod_amount2 = modulator2 * var(&freq) * mod_index2;
                let mod_amount3 = modulator3 * var(&freq) * mod_index3 * 0.75;
                let carrier1 = (var(&freq) + mod_amount1.clone()) >> sine();
                let carrier2 = ((var(&freq) * 1.003) + mod_amount2 * 0.7) >> sine();
                let carrier3 = ((var(&freq) * 0.997) + mod_amount3) >> sine();
                // Add an extra square wave for brightness
                let square_carrier = ((var(&freq) + mod_amount1.clone() * 0.4) >> square()) * 0.3;
                // Mix all carriers with different amplitudes and a LP
                let fm_sound =
                    ((carrier1 * 0.5 + carrier2 * 0.35 + carrier3 * 0.15 + square_carrier) * 0.5)
                        >> lowpass_hz(1600.0, 0.6);
                // Apply envelope
                let final_sound = fm_sound * envelope.clone();
                // Combine final sound with volume and panning (which makes it stereo)
                Box::new(((final_sound * var(&vol)) | var(&panning)) >> panner())
            }
        },
        4,
    )?;
    add_sampler(&mut player, SYNTH, "synth.wav", 16)?;
    // let TONE = add_sampler("tone.wav", None, 16)?;
    add_sampler(&mut player, FX, "fx.wav", 8)?;

    // define our time bases
    let second_time = SecondTimeBase {
        samples_per_sec: player.sample_rate(),
    };
    let beat_time = BeatTimeBase {
        beats_per_min: 130.0,
        beats_per_bar: 4,
        samples_per_sec: second_time.samples_per_sec,
    };

    // generate a few phrases
    let cycle = new_cycle_emitter(
        "bd [~ bd] ~ ~ bd [~ bd] _ ~ bd [~ bd] ~ ~ bd [~ bd] [_ bd2] [~ bd _ ~]",
    )?
    .with_mappings(&[
        ("bd", vec![new_note("c4")]),
        ("bd2", vec![new_note(("c4", None, None, 0.5))]),
    ]);

    let kick_pattern = beat_time.every_nth_beat(16.0).emit(cycle);

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
        .with_rhythm([1.0, 0.5].to_rhythm())
        .with_gate(ProbabilityGate::new(None))
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

    // combine two hi hat rhythms into a phrase
    let hihat_pattern = Phrase::new(
        beat_time,
        vec![hihat_pattern, hihat_pattern2],
        BeatTimeStep::Bar(4.0),
    );

    let bass_notes = Scale::try_from((Note::C3, "aeolian"))?.notes();
    let bass_pattern = beat_time
        .every_nth_eighth(1.0)
        .with_rhythm([1, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1].to_rhythm())
        .emit(new_note_sequence_emitter(vec![
            new_note((bass_notes[0], None, None, 0.33)),
            new_note((bass_notes[2], None, Some(0.5), 0.25)),
            new_note((bass_notes[3], None, None, 0.33)),
            new_note((bass_notes[0], None, None, 0.33)),
            new_note((bass_notes[2], None, Some(0.0), 0.25)),
            new_note((bass_notes[3], None, None, 0.33)),
            new_note((bass_notes[6].transposed(-12), None, Some(1.0), 0.33)),
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

    // arrange rhythms into phrases and sequence up these phrases to create a little arrangement
    let mut sequence = Sequence::new(
        beat_time,
        vec![
            Phrase::new(
                beat_time,
                vec![
                    PatternSlot::from(kick_pattern),
                    PatternSlot::from(snare_pattern),
                    PatternSlot::Stop, // hihat
                    PatternSlot::Stop, // bass
                    PatternSlot::Stop, // synth
                    PatternSlot::Stop, // fx
                ],
                BeatTimeStep::Bar(8.0),
            ),
            Phrase::new(
                beat_time,
                vec![
                    PatternSlot::Continue, // kick
                    PatternSlot::Continue, // snare
                    PatternSlot::from(hihat_pattern),
                    PatternSlot::from(bass_pattern),
                    PatternSlot::Stop, // synth
                    PatternSlot::Stop, // fx
                ],
                BeatTimeStep::Bar(8.0),
            ),
            Phrase::new(
                beat_time,
                vec![
                    PatternSlot::Continue, // kick
                    PatternSlot::Continue, // snare
                    PatternSlot::Continue, // hihat
                    PatternSlot::Continue, // bass
                    PatternSlot::from(synth_pattern),
                    PatternSlot::Stop, // fx
                ],
                BeatTimeStep::Bar(16.0),
            ),
            Phrase::new(
                beat_time,
                vec![
                    PatternSlot::Continue, // kick
                    PatternSlot::Continue, // snare
                    PatternSlot::Continue, // hihat
                    PatternSlot::Continue, // bass
                    PatternSlot::Continue, // synth
                    PatternSlot::from(fx_pattern),
                ],
                BeatTimeStep::Bar(16.0),
            ),
        ],
    );

    // stop on Control-C
    let stop_running = Arc::new(AtomicBool::new(false));
    ctrlc::set_handler({
        let stop_running = stop_running.clone();
        move || {
            stop_running.store(true, Ordering::Relaxed);
        }
    })?;

    // play the sequence and dump events to stdout
    let previous_sequence = None;
    let reset_playback_pos = false;
    player.run_until(
        previous_sequence,
        &mut sequence,
        &beat_time,
        reset_playback_pos,
        || stop_running.load(Ordering::Relaxed),
    );

    #[cfg(feature = "dhat-profiler")]
    drop(profiler);

    Ok(())
}
