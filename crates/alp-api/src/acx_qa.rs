//! ACX audiobook compliance analysis per chapter.
//!
//! Applies the ACX publication targets to each chapter: RMS between -23 and
//! -18 dBFS, true peak at or below -3 dBFS, and noise floor at or below
//! -60 dBFS. The report is plain JSON-serializable data for the UI and MCP.

use crate::analysis_ops::level_stats;
use alp_dsp::measure_true_peak_dbfs;
use serde::{Deserialize, Serialize};

/// Minimum accepted chapter RMS in dBFS.
pub const ACX_RMS_MIN_DBFS: f32 = -23.0;
/// Maximum accepted chapter RMS in dBFS.
pub const ACX_RMS_MAX_DBFS: f32 = -18.0;
/// Maximum accepted true peak in dBFS.
pub const ACX_TRUE_PEAK_MAX_DBFS: f32 = -3.0;
/// Maximum accepted noise floor in dBFS.
pub const ACX_NOISE_FLOOR_MAX_DBFS: f32 = -60.0;

/// One decoded chapter to analyse.
#[derive(Debug, Clone, Copy)]
pub struct AcxChapterInput<'a> {
    /// Display title used in the report.
    pub title: &'a str,
    /// Interleaved f32 samples for this chapter.
    pub samples: &'a [f32],
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u16,
}

/// Compliance result for one chapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcxChapterQa {
    /// Chapter title.
    pub title: String,
    /// Overall RMS in dBFS.
    pub rms_dbfs: f32,
    /// Estimated inter-sample true peak in dBFS.
    pub true_peak_dbfs: f32,
    /// 10th-percentile frame noise floor in dBFS.
    pub noise_floor_dbfs: f32,
    /// Whether the chapter meets all ACX targets.
    pub compliant: bool,
    /// Human-readable reasons the chapter failed.
    pub failures: Vec<String>,
}

/// Batch compliance report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcxQaReport {
    /// Per-chapter results in input order.
    pub chapters: Vec<AcxChapterQa>,
    /// Number of compliant chapters.
    pub compliant_count: usize,
    /// Total number of analysed chapters.
    pub total_count: usize,
    /// Whether every chapter is compliant.
    pub all_compliant: bool,
}

fn rms_dbfs(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return -120.0;
    }
    let sum: f64 = samples
        .iter()
        .map(|sample| f64::from(*sample) * f64::from(*sample))
        .sum();
    let rms = (sum / samples.len() as f64).sqrt();
    if rms <= 1e-9 {
        return -120.0;
    }
    (20.0 * rms.log10()).clamp(-120.0, 6.0) as f32
}

/// Analyse a batch of decoded chapters against the ACX targets.
#[must_use]
pub fn analyze_acx_chapters(chapters: &[AcxChapterInput<'_>]) -> AcxQaReport {
    let results: Vec<AcxChapterQa> = chapters
        .iter()
        .map(|chapter| {
            let rms = rms_dbfs(chapter.samples);
            let peak =
                measure_true_peak_dbfs(chapter.samples, chapter.sample_rate, chapter.channels);
            let floor = level_stats(chapter.samples, chapter.sample_rate, chapter.channels)
                .noise_floor_dbfs;
            let mut failures = Vec::new();
            if rms < ACX_RMS_MIN_DBFS {
                failures.push(format!(
                    "RMS {rms:.1} dBFS is below {ACX_RMS_MIN_DBFS:.0} dBFS"
                ));
            }
            if rms > ACX_RMS_MAX_DBFS {
                failures.push(format!(
                    "RMS {rms:.1} dBFS is above {ACX_RMS_MAX_DBFS:.0} dBFS"
                ));
            }
            if peak > ACX_TRUE_PEAK_MAX_DBFS {
                failures.push(format!(
                    "True peak {peak:.1} dBFS exceeds {ACX_TRUE_PEAK_MAX_DBFS:.0} dBFS"
                ));
            }
            if floor > ACX_NOISE_FLOOR_MAX_DBFS {
                failures.push(format!(
                    "Noise floor {floor:.1} dBFS exceeds {ACX_NOISE_FLOOR_MAX_DBFS:.0} dBFS"
                ));
            }
            AcxChapterQa {
                title: chapter.title.to_owned(),
                rms_dbfs: rms,
                true_peak_dbfs: peak,
                noise_floor_dbfs: floor,
                compliant: failures.is_empty(),
                failures,
            }
        })
        .collect();

    let compliant_count = results.iter().filter(|chapter| chapter.compliant).count();
    let total_count = results.len();
    AcxQaReport {
        chapters: results,
        compliant_count,
        total_count,
        all_compliant: total_count > 0 && compliant_count == total_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn chapter_buffer(duty: f32, amplitude: f32) -> Vec<f32> {
        let rate = 48_000usize;
        let tone_frames = (rate as f32 * duty) as usize;
        let mut samples = Vec::with_capacity(rate);
        for i in 0..rate {
            if i < tone_frames {
                samples.push(amplitude * (2.0 * PI * 1_000.0 * i as f32 / rate as f32).sin());
            } else {
                samples.push(0.0);
            }
        }
        samples
    }

    #[test]
    fn compliant_chapter_passes_all_targets() {
        let samples = chapter_buffer(0.10, 0.35);
        let report = analyze_acx_chapters(&[AcxChapterInput {
            title: "Chapter 1",
            samples: &samples,
            sample_rate: 48_000,
            channels: 1,
        }]);
        assert_eq!(report.total_count, 1);
        assert!(
            report.all_compliant,
            "expected compliant chapter, failures: {:?}",
            report.chapters[0].failures
        );
    }

    #[test]
    fn full_scale_tone_fails_peak_rms_and_floor() {
        let samples: Vec<f32> = (0..48_000)
            .map(|i| (2.0 * PI * 1_000.0 * i as f32 / 48_000.0).sin())
            .collect();
        let report = analyze_acx_chapters(&[AcxChapterInput {
            title: "Loud",
            samples: &samples,
            sample_rate: 48_000,
            channels: 1,
        }]);
        let chapter = &report.chapters[0];
        assert!(!chapter.compliant);
        assert_eq!(
            chapter.failures.len(),
            3,
            "failures: {:?}",
            chapter.failures
        );
    }

    #[test]
    fn empty_batch_is_not_marked_compliant() {
        let report = analyze_acx_chapters(&[]);
        assert_eq!(report.total_count, 0);
        assert!(!report.all_compliant);
    }
}
