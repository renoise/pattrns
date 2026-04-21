use std::{
    cell::RefCell,
    fs,
    path::Path,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use notify::{RecursiveMode, Watcher};
use simplelog::*;

use pattrns::{
    bindings::new_pattern_from_file,
    patterns::BeatTimePattern,
    player::{
        phonic::{generators, DefaultOutputDevice, Generator, GeneratorPlaybackOptions},
        Player,
    },
    BeatTimeBase, BeatTimeStep, Phrase,
};

// -------------------------------------------------------------------------------------------------

#[cfg(feature = "dhat-profiler")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

// -------------------------------------------------------------------------------------------------

// TODO: make this configurable with an cmd line arg
const DEMO_PATH: &str = "./examples/assets";
// Number of voices in each instrument's sampler
const SAMPLER_VOICE_COUNT: usize = 32;

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

    // create sample player
    let mut player = Player::new(DefaultOutputDevice::open()?, None)?;

    // fetch contents from demo dir
    log::info!("Searching for wav/script files in path '{DEMO_PATH}'...");
    let mut script_paths = vec![];
    let mut pattern_index = 0;
    for dir_entry in fs::read_dir(DEMO_PATH)?.flatten() {
        let path = dir_entry.path();
        if let Some(extension) = path.extension().map(|e| e.to_string_lossy()) {
            // collect all audio file's that have a lua file next to it
            if matches!(extension.as_bytes(), b"mp3" | b"wav" | b"flac") {
                let audio_file_path = path;
                let script_path = audio_file_path.clone().with_extension("lua");
                if script_path.exists() {
                    let sampler = generators::Sampler::from_file(
                        audio_file_path,
                        GeneratorPlaybackOptions::default().voices(SAMPLER_VOICE_COUNT),
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
                    player.set_generator(pattern_index, sampler.into_box(), None)?;
                    script_paths.push(script_path);
                    pattern_index += 1;
                }
            }
        }
    }

    // set default time base config
    let beat_time = BeatTimeBase {
        beats_per_min: 124.0,
        beats_per_bar: 4,
        samples_per_sec: player.sample_rate(),
    };

    // Watch for script changes, signaling in 'script_files_changed'
    let script_files_changed = Arc::new(AtomicBool::new(false));

    let mut watcher = notify::recommended_watcher({
        let script_files_changed = script_files_changed.clone();
        move |res: Result<notify::Event, notify::Error>| match res {
            Ok(event) => {
                if !event.kind.is_access() {
                    log::info!("File change event: {event:?}");
                    script_files_changed.store(true, Ordering::Relaxed);
                }
            }
            Err(err) => log::error!("File watch error: {err}"),
        }
    })?;
    watcher.watch(Path::new(DEMO_PATH), RecursiveMode::Recursive)?;

    // stop on Control-C
    let stop_running = Arc::new(AtomicBool::new(false));
    ctrlc::set_handler({
        let stop_running = stop_running.clone();
        move || {
            stop_running.store(true, Ordering::Relaxed);
        }
    })?;

    // Build a phrase from all current script files
    let build_phrase = || {
        let load = |file_name: &Path| {
            new_pattern_from_file(beat_time, None, file_name).unwrap_or_else(|err| {
                log::warn!(
                    "Script '{}' failed to compile:\n{}",
                    file_name.display(),
                    err
                );
                Rc::new(RefCell::new(BeatTimePattern::new(
                    beat_time,
                    BeatTimeStep::Beats(1.0),
                )))
            })
        };
        Phrase::new(
            beat_time,
            script_paths.iter().map(|path| load(path)).collect(),
            BeatTimeStep::Bar(4.0),
        )
    };

    // Start playing and keep the handle alive across script reloads
    let mut handle = player.play_phrase(build_phrase());
    while !stop_running.load(Ordering::Relaxed) {
        let sleep_duration = handle.run(player.inner().output_sample_frame_position());
        if script_files_changed.load(Ordering::Relaxed) {
            script_files_changed.store(false, Ordering::Relaxed);
            log::info!("Rebuilding all patterns...");
            handle.swap_phrase(build_phrase());
        }
        std::thread::sleep(sleep_duration);
    }
    handle.stop();

    #[cfg(feature = "dhat-profiler")]
    drop(profiler);

    Ok(())
}
