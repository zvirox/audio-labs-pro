//! Inter-sample true-peak estimation for mastering compliance.
//!
//! Sample peak alone misses peaks between samples. This module uses Catmull-Rom
//! cubic interpolation at quarter-sample offsets on each channel as a compact,
//! deterministic approximation of oversampled true peak.

/// Oversampling factor used for quarter-sample peak probing.
pub const TRUE_PEAK_OVERSAMPLING: usize = 4;

/// Estimate the inter-sample true peak of an interleaved buffer in dBFS.
///
/// Returns `-120.0` for empty or invalid input. The result is clamped to a
/// display-friendly upper bound of `+6.0 dBFS`.
#[must_use]
pub fn measure_true_peak_dbfs(samples: &[f32], sample_rate: u32, channels: u16) -> f32 {
    if samples.is_empty() || sample_rate == 0 || channels == 0 {
        return -120.0;
    }
    let channels = usize::from(channels);
    let frames = samples.len() / channels;
    if frames == 0 {
        return -120.0;
    }

    let mut peak = 0.0f32;
    for channel in 0..channels.min(2) {
        for frame in 0..frames {
            let index = frame * channels + channel;
            let previous = if frame > 0 {
                samples[index - channels]
            } else {
                samples[index]
            };
            let current = samples[index];
            let next = if frame + 1 < frames {
                samples[index + channels]
            } else {
                current
            };
            let following = if frame + 2 < frames {
                samples[index + 2 * channels]
            } else {
                next
            };

            peak = peak.max(current.abs());
            for step in 1..TRUE_PEAK_OVERSAMPLING {
                let t = step as f32 / TRUE_PEAK_OVERSAMPLING as f32;
                let interpolated = catmull_rom(previous, current, next, following, t);
                peak = peak.max(interpolated.abs());
            }
        }
    }

    if peak <= 0.0 {
        return -120.0;
    }
    (20.0 * peak.log10()).clamp(-120.0, 6.0)
}

/// Catmull-Rom interpolation between `p1` and `p2`.
fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5
        * ((2.0 * p1)
            + (-p0 + p2) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn silence_reports_floor() {
        let buffer = [0.0f32; 64];
        assert_eq!(measure_true_peak_dbfs(&buffer, 48_000, 1), -120.0);
    }

    #[test]
    fn full_scale_sine_reports_near_zero_dbfs() {
        let samples: Vec<f32> = (0..48_000)
            .map(|i| (2.0 * PI * 1_000.0 * i as f32 / 48_000.0).sin())
            .collect();
        let peak = measure_true_peak_dbfs(&samples, 48_000, 1);
        assert!(
            (-0.5..=0.6).contains(&peak),
            "expected near 0 dBFS, got {peak}"
        );
    }

    #[test]
    fn half_scale_sine_is_six_db_lower() {
        let samples: Vec<f32> = (0..48_000)
            .map(|i| 0.5 * (2.0 * PI * 1_000.0 * i as f32 / 48_000.0).sin())
            .collect();
        let peak = measure_true_peak_dbfs(&samples, 48_000, 1);
        assert!(
            (-7.0..=-5.0).contains(&peak),
            "expected about -6 dBFS, got {peak}"
        );
    }
}
