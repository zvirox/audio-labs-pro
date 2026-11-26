//! Video demux/remux helpers for the dubbing pipeline.
//!
//! Audio-only decoding remains in `alp-media::decode`; this module handles the
//! video container side by invoking the external `ffmpeg`/`ffprobe` binaries
//! with explicit argument vectors (no shell interpolation).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use alp_core::write_atomic;
use serde::{Deserialize, Serialize};

use crate::error::MediaError;
use crate::pcm::PcmBuffer;

/// Request to replace or add a dubbed audio track without re-encoding video.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemuxRequest {
    /// Source video (MP4/MKV).
    pub video_path: String,
    /// dubbed audio track; when omitted, the video's own audio is used.
    pub audio_path: Option<String>,
    /// Destination video path.
    pub output_path: String,
    /// Optional subtitle path preserved as a side file (not burned in).
    pub subtitle_path: Option<String>,
}

/// Result of a remux operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemuxReport {
    /// Destination path.
    pub output_path: String,
    /// Source video duration in seconds when probe succeeded.
    pub duration_secs: Option<f64>,
    /// Whether video was stream-copied.
    pub video_stream_copied: bool,
    /// Audio codec used for the output track.
    pub audio_codec: String,
}

/// Basic stream information from `ffprobe`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoProbe {
    /// Container duration in seconds.
    pub duration_secs: f64,
    /// Container format name, when available.
    pub format_name: String,
}

/// Thin wrapper around the external FFmpeg toolchain.
#[derive(Debug, Clone)]
pub struct VideoRemuxer {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
}

impl Default for VideoRemuxer {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoRemuxer {
    /// Create a remuxer using `ALP_FFMPEG` / `ALP_FFPROBE` overrides or PATH.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ffmpeg: std::env::var_os("ALP_FFMPEG")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("ffmpeg")),
            ffprobe: std::env::var_os("ALP_FFPROBE")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("ffprobe")),
        }
    }

    /// Whether both external tools are reachable.
    #[must_use]
    pub fn is_available(&self) -> bool {
        command_available(&self.ffmpeg) && command_available(&self.ffprobe)
    }

    /// Probe a media file's duration and format.
    pub fn probe(&self, path: impl AsRef<Path>) -> Result<VideoProbe, MediaError> {
        let output = Command::new(&self.ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration,format_name",
                "-of",
                "default=noprint_wrappers=1:nokey=0",
            ])
            .arg(path.as_ref())
            .output()
            .map_err(|error| MediaError::Decode(format!("ffprobe failed to start: {error}")))?;
        if !output.status.success() {
            return Err(MediaError::Decode(format!(
                "ffprobe rejected {}: {}",
                path.as_ref().display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut duration_secs = 0.0;
        let mut format_name = String::new();
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("duration=") {
                duration_secs = value.trim().parse().unwrap_or(0.0);
            } else if let Some(value) = line.strip_prefix("format_name=") {
                format_name = value.trim().to_owned();
            }
        }
        Ok(VideoProbe {
            duration_secs,
            format_name,
        })
    }

    /// Extract a video's audio track to a temporary WAV and decode it.
    pub fn extract_audio(&self, video_path: impl AsRef<Path>) -> Result<PcmBuffer, MediaError> {
        let temp = temp_target(&video_path.as_ref().with_extension("extracted.wav"));
        let result = Command::new(&self.ffmpeg)
            .args(["-y", "-v", "error", "-i"])
            .arg(video_path.as_ref())
            .args(["-vn", "-ac", "2", "-ar", "48000", "-c:a", "pcm_s16le"])
            .arg(&temp)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| MediaError::Decode(format!("ffmpeg failed to start: {error}")))?;
        if !result.status.success() {
            let _ = std::fs::remove_file(&temp);
            return Err(MediaError::Decode(format!(
                "audio extraction failed: {}",
                String::from_utf8_lossy(&result.stderr).trim()
            )));
        }
        let pcm = crate::decode::decode(&temp);
        let _ = std::fs::remove_file(&temp);
        pcm
    }

    /// Remux video with a new audio track using `-c:v copy`.
    pub fn remux(&self, request: &RemuxRequest) -> Result<RemuxReport, MediaError> {
        let probe = self.probe(&request.video_path).ok();
        let temp = temp_target(Path::new(&request.output_path));
        let args = build_remux_args(request, &temp);
        let result = Command::new(&self.ffmpeg)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| MediaError::Encode(format!("ffmpeg failed to start: {error}")))?;
        if !result.status.success() {
            let _ = std::fs::remove_file(&temp);
            return Err(MediaError::Encode(format!(
                "remux failed: {}",
                String::from_utf8_lossy(&result.stderr).trim()
            )));
        }
        std::fs::rename(&temp, &request.output_path).map_err(|error| {
            let _ = std::fs::remove_file(&temp);
            MediaError::Encode(format!("failed to place remuxed video: {error}"))
        })?;
        if let Some(subtitle) = &request.subtitle_path {
            let side = Path::new(&request.output_path).with_extension("srt");
            write_atomic(&side, &std::fs::read(subtitle).map_err(MediaError::Io)?)?;
        }
        Ok(RemuxReport {
            output_path: request.output_path.clone(),
            duration_secs: probe.map(|info| info.duration_secs),
            video_stream_copied: true,
            audio_codec: "aac".to_owned(),
        })
    }
}

fn command_available(binary: &Path) -> bool {
    Command::new(binary)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn temp_target(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "remux.mp4".to_owned());
    path.with_file_name(format!(".{name}.part-{}", std::process::id()))
}

/// Build the explicit ffmpeg argument vector (testable without spawning).
fn build_remux_args(request: &RemuxRequest, temp: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        "-y".into(),
        "-v".into(),
        "error".into(),
        "-i".into(),
        request.video_path.clone().into(),
    ];
    if let Some(audio) = &request.audio_path {
        args.push("-i".into());
        args.push(audio.clone().into());
    }
    args.extend([
        "-map".into(),
        "0:v:0".into(),
        "-map".into(),
        if request.audio_path.is_some() {
            "1:a:0".into()
        } else {
            "0:a:0?".into()
        },
        "-c:v".into(),
        "copy".into(),
        "-c:a".into(),
        "aac".into(),
        "-shortest".into(),
    ]);
    args.push(temp.as_os_str().to_owned());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remux_args_keep_video_stream_copy() {
        let request = RemuxRequest {
            video_path: "/tmp/in.mp4".into(),
            audio_path: Some("/tmp/dub.wav".into()),
            output_path: "/tmp/out.mp4".into(),
            subtitle_path: None,
        };
        let args = build_remux_args(&request, Path::new("/tmp/out.part.mp4"));
        let text: Vec<String> = args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(text.windows(2).any(|w| w[0] == "-c:v" && w[1] == "copy"));
        assert!(text.windows(2).any(|w| w[0] == "-map" && w[1] == "1:a:0"));
    }

    #[test]
    fn ffmpeg_availability_is_reported() {
        let remuxer = VideoRemuxer::new();
        let available = remuxer.is_available();
        assert_eq!(
            available,
            remuxer.is_available(),
            "availability must be stable"
        );
    }
}
