//! Reference recording quality gate and acoustics audit for voice cloning.
//!
//! Validates that reference takes meet duration (>= 5.0s), headroom, noise floor,
//! and silence ratio standards, returning clear actionable feedback.

use alp_media::PcmBuffer;
use serde::{Deserialize, Serialize};

/// Detailed quality report for a voice reference recording.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceQualityReport {
    /// Duration of the reference in seconds.
    pub duration_secs: f64,
    /// Audio sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u16,
    /// Peak amplitude in dBFS.
    pub peak_dbfs: f32,
    /// Average RMS amplitude in dBFS.
    pub rms_dbfs: f32,
    /// Whether any samples reached digital full scale (clipping).
    pub clipping_detected: bool,
    /// Estimated background noise floor in dBFS.
    pub noise_floor_dbfs: f32,
    /// Ratio of frames classified as silent or below background threshold (0.0 to 1.0).
    pub silence_ratio: f32,
    /// Whether all mandatory quality gates passed.
    pub passed: bool,
    /// Actionable diagnostic advice for the user.
    pub advice: Vec<String>,
}

impl ReferenceQualityReport {
    /// Minimum required reference speech duration in seconds.
    pub const MIN_DURATION_SECS: f64 = 5.0;
    /// Maximum permitted background noise floor in dBFS.
    pub const MAX_NOISE_FLOOR_DBFS: f32 = -35.0;
    /// Maximum permitted silence ratio.
    pub const MAX_SILENCE_RATIO: f32 = 0.60;
}

/// Analyze a PCM audio buffer and generate a quality report.
#[must_use]
pub fn analyze_reference_quality(buffer: &PcmBuffer) -> ReferenceQualityReport {
    let duration_secs = buffer.duration_secs();
    let sample_rate = buffer.sample_rate();
    let channels = buffer.channels();
    let samples = buffer.samples();

    let mut peak = 0.0f32;
    let mut sum_sq = 0.0f64;
    let mut clipping_samples = 0usize;

    for &s in samples {
        let abs = s.abs();
        if abs > peak {
            peak = abs;
        }
        if abs >= 0.999 {
            clipping_samples += 1;
        }
        sum_sq += f64::from(s * s);
    }

    let clipping_detected = clipping_samples > 0;
    let peak_dbfs = if peak > 1e-6 {
        20.0 * peak.log10()
    } else {
        -120.0
    };

    let rms = if !samples.is_empty() {
        (sum_sq / samples.len() as f64).sqrt() as f32
    } else {
        0.0
    };
    let rms_dbfs = if rms > 1e-6 {
        20.0 * rms.log10()
    } else {
        -120.0
    };

    // Calculate noise floor and silence ratio using 20ms analysis blocks
    let block_size = ((sample_rate as f64 * 0.02) as usize * channels as usize).max(32);
    let mut block_energies = Vec::new();
    let silence_threshold = 10.0f32.powf(-45.0 / 20.0);
    let mut silent_blocks = 0usize;
    let mut total_blocks = 0usize;

    let mut pos = 0;
    while pos + block_size <= samples.len() {
        let slice = &samples[pos..pos + block_size];
        let mut b_sq = 0.0f32;
        for &v in slice {
            b_sq += v * v;
        }
        let b_rms = (b_sq / slice.len() as f32).sqrt();
        block_energies.push(b_rms);
        if b_rms < silence_threshold {
            silent_blocks += 1;
        }
        total_blocks += 1;
        pos += block_size;
    }

    // Estimate noise floor from the 10th percentile lowest-energy block
    let noise_floor_dbfs = if !block_energies.is_empty() {
        block_energies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p10_idx = (block_energies.len() / 10).min(block_energies.len() - 1);
        let val = block_energies[p10_idx];
        if val > 1e-6 {
            20.0 * val.log10()
        } else {
            -100.0
        }
    } else {
        -100.0
    };

    let silence_ratio = if total_blocks > 0 {
        silent_blocks as f32 / total_blocks as f32
    } else {
        0.0
    };

    let mut advice = Vec::new();

    if duration_secs < ReferenceQualityReport::MIN_DURATION_SECS {
        advice.push(format!(
            "Reference is too short ({duration_secs:.1}s). Record at least {:.1}s of speech for voice profile cloning.",
            ReferenceQualityReport::MIN_DURATION_SECS
        ));
    }

    if clipping_detected {
        advice.push(format!(
            "Digital clipping detected ({clipping_samples} peak samples). Lower microphone input gain to avoid distortion."
        ));
    }

    if sample_rate < 24_000 {
        advice.push(format!(
            "Sample rate ({sample_rate} Hz) is below 24 kHz standard. 48 kHz is recommended for crisp acoustic cloning."
        ));
    }

    if noise_floor_dbfs > ReferenceQualityReport::MAX_NOISE_FLOOR_DBFS {
        advice.push(format!(
            "High background noise floor ({noise_floor_dbfs:.1} dBFS). Move to a quieter room or apply the Voice Cleanup preset."
        ));
    }

    if silence_ratio > ReferenceQualityReport::MAX_SILENCE_RATIO {
        advice.push(format!(
            "Audio contains {:.0}% silence. Trim long silent pauses to provide more speech signal.",
            silence_ratio * 100.0
        ));
    }

    let passed = advice.is_empty();

    ReferenceQualityReport {
        duration_secs,
        sample_rate,
        channels,
        peak_dbfs,
        rms_dbfs,
        clipping_detected,
        noise_floor_dbfs,
        silence_ratio,
        passed,
        advice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_reference_passes_quality_gate() {
        let sample_rate = 48_000;
        let mut samples = Vec::new();

        // 0.5s room baseline (-60 dBFS)
        samples.resize(24_000, 0.001);

        // 5.5 seconds of speech tone at -12 dBFS
        let amp = 0.25f32;
        for i in 0..(sample_rate * 5 + 24_000) {
            let s =
                amp * (2.0 * std::f32::consts::PI * 220.0 * (i as f32 / sample_rate as f32)).sin();
            samples.push(s);
        }

        // 0.5s room baseline (-60 dBFS)
        samples.resize(samples.len() + 24_000, 0.001);

        let buf = PcmBuffer::new(sample_rate, 1, samples).expect("buffer");
        let report = analyze_reference_quality(&buf);

        assert!(
            report.passed,
            "clean reference should pass: {:?}",
            report.advice
        );
        assert!(!report.clipping_detected);
        assert!(report.duration_secs >= 5.0);
    }

    #[test]
    fn short_clipping_reference_fails_with_actionable_advice() {
        let sample_rate = 48_000u32;
        // Only 2 seconds of clipped audio (1.0 amp)
        let samples = vec![1.0f32; (sample_rate * 2) as usize];

        let buf = PcmBuffer::new(sample_rate, 1, samples).expect("buffer");
        let report = analyze_reference_quality(&buf);

        assert!(!report.passed);
        assert!(report.clipping_detected);
        assert!(report.duration_secs < 5.0);
        assert!(!report.advice.is_empty());
    }
}
