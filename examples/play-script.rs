use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use notify::{RecursiveMode, Watcher};
use simplelog::*;

use pattrns::prelude::*;

// -------------------------------------------------------------------------------------------------

#[cfg(feature = "dhat-profiler")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

// -------------------------------------------------------------------------------------------------

// TODO: make this configurable with an cmd line arg
const DEMO_PATH: &str = "./examples/assets";

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

    // fetch contents from demo dir
    log::info!("Searching for wav/script files in path '{DEMO_PATH}'...");
    let sample_pool = Arc::new(SamplePool::new());
    struct PatternEntry {
        instrument_id: InstrumentId,
        script_path: PathBuf,
    }
    let mut entries = vec![];
    for dir_entry in fs::read_dir(DEMO_PATH)?.flatten() {
        let path = dir_entry.path();
        if let Some(extension) = path.extension().map(|e| e.to_string_lossy()) {
            // collect all audio file's that have a lua file next to it
            if matches!(extension.as_bytes(), b"mp3" | b"wav" | b"flac") {
                let script_path = path.clone().with_extension("lua");
                if script_path.exists() {
                    let instrument_id = sample_pool.load_sample(path)?;
                    entries.push(PatternEntry {
                        instrument_id,
                        script_path,
                    });
                }
            }
        }
    }

    // create sample player
    let mut player = SamplePlayer::new(sample_pool, None)?;

    // set default time base config
    let beat_time = BeatTimeBase {
        beats_per_min: 124.0,
        beats_per_bar: 4,
        samples_per_sec: player.file_player().output_sample_rate(),
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

    // (re)run all scripts
    while !stop_running.load(Ordering::Relaxed) {
        if script_files_changed.load(Ordering::Relaxed) {
            script_files_changed.store(false, Ordering::Relaxed);
            log::info!("Rebuilding all patterns...");
        }

        // build final phrase
        let load = |instrument: Option<InstrumentId>, file_name: &Path| {
            new_pattern_from_file(beat_time, instrument, file_name).unwrap_or_else(|err| {
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
        let phrase = Phrase::new(
            beat_time,
            entries
                .iter()
                .map(|e| load(Some(e.instrument_id), &e.script_path))
                .collect(),
            BeatTimeStep::Bar(4.0),
        );

        // wrap phrase into a sequence
        let mut sequence = Sequence::new(beat_time, vec![phrase]);

        let reset_playback_pos = false;
        player.run_until(&mut sequence, &beat_time, reset_playback_pos, || {
            script_files_changed.load(Ordering::Relaxed) || stop_running.load(Ordering::Relaxed)
        });
    }

    #[cfg(feature = "dhat-profiler")]
    drop(profiler);

    Ok(())
}
