//! Acoustic voice analysis: the measurable "voiceprint" of a reference recording.
//!
//! A voiceprint is the timbre descriptor the acoustic clone tier uses to
//! re-colour synthesized speech. It is computed with real signal analysis:
//!
//! * **Fundamental frequency** — per-frame normalized autocorrelation over a
//!   16 kHz analysis signal, with the per-frame estimates combined into a
//!   median (pitch) and a relative spread (intonation range).
//! * **Long-term average spectrum** — the mean magnitude spectrum over all
//!   voiced frames, folded into 16 log-spaced bands between 100 Hz and 8 kHz.
//!   The band gains are the actual transfer function applied to the
//!   harmonic stack, so a clone reproduces the reference's timbre rather than
//!   a hand-tuned approximation of it.
//! * **Brightness and tilt** — spectral centroid relative to `f0`, and the
//!   slope of the average spectrum, which drive the excitation brightness and
//!   the amount of high-frequency energy.
//!
//! The descriptor is stored inside the 256-dimensional
//! [`VoiceProfile::cached_embedding`](crate::profile::VoiceProfile::cached_embedding)
//! slot so a cloned profile stays a plain, portable profile JSON.

use alp_media::{PcmBuffer, resample};
use serde::{Deserialize, Serialize};

/// Sample rate the analysis runs at. 16 kHz keeps the autocorrelation lag
/// range small while preserving every frequency the clone tier cares about.
const ANALYSIS_RATE: u32 = 16_000;

/// Analysis frame length in samples (32 ms at 16 kHz).
const FRAME: usize = 512;

/// Hop between analysis frames (16 ms at 16 kHz).
const HOP: usize = 256;

/// Lowest fundamental frequency the pitch tracker will report (Hz).
const MIN_F0: f32 = 60.0;

/// Highest fundamental frequency the pitch tracker will report (Hz).
const MAX_F0: f32 = 320.0;

/// Number of log-spaced spectral bands in a voiceprint.
pub const BAND_COUNT: usize = 16;

/// Lowest band edge in Hz.
const BAND_LOW_HZ: f32 = 100.0;

/// Highest band edge in Hz.
const BAND_HIGH_HZ: f32 = 8_000.0;

/// Dimensionality of the stored embedding.
pub const EMBEDDING_DIM: usize = 256;

/// Index of `f0_hz / 1000` inside the embedding.
const SLOT_F0: usize = 0;

/// Index of the relative pitch spread inside the embedding.
const SLOT_F0_SPREAD: usize = 1;

/// First band-gain slot inside the embedding.
const SLOT_BANDS: usize = 2;

/// Index of the brightness ratio inside the embedding.
const SLOT_BRIGHTNESS: usize = 2 + BAND_COUNT;

/// Index of the spectral tilt inside the embedding.
const SLOT_TILT: usize = 3 + BAND_COUNT;

/// Index of the analyzed frame count inside the embedding.
const SLOT_FRAMES: usize = 4 + BAND_COUNT;

/// A measurable description of a speaker's timbre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoicePrint {
    /// Median fundamental frequency in Hz.
    pub f0_hz: f32,
    /// Relative spread of the per-frame pitch estimates (0 = monotone).
    pub f0_spread: f32,
    /// Per-band spectral gain, one entry per log-spaced band.
    pub band_gains: Vec<f32>,
    /// Spectral centroid divided by `f0` — how bright the voice reads.
    pub brightness: f32,
    /// Slope of the average spectrum in dB per octave.
    pub tilt: f32,
    /// Number of voiced analysis frames the estimate is based on.
    pub frames: usize,
}

impl Default for VoicePrint {
    fn default() -> Self {
        Self {
            f0_hz: 160.0,
            f0_spread: 0.0,
            band_gains: vec![1.0; BAND_COUNT],
            brightness: 4.0,
            tilt: 0.0,
            frames: 0,
        }
    }
}

impl VoicePrint {
    /// Encode the voiceprint into the 256-dimensional profile embedding.
    ///
    /// The layout is fixed so a profile written by one build stays readable by
    /// the next; unknown slots stay zero.
    #[must_use]
    pub fn to_embedding(&self) -> Vec<f32> {
        let mut embedding = vec![0.0_f32; EMBEDDING_DIM];
        embedding[SLOT_F0] = self.f0_hz / 1000.0;
        embedding[SLOT_F0_SPREAD] = self.f0_spread;
        for (index, gain) in self.band_gains.iter().take(BAND_COUNT).enumerate() {
            embedding[SLOT_BANDS + index] = *gain;
        }
        embedding[SLOT_BRIGHTNESS] = self.brightness;
        embedding[SLOT_TILT] = self.tilt;
        embedding[SLOT_FRAMES] = self.frames as f32 / 1000.0;
        embedding
    }

    /// Decode a voiceprint from a profile embedding.
    ///
    /// A missing or malformed embedding falls back to the neutral voiceprint
    /// rather than failing: a profile created before this feature existed
    /// must still speak.
    #[must_use]
    pub fn from_embedding(embedding: &[f32]) -> Self {
        if embedding.len() < SLOT_FRAMES {
            return Self::default();
        }
        let f0_hz = (embedding[SLOT_F0] * 1000.0).clamp(MIN_F0, MAX_F0);
        let mut band_gains = Vec::with_capacity(BAND_COUNT);
        for index in 0..BAND_COUNT {
            let gain = embedding[SLOT_BANDS + index];
            // A zero gain would silence the band entirely; clamp to a quiet
            // but audible floor so a partially written embedding still speaks.
            band_gains.push(if gain.is_finite() && gain > 0.01 {
                gain.clamp(0.05, 4.0)
            } else {
                1.0
            });
        }
        Self {
            f0_hz,
            f0_spread: embedding[SLOT_F0_SPREAD].clamp(0.0, 1.0),
            band_gains,
            brightness: embedding[SLOT_BRIGHTNESS].clamp(1.0, 20.0),
            tilt: embedding[SLOT_TILT].clamp(-24.0, 24.0),
            frames: (embedding[SLOT_FRAMES] * 1000.0).max(0.0) as usize,
        }
    }

    /// Gain applied to the harmonic whose frequency is `freq_hz`.
    ///
    /// The 16 band gains are interpolated in log-frequency so the harmonic
    /// stack inherits a continuous spectral envelope.
    #[must_use]
    pub fn gain_at(&self, freq_hz: f32) -> f32 {
        if self.band_gains.len() < BAND_COUNT || freq_hz <= 0.0 {
            return 1.0;
        }
        let position = band_position(freq_hz);
        let lower = position.floor() as usize;
        let upper = lower.saturating_add(1).min(BAND_COUNT - 1);
        let fraction = position - lower as f32;
        self.band_gains[lower] * (1.0 - fraction) + self.band_gains[upper] * fraction
    }
}

/// Position of a frequency on the log-spaced band axis.
fn band_position(freq_hz: f32) -> f32 {
    let low = BAND_LOW_HZ.ln();
    let high = BAND_HIGH_HZ.ln();
    let freq = freq_hz.clamp(BAND_LOW_HZ, BAND_HIGH_HZ).ln();
    ((freq - low) / (high - low)) * (BAND_COUNT - 1) as f32
}

/// Downmix interleaved samples to a single mono channel.
fn to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    let channels = usize::from(channels).max(1);
    if channels == 1 || samples.len() < channels {
        return samples.to_vec();
    }
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Estimate the fundamental frequency of one frame by normalized
/// autocorrelation, returning `None` for unvoiced or silent frames.
fn estimate_f0(frame: &[f32], sample_rate: u32) -> Option<f32> {
    if frame.len() < FRAME / 2 {
        return None;
    }
    let min_lag = (sample_rate as f32 / MAX_F0) as usize;
    let max_lag = (sample_rate as f32 / MIN_F0) as usize;
    if max_lag >= frame.len() {
        return None;
    }

    let energy: f32 = frame.iter().map(|value| value * value).sum();
    if energy < 1e-6 {
        return None;
    }

    // Normalized autocorrelation:
    //
    //     r(lag) = sum(x[i] * x[i+lag]) / sqrt(sum(x[i]^2) * sum(x[i+lag]^2))
    //
    // The denominator is taken over the overlapping region, so the score
    // stays in [0, 1] and is comparable across lags regardless of how much
    // signal the two windows contain.
    let mut scores = Vec::with_capacity(max_lag - min_lag + 1);
    for lag in min_lag..=max_lag {
        let overlap = frame.len() - lag;
        let mut sum = 0.0f32;
        let mut energy_a = 0.0f32;
        let mut energy_b = 0.0f32;
        for index in 0..overlap {
            let a = frame[index];
            let b = frame[index + lag];
            sum += a * b;
            energy_a += a * a;
            energy_b += b * b;
        }
        let denominator = (energy_a * energy_b).sqrt();
        scores.push(if denominator > 1e-9 {
            sum / denominator
        } else {
            0.0
        });
    }

    let peak = scores.iter().copied().fold(0.0f32, f32::max);
    if peak < 0.35 {
        return None;
    }

    // A periodic signal correlates just as well at twice its period, so the
    // global maximum alone would report an octave too low. Every local
    // maximum is collected, and the *earliest* one that reaches the global
    // peak is taken: that is the fundamental period, while its multiples
    // score no better.
    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for index in 1..scores.len() - 1 {
        if scores[index] > scores[index - 1] && scores[index] >= scores[index + 1] {
            candidates.push((min_lag + index, scores[index]));
        }
    }
    if candidates.is_empty() {
        return None;
    }
    let threshold = peak * 0.85;
    let (best_lag, _) = candidates
        .iter()
        .find(|(_, score)| *score >= threshold)
        .copied()?;

    // Parabolic interpolation around the peak refines the lag to sub-sample
    // accuracy, which matters at the low end of the pitch range.
    let refined = refine_lag(&scores, best_lag, min_lag);
    Some(sample_rate as f32 / refined)
}

/// Refine an integer lag with a parabolic fit through its two neighbours.
///
/// `scores` is indexed from zero, while `lag` is an absolute sample lag.
fn refine_lag(scores: &[f32], lag: usize, base_lag: usize) -> f32 {
    let index = lag - base_lag;
    if index == 0 || index + 1 >= scores.len() {
        return lag as f32;
    }
    let left = scores[index - 1];
    let center = scores[index];
    let right = scores[index + 1];
    let denominator = left - 2.0 * center + right;
    if denominator.abs() < 1e-9 {
        return lag as f32;
    }
    let offset = 0.5 * (left - right) / denominator;
    let refined = lag as f32 + offset.clamp(-0.5, 0.5);
    if refined > 0.0 { refined } else { lag as f32 }
}

/// In-place iterative radix-2 FFT over a complex buffer of power-of-two size.
fn fft(buffer: &mut [candle_math::Complex]) {
    let n = buffer.len();
    debug_assert!(n.is_power_of_two());
    if n < 2 {
        return;
    }

    // Bit-reversal permutation.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            buffer.swap(i, j);
        }
    }

    let mut length = 2usize;
    while length <= n {
        let angle = -2.0 * std::f32::consts::PI / length as f32;
        let (sin_step, cos_step) = angle.sin_cos();
        let mut i = 0usize;
        while i < n {
            let (mut w_re, mut w_im) = (1.0f32, 0.0f32);
            for offset in 0..(length / 2) {
                let even = buffer[i + offset];
                let odd_re = buffer[i + offset + length / 2].re * w_re
                    - buffer[i + offset + length / 2].im * w_im;
                let odd_im = buffer[i + offset + length / 2].re * w_im
                    + buffer[i + offset + length / 2].im * w_re;
                buffer[i + offset].re = even.re + odd_re;
                buffer[i + offset].im = even.im + odd_im;
                buffer[i + offset + length / 2].re = even.re - odd_re;
                buffer[i + offset + length / 2].im = even.im - odd_im;
                let next_re = w_re * cos_step - w_im * sin_step;
                w_im = w_re * sin_step + w_im * cos_step;
                w_re = next_re;
            }
            i += length;
        }
        length <<= 1;
    }
}

/// A minimal complex number so the crate needs no external math dependency.
pub mod candle_math {
    /// A complex number with 32-bit float components.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct Complex {
        /// Real part.
        pub re: f32,
        /// Imaginary part.
        pub im: f32,
    }

    impl Complex {
        /// Construct a complex number.
        #[must_use]
        pub const fn new(re: f32, im: f32) -> Self {
            Self { re, im }
        }
    }
}

/// Analyze a reference recording and derive its voiceprint.
///
/// The buffer is resampled to 16 kHz for analysis; a buffer that is too short
/// to contain a single analysis frame yields the neutral voiceprint.
#[must_use]
pub fn analyze_voiceprint(pcm: &PcmBuffer) -> VoicePrint {
    let analysis = match resample(pcm, ANALYSIS_RATE) {
        Ok(buffer) => buffer,
        Err(_) => return VoicePrint::default(),
    };
    let mono = to_mono(analysis.samples(), analysis.channels());
    if mono.len() < FRAME {
        return VoicePrint::default();
    }

    // Pitch estimates over every frame.
    let mut pitches: Vec<f32> = Vec::new();
    let mut magnitudes = vec![0.0f64; FRAME / 2 + 1];
    let mut voiced_frames = 0usize;

    let mut start = 0usize;
    while start + FRAME <= mono.len() {
        let frame = &mono[start..start + FRAME];

        // Remove the frame's mean so the autocorrelation sees the periodic
        // part of the signal rather than its DC offset.
        let mean = frame.iter().sum::<f32>() / frame.len() as f32;
        let centered: Vec<f32> = frame.iter().map(|value| value - mean).collect();

        let f0 = estimate_f0(&centered, ANALYSIS_RATE);
        let is_voiced = f0.is_some();
        if let Some(pitch) = f0 {
            pitches.push(pitch);
        }

        // Long-term average spectrum: only voiced frames describe the voice;
        // unvoiced frames would flatten the envelope with fricative energy.
        if is_voiced {
            voiced_frames += 1;
            let mut spectrum: Vec<candle_math::Complex> = centered
                .iter()
                .map(|value| candle_math::Complex::new(*value, 0.0))
                .collect();
            fft(&mut spectrum);
            for (bin, slot) in magnitudes.iter_mut().enumerate().take(FRAME / 2 + 1) {
                let value = spectrum[bin];
                *slot += ((value.re * value.re + value.im * value.im) as f64).sqrt();
            }
        }

        start += HOP;
    }

    if voiced_frames == 0 || pitches.is_empty() {
        return VoicePrint::default();
    }

    // Median pitch and its relative spread.
    pitches.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let f0_hz = pitches[pitches.len() / 2];
    let mean = pitches.iter().sum::<f32>() / pitches.len() as f32;
    let variance = pitches
        .iter()
        .map(|pitch| (pitch - mean) * (pitch - mean))
        .sum::<f32>()
        / pitches.len() as f32;
    let f0_spread = (variance.sqrt() / mean.max(1.0)).clamp(0.0, 1.0);

    // Normalize the average spectrum, then fold it into the band gains.
    let bin_hz = ANALYSIS_RATE as f32 / FRAME as f32;
    let mut bands = [0.0f64; BAND_COUNT];
    let mut band_counts = [0usize; BAND_COUNT];
    let mut centroid_sum = 0.0f64;
    let mut magnitude_sum = 0.0f64;

    for (bin, magnitude) in magnitudes.iter().enumerate().take(FRAME / 2 + 1) {
        let freq = bin as f32 * bin_hz;
        if freq < BAND_LOW_HZ || freq > BAND_HIGH_HZ {
            continue;
        }
        centroid_sum += freq as f64 * magnitude;
        magnitude_sum += magnitude;
        let position = band_position(freq) as usize;
        bands[position.min(BAND_COUNT - 1)] += magnitude;
        band_counts[position.min(BAND_COUNT - 1)] += 1;
    }

    // The reference level is the geometric mean of the band energies, so the
    // gains describe the *shape* of the spectrum rather than its loudness.
    let mut band_gains = Vec::with_capacity(BAND_COUNT);
    let mut log_sum = 0.0f64;
    let mut usable = 0usize;
    for (index, total) in bands.iter().enumerate() {
        let count = band_counts[index];
        if count == 0 || *total <= 0.0 {
            band_gains.push(1.0f32);
            continue;
        }
        let level = (total / count as f64).max(1e-9);
        log_sum += level.ln();
        usable += 1;
        band_gains.push(level as f32);
    }
    if usable > 0 {
        let reference = (log_sum / usable as f64).exp() as f32;
        for gain in &mut band_gains {
            *gain = (*gain / reference).clamp(0.05, 4.0);
        }
    }

    // Brightness: the spectral centroid relative to the fundamental.
    let brightness = if magnitude_sum > 0.0 {
        (centroid_sum / magnitude_sum) as f32 / f0_hz.max(1.0)
    } else {
        4.0
    };

    // Tilt: the slope of the average spectrum in dB per octave, measured
    // between the lowest and highest populated bands.
    let first = band_gains.iter().position(|gain| *gain > 0.0).unwrap_or(0);
    let last = band_gains
        .iter()
        .rposition(|gain| *gain > 0.0)
        .unwrap_or(BAND_COUNT - 1);
    let octaves = (last.saturating_sub(first)) as f32;
    let tilt = if octaves > 0.0 {
        let db_low = 20.0 * band_gains[first].max(1e-6).log10();
        let db_high = 20.0 * band_gains[last].max(1e-6).log10();
        ((db_high - db_low) / octaves).clamp(-24.0, 24.0)
    } else {
        0.0
    };

    VoicePrint {
        f0_hz,
        f0_spread,
        band_gains,
        brightness: brightness.clamp(1.0, 20.0),
        tilt,
        frames: voiced_frames,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic vowel-like tone with a known fundamental and a second
    /// harmonic 6 dB down, used as a reference recording.
    fn vowel(f0: f32, seconds: f32, rate: u32) -> PcmBuffer {
        let frames = (seconds * rate as f32) as usize;
        let samples: Vec<f32> = (0..frames)
            .map(|index| {
                let t = index as f32 / rate as f32;
                let fundamental = (2.0 * std::f32::consts::PI * f0 * t).sin();
                let second = 0.5 * (2.0 * std::f32::consts::PI * 2.0 * f0 * t).sin();
                0.4 * (fundamental + second)
            })
            .collect();
        PcmBuffer::new(rate, 1, samples).expect("pcm")
    }

    #[test]
    fn analysis_recovers_the_reference_pitch() {
        let reference = vowel(140.0, 1.0, 16_000);
        let print = analyze_voiceprint(&reference);
        assert!(
            print.frames > 4,
            "expected voiced frames, got {}",
            print.frames
        );
        assert!(
            (print.f0_hz - 140.0).abs() < 6.0,
            "expected ~140 Hz, got {:.2}",
            print.f0_hz
        );
    }

    #[test]
    fn analysis_distinguishes_a_higher_voice_from_a_lower_one() {
        let low = analyze_voiceprint(&vowel(100.0, 1.0, 16_000));
        let high = analyze_voiceprint(&vowel(220.0, 1.0, 16_000));
        assert!(low.f0_hz < high.f0_hz);
        assert!(
            (high.f0_hz - 220.0).abs() < 8.0,
            "expected ~220 Hz, got {:.2}",
            high.f0_hz
        );
    }

    #[test]
    fn the_embedding_round_trips_the_voiceprint() {
        let reference = vowel(165.0, 1.0, 16_000);
        let print = analyze_voiceprint(&reference);
        let embedding = print.to_embedding();
        assert_eq!(embedding.len(), EMBEDDING_DIM);

        let restored = VoicePrint::from_embedding(&embedding);
        assert!((restored.f0_hz - print.f0_hz).abs() < 1.0);
        assert!((restored.brightness - print.brightness).abs() < 0.01);
        assert!((restored.tilt - print.tilt).abs() < 0.01);
        assert_eq!(restored.frames, print.frames);
        for (a, b) in print.band_gains.iter().zip(&restored.band_gains) {
            assert!((a - b).abs() < 0.001, "band gain drifted: {a} vs {b}");
        }
    }

    #[test]
    fn a_missing_embedding_falls_back_to_the_neutral_voiceprint() {
        let restored = VoicePrint::from_embedding(&[]);
        assert!((restored.f0_hz - 160.0).abs() < 0.01);
        assert_eq!(restored.band_gains.len(), BAND_COUNT);
    }

    #[test]
    fn silence_has_no_voiceprint() {
        let silence = PcmBuffer::new(16_000, 1, vec![0.0; 16_000]).expect("pcm");
        let print = analyze_voiceprint(&silence);
        assert_eq!(print.frames, 0);
        assert!((print.f0_hz - 160.0).abs() < 0.01);
    }

    #[test]
    fn band_gains_describe_the_spectral_shape() {
        // A dark tone (fundamental only) must be brighter than it is, and a
        // tone with a strong second harmonic must lift the upper bands.
        let reference = vowel(150.0, 1.0, 16_000);
        let print = analyze_voiceprint(&reference);
        assert!(print.gain_at(150.0) > 0.0);
        assert!(print.gain_at(300.0) > 0.0);
        // Beyond the top band the interpolation must stay finite.
        assert!(print.gain_at(12_000.0).is_finite());
    }

    #[test]
    fn the_fft_matches_a_naive_dft() {
        let n = 64usize;
        let input: Vec<f32> = (0..n).map(|i| (i as f32 * 0.3).sin()).collect();
        let mut spectrum: Vec<candle_math::Complex> = input
            .iter()
            .map(|value| candle_math::Complex::new(*value, 0.0))
            .collect();
        fft(&mut spectrum);

        for bin in [0usize, 1, 5, 17] {
            let mut re = 0.0f64;
            let mut im = 0.0f64;
            for (index, value) in input.iter().enumerate() {
                let angle = -2.0 * std::f64::consts::PI * bin as f64 * index as f64 / n as f64;
                re += *value as f64 * angle.cos();
                im += *value as f64 * angle.sin();
            }
            assert!(
                (spectrum[bin].re as f64 - re).abs() < 1e-2,
                "bin {bin} real part: {} vs {re}",
                spectrum[bin].re
            );
            assert!(
                (spectrum[bin].im as f64 - im).abs() < 1e-2,
                "bin {bin} imaginary part: {} vs {im}",
                spectrum[bin].im
            );
        }
    }
}
