//! Phase 0 audio spike: cpal playback/recording with a lock-free ring buffer,
//! symphonia decoding and a peak-pyramid cache.
//!
//! Modes:
//!   devices
//!   play <seconds>
//!   record <out.wav> <seconds>
//!   gen <out.wav> <seconds>      generate a test tone WAV
//!   peaks <in.wav> <cache.bin>    decode + build + cache pyramid, then reload

mod devices;
mod peaks;
mod wavgen;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("");

    let result = match mode {
        "devices" => devices::list(),
        "play" => devices::play(secs(&args, 2, 10.0)),
        "record" => match args.get(2) {
            Some(path) => devices::record(path, secs(&args, 3, 5.0)),
            None => Err("usage: audio-spike record <out.wav> [seconds]".into()),
        },
        "gen" => match args.get(2) {
            Some(path) => wavgen::generate(path, secs(&args, 3, 3600.0)),
            None => Err("usage: audio-spike gen <out.wav> [seconds]".into()),
        },
        "peaks" => match (args.get(2), args.get(3)) {
            (Some(input), Some(cache)) => peaks::run(input, cache),
            _ => Err("usage: audio-spike peaks <in.wav> <cache.bin>".into()),
        },
        _ => Err("unknown mode; see source header".into()),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn secs(args: &[String], index: usize, default: f32) -> f32 {
    args.get(index)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
