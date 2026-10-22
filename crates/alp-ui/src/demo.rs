//! Synthetic demo media for the Phase 2 shell.
//!
//! No decoder or model is required to open the window. The peaks are generated
//! once with the same streaming builder used for real files, then converted to
//! the toolkit display format. `ALP_DEMO_SECONDS` can request a one-hour source
//! for the 60 fps acceptance run.

use std::sync::Arc;

use alp_media::{PeakBuilder, PeakPyramid};
use alp_ui_kit::waveform::{PeakLevel, WaveformPeaks, WaveformWidget};

/// Default demo length in seconds.
pub const DEFAULT_SECONDS: u32 = 600;
/// Largest demo length accepted from the environment.
pub const MAX_SECONDS: u32 = 3_600;

/// Build peaks at 48 kHz mono.
#[must_use]
pub fn synth_peaks(seconds: u32) -> Arc<WaveformPeaks> {
    let seconds = seconds.clamp(1, MAX_SECONDS);
    let sample_rate = 48_000u32;
    let mut builder = PeakBuilder::default();
    let chunk_frames = 4_800usize;
    let lut: Vec<f32> = (0..4096)
        .map(|index| (std::f32::consts::TAU * index as f32 / 4096.0).sin())
        .collect();

    let chunks_per_second = sample_rate as usize / chunk_frames;
    for second in 0..seconds {
        for chunk_index in 0..chunks_per_second {
            let mut chunk = Vec::with_capacity(chunk_frames);
            for frame in 0..chunk_frames {
                let offset = chunk_index * chunk_frames + frame;
                let t = second as f32 + offset as f32 / sample_rate as f32;
                let phrase = (t * 0.22 * 4096.0) as usize % 4096;
                let tremolo = 0.55 + 0.45 * lut[(t * 11.0 * 4096.0) as usize % 4096];
                let carrier = lut[(t * 220.0 * 4096.0) as usize % 4096] * 0.60
                    + lut[(t * 440.0 * 4096.0) as usize % 4096] * 0.25
                    + lut[(t * 97.0 * 4096.0) as usize % 4096] * 0.15;
                let envelope = (lut[phrase] * 0.5 + 0.5).powf(0.6);
                let sample = (carrier * envelope * tremolo).clamp(-1.0, 1.0);
                chunk.push(sample);
            }
            // The builder only errors on malformed chunks or channel changes.
            builder
                .push_chunk(&chunk, 1)
                .expect("synthetic chunk is frame-aligned");
        }
    }

    let pyramid = builder.finish(sample_rate).expect("synthetic pyramid");
    Arc::new(convert(&pyramid))
}

/// Convert an `alp-media` pyramid into the toolkit display type.
#[must_use]
pub fn convert(pyramid: &PeakPyramid) -> WaveformPeaks {
    let levels = pyramid
        .levels()
        .iter()
        .map(|level| {
            PeakLevel::new(level.bucket_samples(), level.pairs().to_vec())
                .expect("alp-media levels always have a positive bucket")
        })
        .collect();
    WaveformPeaks::new(pyramid.sample_rate(), pyramid.frames(), levels)
        .expect("alp-media pyramids are valid")
}

/// Demo length from the environment.
#[must_use]
pub fn configured_seconds() -> u32 {
    std::env::var("ALP_DEMO_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .map_or(DEFAULT_SECONDS, |seconds| seconds.clamp(1, MAX_SECONDS))
}

/// Default waveform window.
#[must_use]
pub fn waveform(seconds: u32, colors: alp_ui_kit::ThemeColors) -> (WaveformWidget, u64) {
    let peaks = synth_peaks(seconds);
    let frames = peaks.frames();
    (WaveformWidget::new(peaks, colors), frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_peaks_have_three_levels_and_frames() {
        let peaks = synth_peaks(2);
        assert_eq!(peaks.sample_rate(), 48_000);
        assert_eq!(peaks.frames(), 96_000);
        assert_eq!(peaks.levels().len(), 3);
        assert!(peaks.levels()[0].bucket_samples() < peaks.levels()[2].bucket_samples());
    }

    #[test]
    fn conversion_preserves_shape() {
        let peaks = synth_peaks(1);
        assert!(peaks.duration_secs() >= 0.99);
        assert!(!peaks.levels()[0].is_empty());
        assert!(peaks.bytes() > 0);
    }
}
