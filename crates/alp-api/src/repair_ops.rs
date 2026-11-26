//! Automation wrappers for offline spectral repair and time stretching.
//!
//! These operate on decoded PCM buffers through the same DSP implementations
//! used by the real-time engine, but run entirely on an automation worker.

use alp_dsp::{Effect, HarmonicSuppressor, SpectralNotchFilter};
use alp_media::{ExportSettings, WavBitDepth, decode, export, time_stretch};
use serde::{Deserialize, Serialize};

use crate::{ApiError, PermissionTier, session::ApiSession};

/// Request for an offline spectral repair pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralRepairRequestDto {
    /// Source audio file inside the workspace.
    pub input_path: String,
    /// Destination WAV path inside the workspace.
    pub output_path: String,
    /// `notch` or `harmonic`.
    pub mode: String,
    /// Notch centre frequency in Hz.
    pub center_hz: f32,
    /// Harmonic fundamental in Hz.
    pub fundamental_hz: f32,
    /// Number of overtones including the fundamental.
    pub harmonics: usize,
    /// Bandwidth quality factor.
    pub bandwidth_q: f32,
    /// Maximum attenuation in dB (positive).
    pub reduction_db: f32,
}

/// Result of an offline spectral repair pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralRepairReport {
    /// Destination WAV path.
    pub path: String,
    /// Applied repair mode.
    pub mode: String,
    /// Number of processed frames.
    pub frames: u64,
    /// Sample rate of the output file.
    pub sample_rate: u32,
}

/// Request for an offline WSOLA time stretch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeStretchRequestDto {
    /// Source audio file inside the workspace.
    pub input_path: String,
    /// Destination WAV path inside the workspace.
    pub output_path: String,
    /// Target duration multiplier (0 < ratio <= 4).
    pub ratio: f32,
}

/// Result of an offline WSOLA time stretch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeStretchReport {
    /// Destination WAV path.
    pub path: String,
    /// Applied ratio.
    pub ratio: f32,
    /// Resulting duration in seconds.
    pub duration_secs: f64,
}

fn repair_effect(request: &SpectralRepairRequestDto) -> Result<Box<dyn Effect>, ApiError> {
    if !request.reduction_db.is_finite() || request.reduction_db < 0.0 {
        return Err(ApiError::Validation(
            "reduction_db must be non-negative and finite".into(),
        ));
    }
    if request.bandwidth_q <= 0.0 || !request.bandwidth_q.is_finite() {
        return Err(ApiError::Validation(
            "bandwidth_q must be positive and finite".into(),
        ));
    }
    match request.mode.to_ascii_lowercase().as_str() {
        "notch" => {
            if request.center_hz <= 0.0 || !request.center_hz.is_finite() {
                return Err(ApiError::Validation("center_hz must be positive".into()));
            }
            Ok(Box::new(SpectralNotchFilter::new(
                request.center_hz,
                request.bandwidth_q,
                request.reduction_db,
            )))
        }
        "harmonic" => {
            if request.fundamental_hz <= 0.0 || !request.fundamental_hz.is_finite() {
                return Err(ApiError::Validation(
                    "fundamental_hz must be positive".into(),
                ));
            }
            if request.harmonics == 0 || request.harmonics > 64 {
                return Err(ApiError::Validation("harmonics must be in 1..=64".into()));
            }
            Ok(Box::new(HarmonicSuppressor::new(
                request.fundamental_hz,
                request.harmonics,
                request.bandwidth_q,
                request.reduction_db,
            )))
        }
        other => Err(ApiError::Validation(format!(
            "unknown repair mode '{other}'; expected notch or harmonic"
        ))),
    }
}

impl ApiSession {
    /// Apply a narrowband or harmonic spectral repair to an audio file.
    pub fn dsp_spectral_repair(
        &mut self,
        request: &SpectralRepairRequestDto,
    ) -> Result<SpectralRepairReport, ApiError> {
        self.check_permission(PermissionTier::Export)?;
        let mut effect = repair_effect(request)?;
        let input = self.sandbox.validate_path(&request.input_path)?;
        let output = self.sandbox.validate_path(&request.output_path)?;
        let mut pcm = decode(&input)?;
        effect.prepare(pcm.sample_rate(), pcm.channels());
        for chunk in pcm.samples_mut().chunks_mut(4096) {
            effect.process(chunk);
        }
        let frames = pcm.frames();
        let sample_rate = pcm.sample_rate();
        export(&output, &pcm, &ExportSettings::wav(WavBitDepth::Int16))?;
        Ok(SpectralRepairReport {
            path: output.display().to_string(),
            mode: request.mode.to_ascii_lowercase(),
            frames,
            sample_rate,
        })
    }

    /// Stretch an audio file with WSOLA without changing pitch.
    pub fn dsp_time_stretch(
        &mut self,
        request: &TimeStretchRequestDto,
    ) -> Result<TimeStretchReport, ApiError> {
        self.check_permission(PermissionTier::Export)?;
        if !request.ratio.is_finite() || request.ratio <= 0.0 || request.ratio > 4.0 {
            return Err(ApiError::Validation(
                "ratio must satisfy 0 < ratio <= 4".into(),
            ));
        }
        let input = self.sandbox.validate_path(&request.input_path)?;
        let output = self.sandbox.validate_path(&request.output_path)?;
        let pcm = decode(&input)?;
        let stretched = time_stretch(&pcm, request.ratio)?;
        let duration_secs = stretched.duration_secs();
        export(
            &output,
            &stretched,
            &ExportSettings::wav(WavBitDepth::Int16),
        )?;
        Ok(TimeStretchReport {
            path: output.display().to_string(),
            ratio: request.ratio,
            duration_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditLogger, WorkspaceSandbox};
    use alp_core::{Project, ProjectPaths};
    use alp_media::write_test_tone;
    use tempfile::tempdir;

    fn session_in(dir: &std::path::Path) -> ApiSession {
        let paths = ProjectPaths::new(dir.join("proj.alp"));
        paths.ensure_layout().expect("layout");
        let sandbox = WorkspaceSandbox::new([dir]).expect("sandbox");
        let audit = AuditLogger::new(dir.join("audit.jsonl")).expect("audit");
        ApiSession::new(
            Project::new("Repair Test").expect("project"),
            paths,
            PermissionTier::Export,
            sandbox,
            audit,
            "agent",
        )
        .expect("session")
    }

    #[test]
    fn notch_repair_writes_a_wav() {
        let dir = tempdir().expect("tempdir");
        let input = dir.path().join("tone.wav");
        let output = dir.path().join("notched.wav");
        write_test_tone(&input, 0.1, 48_000, 1, 1_000.0, 0.3).expect("write tone");
        let mut session = session_in(dir.path());
        let report = session
            .dsp_spectral_repair(&SpectralRepairRequestDto {
                input_path: input.display().to_string(),
                output_path: output.display().to_string(),
                mode: "notch".into(),
                center_hz: 1_000.0,
                fundamental_hz: 220.0,
                harmonics: 4,
                bandwidth_q: 12.0,
                reduction_db: 24.0,
            })
            .expect("notch repair");
        assert_eq!(report.mode, "notch");
        assert!(output.is_file());
    }

    #[test]
    fn harmonic_repair_writes_a_wav() {
        let dir = tempdir().expect("tempdir");
        let input = dir.path().join("tone.wav");
        let output = dir.path().join("harmonic.wav");
        write_test_tone(&input, 0.1, 48_000, 1, 220.0, 0.3).expect("write tone");
        let mut session = session_in(dir.path());
        let report = session
            .dsp_spectral_repair(&SpectralRepairRequestDto {
                input_path: input.display().to_string(),
                output_path: output.display().to_string(),
                mode: "harmonic".into(),
                center_hz: 1_000.0,
                fundamental_hz: 220.0,
                harmonics: 4,
                bandwidth_q: 12.0,
                reduction_db: 24.0,
            })
            .expect("harmonic repair");
        assert_eq!(report.mode, "harmonic");
        assert!(output.is_file());
    }

    #[test]
    fn time_stretch_writes_a_wav() {
        let dir = tempdir().expect("tempdir");
        let input = dir.path().join("tone.wav");
        let output = dir.path().join("stretched.wav");
        write_test_tone(&input, 0.2, 48_000, 1, 440.0, 0.3).expect("write tone");
        let mut session = session_in(dir.path());
        let report = session
            .dsp_time_stretch(&TimeStretchRequestDto {
                input_path: input.display().to_string(),
                output_path: output.display().to_string(),
                ratio: 1.5,
            })
            .expect("time stretch");
        assert!(output.is_file());
        assert!((report.ratio - 1.5).abs() < 1e-6);
    }

    #[test]
    fn invalid_repair_mode_is_rejected() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path());
        let error = session
            .dsp_spectral_repair(&SpectralRepairRequestDto {
                input_path: "in.wav".into(),
                output_path: "out.wav".into(),
                mode: "magic".into(),
                center_hz: 1_000.0,
                fundamental_hz: 220.0,
                harmonics: 4,
                bandwidth_q: 12.0,
                reduction_db: 24.0,
            })
            .expect_err("unknown mode");
        assert!(matches!(error, ApiError::Validation(_)));
    }
}
