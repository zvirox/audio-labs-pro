//! A/B comparison tool between reference voice recordings and synthesized output.
//!
//! Provides objective measurements of level, duration, and spectral balance.

use alp_media::PcmBuffer;
use serde::{Deserialize, Serialize};

/// Objective comparison metrics between a reference sample and a synthesized take.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceComparison {
    /// Reference duration in seconds.
    pub ref_duration_secs: f64,
    /// Generated duration in seconds.
    pub gen_duration_secs: f64,
    /// Duration difference (gen - ref) in seconds.
    pub duration_diff_secs: f64,
    /// Reference RMS level in dBFS.
    pub ref_rms_dbfs: f32,
    /// Generated RMS level in dBFS.
    pub gen_rms_dbfs: f32,
    /// RMS level difference in dB.
    pub level_diff_db: f32,
    /// Reference zero-crossing rate (indicative of fundamental and brightness).
    pub ref_zcr: f32,
    /// Generated zero-crossing rate.
    pub gen_zcr: f32,
    /// Estimated timbre closeness score from 0.0 (unrelated) to 1.0 (identical acoustics).
    pub similarity_score: f32,
}

/// Compare a reference audio recording against a generated sample.
#[must_use]
pub fn compare_audio(reference: &PcmBuffer, generated: &PcmBuffer) -> VoiceComparison {
    let ref_duration = reference.duration_secs();
    let gen_duration = generated.duration_secs();
    let duration_diff = gen_duration - ref_duration;

    let (ref_rms, ref_zcr) = calculate_acoustics(reference.samples());
    let (gen_rms, gen_zcr) = calculate_acoustics(generated.samples());

    let ref_rms_dbfs = if ref_rms > 1e-6 {
        20.0 * ref_rms.log10()
    } else {
        -120.0
    };
    let gen_rms_dbfs = if gen_rms > 1e-6 {
        20.0 * gen_rms.log10()
    } else {
        -120.0
    };
    let level_diff_db = gen_rms_dbfs - ref_rms_dbfs;

    // Similarity score based on normalized distance across level, length ratio, and zero-crossings
    let dur_ratio =
        (gen_duration / ref_duration.max(0.01)).min(ref_duration / gen_duration.max(0.01)) as f32;
    let level_match = (1.0 - (level_diff_db.abs() / 20.0)).clamp(0.0, 1.0);
    let zcr_match = (1.0 - ((gen_zcr - ref_zcr).abs() / 0.2).min(1.0)).clamp(0.0, 1.0);

    let similarity_score = (dur_ratio * 0.3 + level_match * 0.4 + zcr_match * 0.3).clamp(0.0, 1.0);

    VoiceComparison {
        ref_duration_secs: ref_duration,
        gen_duration_secs: gen_duration,
        duration_diff_secs: duration_diff,
        ref_rms_dbfs,
        gen_rms_dbfs,
        level_diff_db,
        ref_zcr,
        gen_zcr,
        similarity_score,
    }
}

fn calculate_acoustics(samples: &[f32]) -> (f32, f32) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }

    let mut sum_sq = 0.0f32;
    let mut zc = 0usize;
    let mut prev = samples[0];

    for &s in samples {
        sum_sq += s * s;
        if (s >= 0.0 && prev < 0.0) || (s < 0.0 && prev >= 0.0) {
            zc += 1;
        }
        prev = s;
    }

    let rms = (sum_sq / samples.len() as f32).sqrt();
    let zcr = zc as f32 / samples.len() as f32;
    (rms, zcr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_audio_yields_high_similarity() {
        let sample_rate = 48_000;
        let mut samples = Vec::new();
        for i in 0..sample_rate {
            let s =
                0.2 * (2.0 * std::f32::consts::PI * 440.0 * (i as f32 / sample_rate as f32)).sin();
            samples.push(s);
        }

        let b1 = PcmBuffer::new(sample_rate, 1, samples.clone()).expect("b1");
        let b2 = PcmBuffer::new(sample_rate, 1, samples).expect("b2");

        let comp = compare_audio(&b1, &b2);
        assert!((comp.duration_diff_secs).abs() < 1e-4);
        assert!((comp.level_diff_db).abs() < 1e-4);
        assert!(comp.similarity_score > 0.95);
    }
}
