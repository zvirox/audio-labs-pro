//! Command-line grammar for `alp-cli`.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Headless tools for Audio Labs Pro.
#[derive(Debug, Parser)]
#[command(
    name = "alp-cli",
    version,
    about = "Decode, inspect, render and play Audio Labs Pro projects"
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

    /// Render a project to WAV or FLAC offline.
    Render {
        /// Project directory or `project.json`.
        project: PathBuf,
        /// Output media path.
        output: PathBuf,
        /// Container: `wav` or `flac`.
        #[arg(long, default_value = "wav")]
        format: String,
        /// Bit depth: `int16`, `int24` or `float32` (WAV only).
        #[arg(long, default_value = "int16")]
        bit_depth: String,
        /// Target sample rate in Hz (defaults to the project rate).
        #[arg(long)]
        sample_rate: Option<u32>,
        /// Dither: `none` or `triangular`.
        #[arg(long, default_value = "none")]
        dither: String,
        /// Title tag.
        #[arg(long)]
        title: Option<String>,
        /// Artist tag.
        #[arg(long)]
        artist: Option<String>,
        /// Album tag.
        #[arg(long)]
        album: Option<String>,
        /// Comment tag.
        #[arg(long)]
        comment: Option<String>,
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

    /// Record an input device to a crash-safe WAV file.
    Record {
        /// Output WAV path.
        output: PathBuf,
        /// Duration in seconds.
        #[arg(long, default_value_t = 5.0)]
        seconds: f64,
        /// Exact input device name (defaults to the host default).
        #[arg(long)]
        device: Option<String>,
        /// Monitor input through the default output device.
        #[arg(long)]
        monitor: bool,
        /// Manifest sync interval in seconds.
        #[arg(long, default_value_t = 10.0)]
        chunk_seconds: f64,
    },

    /// Finish a recording left behind by a forced kill.
    Recover {
        /// Output WAV path used by the interrupted recording.
        output: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Delete a timeline range and write the result to autosave only.
    DeleteRange {
        /// Project directory or `project.json`.
        project: PathBuf,
        /// Range start in seconds.
        start: f64,
        /// Range end in seconds.
        end: f64,
        /// Leave a gap instead of closing it.
        #[arg(long)]
        no_ripple: bool,
    },

    /// Write a crash-recovery autosave without replacing project.json.
    Autosave {
        /// Project directory or `project.json`.
        project: PathBuf,
    },

    /// Restore the latest autosave over project.json.
    RecoverProject {
        /// Project directory or `project.json`.
        project: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Null-test an export against a reference render.
    NullTest {
        /// Reference render, usually a float WAV from `render`.
        reference: PathBuf,
        /// Exported file to compare.
        rendered: PathBuf,
        /// Maximum absolute sample difference (linear, 0..=1).
        #[arg(long, default_value_t = 0.001)]
        tolerance: f32,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}
