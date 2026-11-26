//! Directory batch processing for DSP macro chains.
//!
//! A `MacroChain` is a small ordered list of effect steps. `process_directory`
//! decodes every matching audio file in an input directory, runs the chain
//! offline, and writes a WAV to the output directory with the same stem.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use alp_core::EffectRef;
use alp_dsp::{Effect, EffectChain};
use alp_media::{ExportSettings, PcmBuffer, WavBitDepth, decode, export};
use serde::{Deserialize, Serialize};

use crate::EngineError;
use crate::insert_rack::effect_from_ref;

/// One effect in a macro chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroStep {
    /// Effect id from the built-in catalog (e.g. `compressor`).
    pub effect_id: String,
    /// Parameters keyed by catalog names.
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
}

impl MacroStep {
    /// Convert to an `EffectRef` understood by the insert rack factory.
    #[must_use]
    pub fn to_effect_ref(&self, index: usize) -> EffectRef {
        let mut reference = EffectRef::new(format!("macro-step-{index}"), &self.effect_id);
        reference.params = self.params.clone();
        reference
    }
}

/// Ordered DSP chain applied to every input file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MacroChain {
    /// Steps in processing order.
    pub steps: Vec<MacroStep>,
}

impl MacroChain {
    /// Build an `EffectChain` from the configured steps.
    #[must_use]
    pub fn build(&self) -> EffectChain {
        let mut chain = EffectChain::new();
        for (index, step) in self.steps.iter().enumerate() {
            chain.add(effect_from_ref(&step.to_effect_ref(index)));
        }
        chain
    }
}

/// Request to process a directory of audio files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroRequest {
    /// Source directory.
    pub input_dir: String,
    /// Destination directory for WAV results.
    pub output_dir: String,
    /// Optional filename substring filter.
    #[serde(default)]
    pub pattern: Option<String>,
    /// Whether existing outputs may be replaced.
    #[serde(default)]
    pub overwrite: bool,
    /// Effect steps to apply.
    pub steps: Vec<MacroStep>,
}

/// Outcome for one file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroFileResult {
    /// Source path.
    pub input: String,
    /// Destination path.
    pub output: String,
    /// Whether the file was processed successfully.
    pub ok: bool,
    /// Error message when `ok` is false.
    pub error: Option<String>,
}

/// Aggregate result of a directory batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroReport {
    /// Files considered.
    pub processed: usize,
    /// Successfully written files.
    pub succeeded: usize,
    /// Failed files.
    pub failed: usize,
    /// Per-file outcomes.
    pub files: Vec<MacroFileResult>,
    /// Whether the caller stopped the batch early.
    pub cancelled: bool,
}

fn audio_files(dir: &Path, pattern: Option<&str>) -> Result<Vec<PathBuf>, EngineError> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            let is_audio = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    matches!(
                        extension.to_ascii_lowercase().as_str(),
                        "wav" | "flac" | "mp3" | "ogg" | "aiff"
                    )
                });
            let matches_pattern = pattern.is_none_or(|needle| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains(needle))
            });
            path.is_file() && is_audio && matches_pattern
        })
        .collect();
    files.sort();
    Ok(files)
}

/// Process every matching file in `request.input_dir`.
///
/// The progress callback receives `(completed, total, current_path)` and
/// returns `false` to stop the batch cooperatively.
pub fn process_directory(
    request: &MacroRequest,
    mut progress: impl FnMut(usize, usize, &Path) -> bool,
) -> Result<MacroReport, EngineError> {
    let files = audio_files(Path::new(&request.input_dir), request.pattern.as_deref())?;
    std::fs::create_dir_all(&request.output_dir)?;
    let mut chain = MacroChain {
        steps: request.steps.clone(),
    }
    .build();
    let total = files.len();
    let mut report = MacroReport {
        processed: 0,
        succeeded: 0,
        failed: 0,
        files: Vec::with_capacity(total),
        cancelled: false,
    };

    for (index, input) in files.iter().enumerate() {
        if !progress(index, total, input) {
            report.cancelled = true;
            break;
        }
        let output = Path::new(&request.output_dir).join(format!(
            "{}.wav",
            input
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_else(|| "take".to_owned())
        ));
        if output.exists() && !request.overwrite {
            report.failed += 1;
            report.files.push(MacroFileResult {
                input: input.display().to_string(),
                output: output.display().to_string(),
                ok: false,
                error: Some("output already exists; set overwrite to replace it".into()),
            });
            report.processed += 1;
            continue;
        }
        let outcome = process_one(&mut chain, input, &output);
        match outcome {
            Ok(()) => report.succeeded += 1,
            Err(error) => {
                report.failed += 1;
                report.files.push(MacroFileResult {
                    input: input.display().to_string(),
                    output: output.display().to_string(),
                    ok: false,
                    error: Some(error),
                });
            }
        }
        report.processed += 1;
    }

    Ok(report)
}

fn process_one(chain: &mut EffectChain, input: &Path, output: &Path) -> Result<(), String> {
    let pcm: PcmBuffer = decode(input).map_err(|error| format!("decode failed: {error}"))?;
    let mut pcm = pcm;
    chain.reset();
    chain.prepare(pcm.sample_rate(), pcm.channels());
    chain.process(pcm.samples_mut());
    export(output, &pcm, &ExportSettings::wav(WavBitDepth::Int24))
        .map(|_| ())
        .map_err(|error| format!("export failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alp_media::{ExportSettings, WavBitDepth};
    use std::f32::consts::PI;

    fn write_tone(path: &Path, frames: usize) {
        let samples: Vec<f32> = (0..frames)
            .map(|i| 0.3 * (2.0 * PI * 220.0 * i as f32 / 48_000.0).sin())
            .collect();
        let pcm = PcmBuffer::new(48_000, 1, samples).expect("pcm");
        export(path, &pcm, &ExportSettings::wav(WavBitDepth::Int16)).expect("write tone");
    }

    fn request(input: &Path, output: &Path) -> MacroRequest {
        MacroRequest {
            input_dir: input.display().to_string(),
            output_dir: output.display().to_string(),
            pattern: None,
            overwrite: true,
            steps: vec![MacroStep {
                effect_id: "gain".into(),
                params: BTreeMap::from([("gain".into(), 0.5)]),
            }],
        }
    }

    #[test]
    fn processes_every_matching_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let input = dir.path().join("in");
        let output = dir.path().join("out");
        std::fs::create_dir_all(&input).expect("input dir");
        write_tone(&input.join("a.wav"), 4_800);
        write_tone(&input.join("b.wav"), 4_800);

        let report = process_directory(&request(&input, &output), |_, _, _| true).expect("batch");
        assert_eq!(report.processed, 2);
        assert_eq!(report.succeeded, 2);
        assert!(output.join("a.wav").is_file());
        assert!(output.join("b.wav").is_file());
    }

    #[test]
    fn pattern_filters_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let input = dir.path().join("in");
        let output = dir.path().join("out");
        std::fs::create_dir_all(&input).expect("input dir");
        write_tone(&input.join("keep.wav"), 4_800);
        write_tone(&input.join("skip.wav"), 4_800);
        let mut req = request(&input, &output);
        req.pattern = Some("keep".into());

        let report = process_directory(&req, |_, _, _| true).expect("batch");
        assert_eq!(report.processed, 1);
        assert!(output.join("keep.wav").is_file());
        assert!(!output.join("skip.wav").exists());
    }

    #[test]
    fn progress_callback_can_cancel() {
        let dir = tempfile::tempdir().expect("tempdir");
        let input = dir.path().join("in");
        let output = dir.path().join("out");
        std::fs::create_dir_all(&input).expect("input dir");
        write_tone(&input.join("a.wav"), 4_800);
        write_tone(&input.join("b.wav"), 4_800);

        let mut calls = 0;
        let report = process_directory(&request(&input, &output), |_, _, _| {
            calls += 1;
            calls == 1
        })
        .expect("batch");
        assert!(report.cancelled);
        assert_eq!(report.processed, 1);
    }
}
