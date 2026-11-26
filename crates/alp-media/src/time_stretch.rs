//! WSOLA (Waveform Similarity Overlap-Add) time stretching.
//!
//! Changes duration without changing pitch. Frames are overlap-added at a
//! synthesis hop derived from the stretch ratio; for each frame the input
//! offset is chosen by matching the incoming waveform against the tail of the
//! previously synthesised signal, which avoids phase discontinuities.

use crate::error::MediaError;
use crate::pcm::PcmBuffer;

const FRAME_LEN: usize = 2048;
const ANALYSIS_HOP: usize = FRAME_LEN / 2;
const SEARCH_RANGE: usize = ANALYSIS_HOP / 2;

/// Stretch a PCM buffer by `ratio` (1.0 keeps duration, 1.5 makes it 50% longer).
///
/// Returns a copy of `buffer` when `ratio` is within 1% of unity.
pub fn time_stretch(buffer: &PcmBuffer, ratio: f32) -> Result<PcmBuffer, MediaError> {
    if !ratio.is_finite() || ratio <= 0.0 || ratio > 4.0 {
        return Err(MediaError::Encode(format!(
            "invalid time-stretch ratio {ratio}; expected 0 < ratio <= 4"
        )));
    }
    if (ratio - 1.0).abs() < 0.01 {
        return Ok(buffer.clone());
    }

    let channels = usize::from(buffer.channels());
    let input = buffer.samples();
    let frames = input.len() / channels;
    if frames < FRAME_LEN {
        return Ok(buffer.clone());
    }

    let synthesis_hop = (ANALYSIS_HOP as f32 * ratio).round().max(1.0) as usize;
    let frame_count = 1 + (frames - FRAME_LEN) / ANALYSIS_HOP;
    let out_frames = (frame_count - 1) * synthesis_hop + FRAME_LEN;
    let mut output = vec![0.0f32; out_frames * channels];
    let mut normalization = vec![0.0f32; out_frames];
    let window = hann_window(FRAME_LEN);

    let mut previous: Option<Vec<f32>> = None;
    for frame_index in 0..frame_count {
        let expected = frame_index * ANALYSIS_HOP;
        let offset = match &previous {
            None => expected,
            Some(reference) => best_offset(input, channels, frames, expected, reference),
        };

        let destination = frame_index * synthesis_hop;
        for sample in 0..FRAME_LEN {
            let source_frame = offset + sample;
            if source_frame >= frames || destination + sample >= out_frames {
                continue;
            }
            let weight = window[sample];
            for channel in 0..channels {
                output[(destination + sample) * channels + channel] +=
                    input[source_frame * channels + channel] * weight;
            }
            normalization[destination + sample] += weight;
        }

        let overlap = FRAME_LEN.saturating_sub(synthesis_hop);
        let reference_start = (destination + FRAME_LEN).saturating_sub(overlap);
        if reference_start + overlap <= out_frames {
            let mut reference = Vec::with_capacity(overlap * channels);
            for sample in 0..overlap {
                for channel in 0..channels {
                    reference.push(output[(reference_start + sample) * channels + channel]);
                }
            }
            previous = Some(reference);
        }
    }

    for frame in 0..out_frames {
        let gain = if normalization[frame] > 1e-6 {
            1.0 / normalization[frame]
        } else {
            0.0
        };
        for channel in 0..channels {
            output[frame * channels + channel] *= gain;
        }
    }

    PcmBuffer::new(buffer.sample_rate(), buffer.channels(), output)
}

fn hann_window(length: usize) -> Vec<f32> {
    (0..length)
        .map(|i| {
            0.5
                * (1.0
                    - (std::f32::consts::TAU * i as f32 / length as f32).cos())
        })
        .collect()
}

/// Choose the input offset whose leading samples best match the previous
/// synthesised tail (mono correlation; the same shift is applied to all channels).
fn best_offset(
    input: &[f32],
    channels: usize,
    frames: usize,
    expected: usize,
    reference: &[f32],
) -> usize {
    let overlap_frames = reference.len() / channels;
    if overlap_frames == 0 {
        return expected;
    }
    let mut best = expected;
    let mut best_error = f64::INFINITY;
    let low = expected.saturating_sub(SEARCH_RANGE);
    let high = (expected + SEARCH_RANGE).min(frames.saturating_sub(FRAME_LEN));
    for candidate in low..=high {
        let mut error = 0.0f64;
        for sample in 0..overlap_frames {
            let value = input[(candidate + sample) * channels];
            let target = reference[sample * channels];
            let delta = f64::from(value - target);
            error += delta * delta;
        }
        if error < best_error {
            best_error = error;
            best = candidate;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, frames: usize, frequency: f32) -> PcmBuffer {
        let samples: Vec<f32> = (0..frames)
            .map(|i| 0.4 * (std::f32::consts::TAU * frequency * i as f32 / rate as f32).sin())
            .collect();
        PcmBuffer::new(rate, 1, samples).expect("pcm")
    }

    fn zero_crossings(buffer: &PcmBuffer) -> usize {
        let samples = buffer.samples();
        samples
            .windows(2)
            .filter(|pair| pair[0] * pair[1] < 0.0)
            .count()
    }

    #[test]
    fn unity_ratio_returns_a_copy() {
        let buffer = sine(48_000, 4_096, 220.0);
        let stretched = time_stretch(&buffer, 1.0).expect("stretch");
        assert_eq!(stretched, buffer);
    }

    #[test]
    fn stretch_changes_duration() {
        let buffer = sine(48_000, 24_000, 220.0);
        let longer = time_stretch(&buffer, 1.5).expect("stretch");
        let ratio = longer.duration_secs() / buffer.duration_secs();
        assert!(
            (1.4..=1.6).contains(&ratio),
            "expected ~1.5x duration, got {ratio}"
        );

        let shorter = time_stretch(&buffer, 0.75).expect("stretch");
        let ratio = shorter.duration_secs() / buffer.duration_secs();
        assert!(
            (0.7..=0.8).contains(&ratio),
            "expected ~0.75x duration, got {ratio}"
        );
    }

    #[test]
    fn stretch_preserves_pitch() {
        let buffer = sine(48_000, 24_000, 220.0);
        let stretched = time_stretch(&buffer, 1.25).expect("stretch");
        let original_rate = zero_crossings(&buffer) as f32 / buffer.duration_secs() as f32;
        let stretched_rate = zero_crossings(&stretched) as f32 / stretched.duration_secs() as f32;
        assert!(
            (original_rate * 0.85..=original_rate * 1.15).contains(&stretched_rate),
            "pitch changed: {original_rate} -> {stretched_rate}"
        );
    }

    #[test]
    fn invalid_ratio_is_rejected() {
        let buffer = sine(48_000, 4_096, 220.0);
        assert!(time_stretch(&buffer, 0.0).is_err());
        assert!(time_stretch(&buffer, 5.0).is_err());
    }
}
