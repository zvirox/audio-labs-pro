//! Frequency-response telemetry for the active EQ/dynamics chain.

use alp_dsp::{Coefficients, FilterType};
use serde::{Deserialize, Serialize};

/// One EQ band as configured by the Effects workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiquadBandDto {
    /// Filter shape: `highpass`, `lowpass`, `peaking`, `lowshelf`, `highshelf`,
    /// `notch`, or `bandpass`.
    pub filter_type: String,
    /// Centre or cutoff frequency in Hz.
    pub frequency_hz: f32,
    /// Quality factor.
    pub q: f32,
    /// Peaking/shelving gain in dB.
    pub gain_db: f32,
    /// Whether the band contributes to the response.
    pub enabled: bool,
}

/// Sampled complex response of the active band chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiquadResponseDto {
    /// Log-spaced analysis frequencies in Hz.
    pub frequencies_hz: Vec<f32>,
    /// Combined magnitude in dB at each frequency.
    pub magnitude_db: Vec<f32>,
    /// Combined unwrapped phase in degrees at each frequency.
    pub phase_deg: Vec<f32>,
}

fn filter_type(name: &str) -> FilterType {
    match name.to_lowercase().as_str() {
        "highpass" | "high_pass" | "hp" => FilterType::HighPass,
        "lowpass" | "low_pass" | "lp" => FilterType::LowPass,
        "lowshelf" | "low_shelf" => FilterType::LowShelf,
        "highshelf" | "high_shelf" => FilterType::HighShelf,
        "notch" => FilterType::Notch,
        "bandpass" | "band_pass" => FilterType::BandPass,
        _ => FilterType::Peaking,
    }
}

/// Compute the combined response of enabled bands on a log frequency grid.
///
/// This is a pure function of its inputs; it reads no session state and can be
/// called from the UI command handler without touching the audio thread.
#[must_use]
pub fn compute_biquad_response(
    bands: &[BiquadBandDto],
    sample_rate: u32,
    points: usize,
) -> BiquadResponseDto {
    let points = points.clamp(16, 512);
    let nyquist = (sample_rate as f32 * 0.5).max(1.0);
    let max_hz = nyquist * 0.99;
    let min_hz = 20.0f32.min(max_hz);
    let ratio = (max_hz / min_hz).ln() / (points.saturating_sub(1).max(1) as f32);

    let coefficients: Vec<_> = bands
        .iter()
        .filter(|band| band.enabled)
        .map(|band| {
            Coefficients::design(
                filter_type(&band.filter_type),
                sample_rate,
                band.frequency_hz,
                band.q,
                band.gain_db,
            )
        })
        .collect();

    let mut frequencies_hz = Vec::with_capacity(points);
    let mut magnitude_db = Vec::with_capacity(points);
    let mut phase_deg = Vec::with_capacity(points);

    for index in 0..points {
        let frequency = min_hz * (index as f32 * ratio).exp();
        frequencies_hz.push(frequency);
        let mut magnitude = 1.0f32;
        let mut phase = 0.0f32;
        for coefficient in &coefficients {
            let (db, degrees) = coefficient.response(sample_rate, frequency);
            magnitude *= 10.0f32.powf(db / 20.0);
            phase += degrees;
        }
        magnitude_db.push((20.0 * magnitude.max(1e-6).log10()).clamp(-120.0, 24.0));
        phase_deg.push(phase.clamp(-360.0, 360.0));
    }

    BiquadResponseDto {
        frequencies_hz,
        magnitude_db,
        phase_deg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_bands_produce_unity_response() {
        let bands = [BiquadBandDto {
            filter_type: "lowpass".into(),
            frequency_hz: 500.0,
            q: 0.7,
            gain_db: 0.0,
            enabled: false,
        }];
        let response = compute_biquad_response(&bands, 48_000, 64);
        assert!(response.magnitude_db.iter().all(|db| db.abs() < 0.01));
        assert!(response.phase_deg.iter().all(|deg| deg.abs() < 0.01));
    }

    #[test]
    fn enabled_lowpass_rolls_off_high_frequencies() {
        let bands = [BiquadBandDto {
            filter_type: "lowpass".into(),
            frequency_hz: 1_000.0,
            q: 0.707,
            gain_db: 0.0,
            enabled: true,
        }];
        let response = compute_biquad_response(&bands, 48_000, 128);
        let first = response.magnitude_db[0];
        let last = response.magnitude_db[response.magnitude_db.len() - 1];
        assert!(first.abs() < 1.0, "low end should pass, got {first}");
        assert!(last < -20.0, "high end should roll off, got {last}");
        assert_eq!(response.frequencies_hz.len(), response.phase_deg.len());
    }
}
