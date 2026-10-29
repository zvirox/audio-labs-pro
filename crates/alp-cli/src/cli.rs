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
        /// Delivery preset: `podcast`, `audiobook` or `voice-over`.
        #[arg(long)]
        preset: Option<String>,
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

    /// Render each track as a separate stem.
    RenderStems {
        /// Project directory or `project.json`.
        project: PathBuf,
        /// Output directory for stem files.
        #[arg(long)]
        output_dir: Option<PathBuf>,
        /// Delivery preset: `podcast`, `audiobook` or `voice-over`.
        #[arg(long)]
        preset: Option<String>,
        /// Container: `wav` or `flac`.
        #[arg(long, default_value = "wav")]
        format: String,
        /// Bit depth: `int16`, `int24` or `float32` (WAV only).
        #[arg(long, default_value = "int16")]
        bit_depth: String,
        /// Dither: `none` or `triangular`.
        #[arg(long, default_value = "none")]
        dither: String,
        /// Title tag prefix.
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

    /// Measure ITU-R BS.1770 integrated loudness (LUFS).
    Lufs {
        /// Input media file.
        input: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Apply the One-Click Voice Cleanup chain.
    Cleanup {
        /// Input media file.
        input: PathBuf,
        /// Output WAV path.
        output: PathBuf,
        /// Mains hum frequency in Hz (50 or 60).
        #[arg(long, default_value_t = 50.0)]
        mains_freq: f32,
    },

    /// Synthesize speech from text using the TTS engine.
    Tts {
        /// Text string to speak.
        text: String,
        /// Output audio WAV path.
        output: PathBuf,
        /// Target voice identifier (defaults to "aura").
        #[arg(long, default_value = "aura")]
        voice: String,
        /// Speed multiplier (default 1.0).
        #[arg(long, default_value_t = 1.0)]
        speed: f32,
    },

    /// Manage local and downloadable AI models.
    #[command(subcommand)]
    Model(ModelCommand),

    /// Render multi-line scripts to audio.
    #[command(subcommand)]
    Script(ScriptCommand),

    /// Transcribe speech audio into text, SRT, or WebVTT.
    Transcribe {
        /// Input audio file.
        input: PathBuf,
        /// Output file path (defaults to stdout).
        #[arg(long, short)]
        out: Option<PathBuf>,
        /// Export format: "txt", "srt", or "vtt" (defaults to "txt").
        #[arg(long, default_value = "txt")]
        format: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Run Voice Activity Detection on an audio file and output speech intervals.
    Vad {
        /// Input audio file.
        input: PathBuf,
        /// Threshold in dBFS (defaults to -32.0).
        #[arg(long, default_value_t = -32.0, allow_hyphen_values = true)]
        threshold_db: f32,
        /// Hangover duration in milliseconds (defaults to 300).
        #[arg(long, default_value_t = 300)]
        hangover_ms: u32,
        /// Minimum speech duration in milliseconds (defaults to 100).
        #[arg(long, default_value_t = 100)]
        min_speech_ms: u32,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Manage reusable voice profiles, cloning, and design.
    #[command(subcommand)]
    Profile(ProfileCommand),

    /// Scan, list, and inspect CLAP and LV2 audio plugins.
    #[command(subcommand)]
    Plugin(PluginCommand),

    /// Probe system hardware capabilities and model tier recommendations.
    Hardware {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

/// Model manager subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum ModelCommand {
    /// List all models in the catalogue.
    List {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Verify an installed model against its checksum.
    Verify {
        /// Model identifier (e.g. kokoro-82m).
        model: String,
    },
    /// Download model files with resume support and SHA-256 verification.
    Download {
        /// Model identifier (e.g. kitten-nano).
        model: String,
        /// Refuse any network access and fail fast.
        #[arg(long)]
        offline: bool,
    },
    /// Acknowledge and accept the license terms for a model.
    AcceptLicense {
        /// Model identifier.
        model: String,
    },
}

/// Script rendering subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum ScriptCommand {
    /// Render a script file into speech audio.
    Render {
        /// Path to JSON script file.
        script: PathBuf,
        /// Output WAV path (if stitched) or output directory (if stems).
        output: PathBuf,
        /// Stitch lines together into a single audio file instead of exporting individual stems.
        #[arg(long)]
        stitch: bool,
    },
}

/// Voice profile subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum ProfileCommand {
    /// List all installed voice profiles.
    List {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show metadata for a voice profile.
    Info {
        /// Profile identifier.
        id: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Audit a reference recording against cloning quality requirements.
    CheckReference {
        /// Input reference WAV file.
        input: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Design a new voice profile from a natural language prompt.
    Design {
        /// Friendly name for the voice.
        name: String,
        /// Prompt describing the vocal qualities and character.
        #[arg(long)]
        prompt: String,
        /// Comma-separated tags.
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Export a voice profile to an `.alpvoice` package.
    Export {
        /// Profile identifier.
        id: String,
        /// Destination `.alpvoice` file path.
        output: PathBuf,
    },
    /// Import a voice profile from an `.alpvoice` package.
    Import {
        /// Input `.alpvoice` file path.
        input: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Compare reference audio against generated speech.
    Compare {
        /// Reference recording path.
        reference: PathBuf,
        /// Generated speech path.
        generated: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Delete a voice profile from the local library.
    Delete {
        /// Profile identifier.
        id: String,
    },
}

/// Audio plugin subcommands (CLAP and LV2 hosting).
#[derive(Debug, clap::Subcommand)]
pub enum PluginCommand {
    /// Scan directories for installed CLAP and LV2 plugins.
    Scan {
        /// Optional custom directory to scan instead of standard paths.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// List all discovered and virtual audio plugins.
    List {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Inspect a plugin's exposed parameters and metadata.
    Inspect {
        /// Plugin ID or URI.
        id: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}
