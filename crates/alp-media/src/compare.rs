//! Null-test comparison between a reference render and an exported file.
//!
//! Both buffers must already share a sample rate and channel layout. The CLI
//! resamples decoded exports to the reference rate before calling this helper,
//! so a failed comparison reflects audio content, not container conversion.

use crate::error::MediaError;
use crate::pcm::PcmBuffer;

/// Result of subtracting one buffer from another.
#[derive(Debug, Clone, PartialEq)]
pub struct NullReport {
    /// Frames in the reference buffer.
    pub reference_frames: u64,
    /// Frames in the rendered buffer.
    pub rendered_frames: u64,
    /// Frames actually compared (the shorter length).
    pub compared_frames: u64,
    /// Largest absolute sample difference.
    pub max_abs_diff: f32,
    /// RMS of the difference signal.
    pub rms_diff: f64,
}

impl NullReport {
    /// True when lengths match and every sample is within `tolerance`.
    #[must_use]
    pub fn passed(&self, tolerance: f32) -> bool {
        self.reference_frames == self.rendered_frames && self.max_abs_diff <= tolerance
    }
}

/// Compare two same-rate, same-layout buffers sample by sample.
pub fn compare(reference: &PcmBuffer, rendered: &PcmBuffer) -> Result<NullReport, MediaError> {
    if reference.sample_rate() != rendered.sample_rate() {
        return Err(MediaError::InvalidBuffer(format!(
            "sample rates differ: {} vs {}",
            reference.sample_rate(),
            rendered.sample_rate()
        )));
    }
    if reference.channels() != rendered.channels() {
        return Err(MediaError::InvalidBuffer(format!(
            "channel counts differ: {} vs {}",
            reference.channels(),
            rendered.channels()
        )));
    }
    let compared = reference.samples().len().min(rendered.samples().len());
    let mut max_abs_diff = 0.0_f32;
    let mut sum_squares = 0.0_f64;
    for (left, right) in reference.samples()[..compared]
        .iter()
        .zip(&rendered.samples()[..compared])
    {
        let difference = *left - *right;
        max_abs_diff = max_abs_diff.max(difference.abs());
        sum_squares += f64::from(difference) * f64::from(difference);
    }
    let rms_diff = if compared == 0 {
        0.0
    } else {
        (sum_squares / compared as f64).sqrt()
    };
    Ok(NullReport {
        reference_frames: reference.frames(),
        rendered_frames: rendered.frames(),
        compared_frames: compared as u64 / u64::from(reference.channels()),
        max_abs_diff,
        rms_diff,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(samples: Vec<f32>) -> PcmBuffer {
        PcmBuffer::new(48_000, 1, samples).expect("buffer")
    }

    #[test]
    fn identical_buffers_pass() {
        let left = buffer(vec![0.25, -0.5, 0.0]);
        let report = compare(&left, &left).expect("compare");
        assert!(report.passed(0.0));
        assert_eq!(report.compared_frames, 3);
    }

    #[test]
    fn a_small_difference_is_measured() {
        let left = buffer(vec![0.0, 0.0]);
        let right = buffer(vec![0.0005, -0.0005]);
        let report = compare(&left, &right).expect("compare");
        assert!((report.max_abs_diff - 0.0005).abs() < 1e-7);
        assert!(report.passed(0.001));
        assert!(!report.passed(0.0001));
    }

    #[test]
    fn length_mismatch_fails() {
        let left = buffer(vec![0.0, 0.0]);
        let right = buffer(vec![0.0]);
        let report = compare(&left, &right).expect("compare");
        assert!(!report.passed(1.0));
    }

    #[test]
    fn layout_mismatch_is_rejected() {
        let left = buffer(vec![0.0]);
        let right = PcmBuffer::new(44_100, 1, vec![0.0]).expect("buffer");
        assert!(compare(&left, &right).is_err());
    }
}
