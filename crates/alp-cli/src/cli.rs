//! Command-line grammar for `alp-cli`.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Headless tools for Audio Lab Pro.
#[derive(Debug, Parser)]
#[command(
    name = "alp-cli",
    version,
    about = "Decode, inspect, render and play Audio Lab Pro projects"
)]
pub struct Cli {
    /// Tracing filter (defaults to ALP_LOG, then `info`).
    #[arg(long, global = true, value_name = "FILTER")]
    pub log: Option<String>,

    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// One CLI action.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate a deterministic tone/noise WAV fixture.
    Gen {
        /// Output WAV path.
        output: PathBuf,
        /// Duration in seconds.
        #[arg(long, default_value_t = 3600.0)]
        seconds: f64,
        /// Sample rate in Hz.
        #[arg(long, default_value_t = 48_000)]
        sample_rate: u32,
        /// Channel count.
        #[arg(long, default_value_t = 1)]
        channels: u16,
        /// Base tone frequency in Hz.
        #[arg(long, default_value_t = 180.0)]
        frequency: f32,
        /// Peak amplitude (0..=1).
        #[arg(long, default_value_t = 0.4)]
        amplitude: f32,
    },

    /// Decode a file and report its format and levels.
    Decode {
        /// Input media file.
        input: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Build or load the peak pyramid for a file.
    Peaks {
        /// Input media file.
        input: PathBuf,
        /// Cache directory (defaults to `peaks/` beside the file).
        #[arg(long)]
        cache_dir: Option<PathBuf>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Create a new project directory.
    New {
        /// Project directory (usually ending in `.alp`).
        directory: PathBuf,
        /// Project name (defaults to the directory name).
        #[arg(long)]
        name: Option<String>,
        /// Project sample rate.
        #[arg(long, default_value_t = 48_000)]
        sample_rate: u32,
    },

    /// Print a project summary.
    Info {
        /// Project directory or `project.json`.
        project: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Copy a media file into a project and add it as a clip.
    Import {
        /// Project directory or `project.json`.
        project: PathBuf,
        /// Media file to import.
        input: PathBuf,
        /// Track name (defaults to the first track).
        #[arg(long)]
        track: Option<String>,
        /// Timeline position in seconds.
        #[arg(long, default_value_t = 0.0)]
        at: f64,
        /// Display name for the source.
        #[arg(long)]
        name: Option<String>,
    },

    /// Render a project to WAV offline.
    Render {
        /// Project directory or `project.json`.
        project: PathBuf,
        /// Output WAV path.
        output: PathBuf,
        /// Output format: `16`, `24` or `float32`.
        #[arg(long, default_value = "int16")]
        bit_depth: String,
    },

    /// Play a project on the default output device.
    Play {
        /// Project directory or `project.json`.
        project: PathBuf,
        /// Stop after this many seconds (defaults to the whole project).
        #[arg(long)]
        seconds: Option<f64>,
    },

    /// List audio devices.
    Devices {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Record the default input device to a WAV file.
    Record {
        /// Output WAV path.
        output: PathBuf,
        /// Duration in seconds.
        #[arg(long, default_value_t = 5.0)]
        seconds: f64,
    },
}
