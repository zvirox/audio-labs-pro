//! Transcript document model, SRT/VTT/TXT export, and text-based timeline cut mapping.

use alp_core::{CoreError, FrameRange};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

use crate::TranscriptSegment;

/// A formatted transcription document.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Transcript {
    /// Segments in chronological order.
    pub segments: Vec<TranscriptSegment>,
}

impl Transcript {
    /// Create a new empty transcript.
    #[must_use]
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Add a segment to the transcript.
    pub fn add_segment(&mut self, start_frame: u64, end_frame: u64, text: impl Into<String>) {
        self.segments.push(TranscriptSegment {
            start_frame,
            end_frame,
            text: text.into(),
        });
    }

    /// Total duration in seconds based on sample rate.
    #[must_use]
    pub fn duration_secs(&self, sample_rate: u32) -> f64 {
        if sample_rate == 0 {
            return 0.0;
        }
        self.segments
            .last()
            .map_or(0.0, |s| s.end_frame as f64 / sample_rate as f64)
    }

    /// Export transcript as plain text.
    #[must_use]
    pub fn to_plain_text(&self) -> String {
        let mut out = String::new();
        for (i, seg) in self.segments.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(seg.text.trim());
        }
        out
    }

    /// Export transcript as standard SubRip Subtitles (.srt).
    #[must_use]
    pub fn to_srt(&self, sample_rate: u32) -> String {
        let mut out = String::new();
        let sr = sample_rate.max(1) as f64;

        for (index, seg) in self.segments.iter().enumerate() {
            let num = index + 1;
            let start_secs = seg.start_frame as f64 / sr;
            let end_secs = seg.end_frame as f64 / sr;

            let start_tc = format_srt_timecode(start_secs);
            let end_tc = format_srt_timecode(end_secs);

            let _ = writeln!(out, "{num}");
            let _ = writeln!(out, "{start_tc} --> {end_tc}");
            let _ = writeln!(out, "{}\n", seg.text.trim());
        }

        out
    }

    /// Export transcript as standard WebVTT (.vtt).
    #[must_use]
    pub fn to_vtt(&self, sample_rate: u32) -> String {
        let mut out = String::from("WEBVTT\n\n");
        let sr = sample_rate.max(1) as f64;

        for (index, seg) in self.segments.iter().enumerate() {
            let num = index + 1;
            let start_secs = seg.start_frame as f64 / sr;
            let end_secs = seg.end_frame as f64 / sr;

            let start_tc = format_vtt_timecode(start_secs);
            let end_tc = format_vtt_timecode(end_secs);

            let _ = writeln!(out, "{num}");
            let _ = writeln!(out, "{start_tc} --> {end_tc}");
            let _ = writeln!(out, "{}\n", seg.text.trim());
        }

        out
    }

    /// Calculate the timeline frame range to cut/delete corresponding to a segment index.
    #[must_use]
    pub fn cut_range_for_segment(&self, index: usize) -> Option<FrameRange> {
        let seg = self.segments.get(index)?;
        FrameRange::new(seg.start_frame, seg.end_frame).ok()
    }

    /// Parse a SubRip Subtitle (.srt) file into a Transcript.
    pub fn parse_srt(content: &str, sample_rate: u32) -> Result<Self, CoreError> {
        let mut transcript = Self::new();
        let sr = sample_rate.max(1) as f64;

        let blocks: Vec<&str> = content.split("\n\n").collect();
        for block in blocks {
            let lines: Vec<&str> = block
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect();
            if lines.len() < 2 {
                continue;
            }

            // Find the arrow line `00:00:01,000 --> 00:00:03,000`
            let timecode_idx = if lines[0].contains("-->") { 0 } else { 1 };
            if timecode_idx >= lines.len() {
                continue;
            }

            let tc_line = lines[timecode_idx];
            if let Some((start_s, end_s)) = tc_line.split_once("-->") {
                let start_secs = parse_srt_timecode(start_s.trim())?;
                let end_secs = parse_srt_timecode(end_s.trim())?;

                let text_lines = &lines[timecode_idx + 1..];
                let text = text_lines.join(" ");

                let start_frame = (start_secs * sr).round() as u64;
                let end_frame = (end_secs * sr).round() as u64;

                transcript.add_segment(start_frame, end_frame, text);
            }
        }

        Ok(transcript)
    }
}

fn format_srt_timecode(seconds: f64) -> String {
    let total_ms = (seconds.max(0.0) * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

fn format_vtt_timecode(seconds: f64) -> String {
    let total_ms = (seconds.max(0.0) * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{h:02}:{m:02}:{s:02}.{ms:03}")
}

fn parse_srt_timecode(tc: &str) -> Result<f64, CoreError> {
    let parts: Vec<&str> = tc.split(':').collect();
    if parts.len() != 3 {
        return Err(CoreError::validation(format!("invalid timecode: {tc}")));
    }

    let hours: f64 = parts[0]
        .parse()
        .map_err(|e| CoreError::validation(format!("{e}")))?;
    let minutes: f64 = parts[1]
        .parse()
        .map_err(|e| CoreError::validation(format!("{e}")))?;

    let sec_parts: Vec<&str> = parts[2].split([',', '.']).collect();
    let seconds: f64 = sec_parts[0]
        .parse()
        .map_err(|e| CoreError::validation(format!("{e}")))?;
    let ms: f64 = if sec_parts.len() > 1 {
        sec_parts[1]
            .parse()
            .map_err(|e| CoreError::validation(format!("{e}")))?
    } else {
        0.0
    };

    Ok(hours * 3600.0 + minutes * 60.0 + seconds + ms * 0.001)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srt_export_and_parse_round_trip() {
        let mut transcript = Transcript::new();
        transcript.add_segment(0, 48_000, "First sentence.");
        transcript.add_segment(48_000, 96_000, "Second sentence.");

        let srt = transcript.to_srt(48_000);
        assert!(srt.contains("00:00:00,000 --> 00:00:01,000"));
        assert!(srt.contains("00:00:01,000 --> 00:00:02,000"));

        let parsed = Transcript::parse_srt(&srt, 48_000).expect("parse srt");
        assert_eq!(parsed.segments.len(), 2);
        assert_eq!(parsed.segments[0].text, "First sentence.");
        assert_eq!(parsed.segments[1].text, "Second sentence.");
    }

    #[test]
    fn vtt_export_includes_webvtt_header() {
        let mut transcript = Transcript::new();
        transcript.add_segment(0, 24_000, "Hello WebVTT");

        let vtt = transcript.to_vtt(48_000);
        assert!(vtt.starts_with("WEBVTT\n\n"));
        assert!(vtt.contains("00:00:00.000 --> 00:00:00.500"));
    }

    #[test]
    fn cut_range_for_segment_produces_valid_frame_range() {
        let mut transcript = Transcript::new();
        transcript.add_segment(1000, 5000, "Cut this section");

        let range = transcript.cut_range_for_segment(0).expect("cut range");
        assert_eq!(range.start, 1000);
        assert_eq!(range.end, 5000);
    }
}
