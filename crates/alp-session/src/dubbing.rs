//! Dubbing IPC bridge: stem separation, rate fitting and video remux.
//!
//! Keeps `alp-session` as the only crate the desktop shell depends on for
//! these operations. Long model work remains in `alp-infer`, reached through
//! `alp_voice::InferClient`.

use std::path::Path;

use alp_media::{
    ExportSettings, MediaError, PcmBuffer, RemuxReport, RemuxRequest, VideoRemuxer, WavBitDepth,
    decode, export,
};
use alp_voice::{FitPlan, InferClient, fit_take};
use serde::{Deserialize, Serialize};

/// Stem names in the order returned by the inference daemon.
pub const DUB_STEM_NAMES: [&str; 4] = ["vocals", "drums", "bass", "other"];

/// Request to separate a mixed audio file into four stems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StemSeparationRequestDto {
    /// Mixed audio file on disk.
    pub audio_path: String,
    /// Directory that receives the four WAV stems.
    pub output_dir: String,
}

/// Path of one separated stem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StemFileDto {
    /// `vocals`, `drums`, `bass` or `other`.
    pub name: String,
    /// Exported WAV path.
    pub path: String,
}

/// Result of a stem separation request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StemSeparationReport {
    /// Exported stems in `DUB_STEM_NAMES` order.
    pub stems: Vec<StemFileDto>,
    /// Sample rate of the exported stems.
    pub sample_rate: u32,
}

/// Request to fit one rendered take to a dialogue window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateFitRequestDto {
    /// Rendered take on disk.
    pub audio_path: String,
    /// Target window duration in seconds.
    pub target_duration_secs: f64,
    /// Destination WAV path.
    pub output_path: String,
}

/// Result of a rate-fit request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateFitReport {
    /// Destination WAV path.
    pub path: String,
    /// Resulting duration in seconds.
    pub duration_secs: f64,
    /// Plan used for the fit.
    pub plan: FitPlan,
}

fn export_wav(path: &Path, pcm: &PcmBuffer) -> Result<(), String> {
    export(path, pcm, &ExportSettings::wav(WavBitDepth::Int16))
        .map(|_| ())
        .map_err(|error: MediaError| format!("failed to write {}: {error}", path.display()))
}

/// Separate a mixed file into four stems through the local inference daemon.
pub fn separate_stems(request: &StemSeparationRequestDto) -> Result<StemSeparationReport, String> {
    let mixed = decode(Path::new(&request.audio_path))
        .map_err(|error| format!("failed to decode {}: {error}", request.audio_path))?;
    let client = InferClient::new();
    if !client.is_alive() {
        return Err("alp-infer daemon is not running; start it before separating stems".into());
    }
    let status = client
        .status()
        .map_err(|error| format!("inference status failed: {error}"))?;
    if status.active_stem_model.is_none() {
        return Err("no stem model is loaded; call LoadModel demucs-4stem first".into());
    }
    let stems = client
        .separate_stems(&mixed)
        .map_err(|error| format!("stem separation failed: {error}"))?;
    std::fs::create_dir_all(&request.output_dir)
        .map_err(|error| format!("failed to create {}: {error}", request.output_dir))?;
    let mut files = Vec::with_capacity(4);
    for (name, pcm) in DUB_STEM_NAMES.iter().zip(stems.iter()) {
        let path = Path::new(&request.output_dir).join(format!("{name}.wav"));
        export_wav(&path, pcm)?;
        files.push(StemFileDto {
            name: (*name).to_owned(),
            path: path.display().to_string(),
        });
    }
    Ok(StemSeparationReport {
        stems: files,
        sample_rate: stems[0].sample_rate(),
    })
}

/// Fit a rendered take to a target duration and write the result as WAV.
pub fn fit_rate(request: &RateFitRequestDto) -> Result<RateFitReport, String> {
    let take = decode(Path::new(&request.audio_path))
        .map_err(|error| format!("failed to decode {}: {error}", request.audio_path))?;
    let (fitted, plan) = fit_take(&take, request.target_duration_secs)
        .map_err(|error| format!("rate fitting failed: {error}"))?;
    export_wav(Path::new(&request.output_path), &fitted)?;
    Ok(RateFitReport {
        path: request.output_path.clone(),
        duration_secs: fitted.duration_secs(),
        plan,
    })
}

/// Remux a video with a dubbed audio track without re-encoding video.
pub fn remux_video(request: &RemuxRequest) -> Result<RemuxReport, String> {
    VideoRemuxer::new()
        .remux(request)
        .map_err(|error| error.to_string())
}

/// Re-export the remux DTO so the desktop command can expose it directly.
pub use alp_media::RemuxRequest as VideoRemuxRequestDto;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_names_follow_daemon_order() {
        assert_eq!(DUB_STEM_NAMES, ["vocals", "drums", "bass", "other"]);
    }

    #[test]
    fn missing_audio_is_reported() {
        let request = StemSeparationRequestDto {
            audio_path: "/definitely/not/here.wav".into(),
            output_dir: "/tmp/alp-stems".into(),
        };
        assert!(separate_stems(&request).is_err());
    }
}
