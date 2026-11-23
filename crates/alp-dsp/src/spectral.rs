//! High-resolution Short-Time Fourier Transform (STFT) and spectrogram computation.
//!
//! Provides `SpectrogramEngine` for computing time-frequency intensity matrices from PCM audio
//! with configurable window sizes (512 to 8192), window functions (Hanning, Hamming, Blackman-Harris),
//! power spectral density normalization (dBFS), and linear / logarithmic / Mel frequency bin aggregation.

use std::f32::consts::PI;
use std::sync::Arc;

use alp_core::CoreError;
use realfft::num_complex::Complex32;
use realfft::{RealFftPlanner, RealToComplex};
use serde::{Deserialize, Serialize};

/// Supported analysis window functions for STFT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WindowType {
    /// Hanning (Hann) raised cosine window with -31.5 dB sidelobe rejection.
    #[default]
    Hanning,
    /// Hamming window with -42.7 dB sidelobe rejection.
    Hamming,
    /// 4-term Blackman-Harris window with -92 dB sidelobe rejection for high dynamic range analysis.
    BlackmanHarris,
    /// Rectangular window (boxcar, uniform weighting).
    Rectangular,
}

impl WindowType {
    /// Generate a window of length `size`.
    #[must_use]
    pub fn generate_window(self, size: usize) -> Vec<f32> {
        if size == 0 {
            return Vec::new();
        }
        if size == 1 {
            return vec![1.0];
        }

        let denom = (size - 1) as f32;
        let mut window = Vec::with_capacity(size);

        match self {
            Self::Hanning => {
                for i in 0..size {
                    let w = 0.5 - 0.5 * (2.0 * PI * i as f32 / denom).cos();
                    window.push(w);
                }
            }
            Self::Hamming => {
                for i in 0..size {
                    let w = 0.54 - 0.46 * (2.0 * PI * i as f32 / denom).cos();
                    window.push(w);
                }
            }
            Self::BlackmanHarris => {
                const A0: f32 = 0.35875;
                const A1: f32 = 0.48829;
                const A2: f32 = 0.14128;
                const A3: f32 = 0.01168;
                for i in 0..size {
                    let frac = i as f32 / denom;
                    let w = A0 - A1 * (2.0 * PI * frac).cos() + A2 * (4.0 * PI * frac).cos()
                        - A3 * (6.0 * PI * frac).cos();
                    window.push(w);
                }
            }
            Self::Rectangular => {
                window.resize(size, 1.0);
            }
        }

        window
    }
}

/// Frequency bin scaling mode for display aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FrequencyScale {
    /// Linear spacing from minimum frequency to maximum frequency.
    Linear,
    /// Logarithmic frequency spacing (closer matching human pitch perception).
    #[default]
    Logarithmic,
    /// Mel scale representation.
    Mel,
}

/// Configuration parameters for spectrogram generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectrogramConfig {
    /// FFT size (typically 512, 1024, 2048, 4096, 8192).
    pub fft_size: usize,
    /// Hop size in samples (typically fft_size / 4 for 75% overlap or fft_size / 2 for 50%).
    pub hop_size: usize,
    /// Window function.
    pub window_type: WindowType,
    /// Frequency scaling mode.
    pub freq_scale: FrequencyScale,
    /// Minimum frequency in Hz (e.g. 20.0).
    pub min_freq_hz: f32,
    /// Maximum frequency in Hz (e.g. 20000.0 or sample_rate / 2).
    pub max_freq_hz: f32,
    /// Target number of frequency display bins (e.g. 128, 256, 512).
    pub num_freq_bins: usize,
    /// Floor decibel level for dBFS clipping (e.g. -100.0 dBFS).
    pub min_db: f32,
    /// Ceiling decibel level for normalization (e.g. 0.0 dBFS).
    pub max_db: f32,
}

impl Default for SpectrogramConfig {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            hop_size: 512,
            window_type: WindowType::Hanning,
            freq_scale: FrequencyScale::Logarithmic,
            min_freq_hz: 20.0,
            max_freq_hz: 20000.0,
            num_freq_bins: 256,
            min_db: -100.0,
            max_db: 0.0,
        }
    }
}

impl SpectrogramConfig {
    /// Validates configuration parameters.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.fft_size < 64 || (self.fft_size & (self.fft_size - 1)) != 0 {
            return Err(CoreError::validation(
                "fft_size must be a power of two >= 64",
            ));
        }
        if self.hop_size == 0 || self.hop_size > self.fft_size {
            return Err(CoreError::validation(
                "hop_size must be > 0 and <= fft_size",
            ));
        }
        if self.min_freq_hz < 0.0 || self.min_freq_hz >= self.max_freq_hz {
            return Err(CoreError::validation(
                "min_freq_hz must be >= 0 and < max_freq_hz",
            ));
        }
        if self.num_freq_bins == 0 {
            return Err(CoreError::validation("num_freq_bins must be > 0"));
        }
        if self.min_db >= self.max_db {
            return Err(CoreError::validation("min_db must be < max_db"));
        }
        Ok(())
    }
}

/// Computed spectrogram data containing time, frequency, and normalized intensity grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectrogramData {
    /// Audio sample rate in Hz.
    pub sample_rate: u32,
    /// Total duration of processed audio in seconds.
    pub duration_sec: f64,
    /// Number of time frames (columns).
    pub num_frames: usize,
    /// Number of frequency bins (rows).
    pub num_bins: usize,
    /// Center frequencies in Hz for each row bin `[num_bins]`.
    pub freq_bins_hz: Vec<f32>,
    /// Time positions in seconds for each column frame `[num_frames]`.
    pub time_stamps_sec: Vec<f64>,
    /// Flattened row-major intensity matrix `[num_frames * num_bins]` normalized to `[0.0, 1.0]`.
    /// Frame `f` and frequency bin `b` is at index `f * num_bins + b`.
    pub matrix: Vec<f32>,
    /// Configured minimum dBFS.
    pub min_db: f32,
    /// Configured maximum dBFS.
    pub max_db: f32,
}

impl SpectrogramData {
    /// Returns the normalized intensity `[0.0, 1.0]` for frame `frame_idx` and frequency bin `bin_idx`.
    #[must_use]
    pub fn get_intensity(&self, frame_idx: usize, bin_idx: usize) -> f32 {
        if frame_idx < self.num_frames && bin_idx < self.num_bins {
            self.matrix[frame_idx * self.num_bins + bin_idx]
        } else {
            0.0
        }
    }

    /// Converts normalized intensity `[0.0, 1.0]` back to dBFS value.
    #[must_use]
    pub fn intensity_to_db(&self, intensity: f32) -> f32 {
        self.min_db + intensity * (self.max_db - self.min_db)
    }
}

/// Short-Time Fourier Transform Spectrogram Computation Engine.
pub struct SpectrogramEngine {
    config: SpectrogramConfig,
    window: Vec<f32>,
    window_sum: f32,
    r2c: Arc<dyn RealToComplex<f32>>,
}

impl SpectrogramEngine {
    /// Creates a new `SpectrogramEngine` with the given configuration.
    pub fn new(config: SpectrogramConfig) -> Result<Self, CoreError> {
        config.validate()?;

        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(config.fft_size);

        let window = config.window_type.generate_window(config.fft_size);
        let window_sum: f32 = window.iter().sum::<f32>().max(1e-12);

        Ok(Self {
            config,
            window,
            window_sum,
            r2c,
        })
    }

    /// Returns the active configuration.
    #[must_use]
    pub fn config(&self) -> &SpectrogramConfig {
        &self.config
    }

    /// Computes the spectrogram from raw mono audio samples.
    pub fn compute(&self, samples: &[f32], sample_rate: u32) -> Result<SpectrogramData, CoreError> {
        if samples.is_empty() {
            return Err(CoreError::validation(
                "audio buffer cannot be empty for spectrogram computation",
            ));
        }
        if sample_rate == 0 {
            return Err(CoreError::validation("sample_rate must be > 0"));
        }

        let fft_size = self.config.fft_size;
        let hop_size = self.config.hop_size;
        let num_raw_bins = fft_size / 2 + 1;
        let nyquist = sample_rate as f32 / 2.0;

        let max_freq = self
            .config
            .max_freq_hz
            .min(nyquist)
            .max(self.config.min_freq_hz + 10.0);
        let min_freq = self.config.min_freq_hz.max(1.0);

        // Precompute target frequency bins
        let target_bins = self.config.num_freq_bins;
        let mut freq_bins_hz = Vec::with_capacity(target_bins);
        match self.config.freq_scale {
            FrequencyScale::Linear => {
                for i in 0..target_bins {
                    let frac = i as f32 / (target_bins.max(2) - 1) as f32;
                    freq_bins_hz.push(min_freq + frac * (max_freq - min_freq));
                }
            }
            FrequencyScale::Logarithmic => {
                let log_min = min_freq.ln();
                let log_max = max_freq.ln();
                for i in 0..target_bins {
                    let frac = i as f32 / (target_bins.max(2) - 1) as f32;
                    let freq = (log_min + frac * (log_max - log_min)).exp();
                    freq_bins_hz.push(freq);
                }
            }
            FrequencyScale::Mel => {
                let hz_to_mel = |hz: f32| 2595.0 * (1.0 + hz / 700.0).log10();
                let mel_to_hz = |mel: f32| 700.0 * (10.0f32.powf(mel / 2595.0) - 1.0);
                let mel_min = hz_to_mel(min_freq);
                let mel_max = hz_to_mel(max_freq);
                for i in 0..target_bins {
                    let frac = i as f32 / (target_bins.max(2) - 1) as f32;
                    let mel = mel_min + frac * (mel_max - mel_min);
                    freq_bins_hz.push(mel_to_hz(mel));
                }
            }
        }

        // Map target frequency bins to raw FFT bin indices
        let bin_hz_step = nyquist / (num_raw_bins - 1) as f32;
        let raw_bin_indices: Vec<usize> = freq_bins_hz
            .iter()
            .map(|&f| {
                let idx = (f / bin_hz_step).round() as usize;
                idx.min(num_raw_bins - 1)
            })
            .collect();

        // Calculate time frames
        let num_frames = if samples.len() < fft_size {
            1
        } else {
            (samples.len() - fft_size) / hop_size + 1
        };

        let mut time_stamps_sec = Vec::with_capacity(num_frames);
        for f in 0..num_frames {
            let sample_center = f * hop_size + fft_size / 2;
            time_stamps_sec.push(sample_center as f64 / sample_rate as f64);
        }

        // Allocate buffers for real FFT
        let mut input_buf = vec![0.0f32; fft_size];
        let mut spectrum = vec![Complex32::new(0.0, 0.0); num_raw_bins];
        let mut scratch = self.r2c.make_scratch_vec();

        let mut matrix = Vec::with_capacity(num_frames * target_bins);
        let min_db = self.config.min_db;
        let max_db = self.config.max_db;
        let db_range = (max_db - min_db).max(1e-6);

        // Amplitude normalization factor based on coherent gain of the window
        let norm_factor = 2.0 / self.window_sum;

        for f in 0..num_frames {
            let start = f * hop_size;
            let end = (start + fft_size).min(samples.len());

            // Zero buffer and apply window
            input_buf.fill(0.0);
            for (i, &s) in samples[start..end].iter().enumerate() {
                input_buf[i] = s * self.window[i];
            }

            // Perform Real-to-Complex FFT
            let _ = self
                .r2c
                .process_with_scratch(&mut input_buf, &mut spectrum, &mut scratch);

            // Extract target bins and convert magnitude to normalized dBFS
            for &raw_idx in &raw_bin_indices {
                let comp = spectrum[raw_idx];
                let mag = (comp.re * comp.re + comp.im * comp.im).sqrt() * norm_factor;
                let db = 20.0 * (mag.max(1e-10)).log10();
                let clamped_db = db.clamp(min_db, max_db);
                let normalized = (clamped_db - min_db) / db_range;
                matrix.push(normalized);
            }
        }

        Ok(SpectrogramData {
            sample_rate,
            duration_sec: samples.len() as f64 / sample_rate as f64,
            num_frames,
            num_bins: target_bins,
            freq_bins_hz,
            time_stamps_sec,
            matrix,
            min_db,
            max_db,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_generation_and_symmetry() {
        for wtype in [
            WindowType::Hanning,
            WindowType::Hamming,
            WindowType::BlackmanHarris,
            WindowType::Rectangular,
        ] {
            let win = wtype.generate_window(256);
            assert_eq!(win.len(), 256);
            // Symmetry check
            for i in 0..128 {
                assert!((win[i] - win[255 - i]).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn test_spectrogram_config_validation() {
        let valid = SpectrogramConfig::default();
        assert!(valid.validate().is_ok());

        let mut invalid_fft = valid.clone();
        invalid_fft.fft_size = 500; // Not a power of two
        assert!(invalid_fft.validate().is_err());

        let mut invalid_hop = valid.clone();
        invalid_hop.hop_size = 4096; // Hop > FFT
        assert!(invalid_hop.validate().is_err());

        let mut invalid_freq = valid.clone();
        invalid_freq.min_freq_hz = 1000.0;
        invalid_freq.max_freq_hz = 500.0;
        assert!(invalid_freq.validate().is_err());
    }

    #[test]
    fn test_spectrogram_pure_tone_peak_detection() {
        let config = SpectrogramConfig {
            fft_size: 1024,
            hop_size: 256,
            window_type: WindowType::Hanning,
            freq_scale: FrequencyScale::Linear,
            min_freq_hz: 0.0,
            max_freq_hz: 4000.0,
            num_freq_bins: 128,
            min_db: -90.0,
            max_db: 0.0,
        };

        let engine = SpectrogramEngine::new(config).expect("create engine");
        let sample_rate = 16000;
        let tone_freq = 1000.0f32; // 1 kHz pure sine tone
        let num_samples = 4000; // 0.25 s

        let samples: Vec<f32> = (0..num_samples)
            .map(|i| (2.0 * PI * tone_freq * (i as f32 / sample_rate as f32)).sin())
            .collect();

        let data = engine.compute(&samples, sample_rate).expect("compute");
        assert!(data.num_frames > 0);
        assert_eq!(data.num_bins, 128);

        // Find bin closest to 1000 Hz
        let target_bin = data
            .freq_bins_hz
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| (tone_freq - *a).abs().total_cmp(&(tone_freq - *b).abs()))
            .map(|(idx, _)| idx)
            .unwrap();

        // Check that the middle frame has maximum intensity at target_bin
        let mid_frame = data.num_frames / 2;
        let mut max_intensity = 0.0f32;
        let mut peak_bin = 0;

        for b in 0..data.num_bins {
            let val = data.get_intensity(mid_frame, b);
            if val > max_intensity {
                max_intensity = val;
                peak_bin = b;
            }
        }

        // Peak bin must match target frequency bin within 1 bin tolerance
        assert!(
            (peak_bin as isize - target_bin as isize).abs() <= 1,
            "peak_bin {} did not match target_bin {}",
            peak_bin,
            target_bin
        );
        assert!(
            max_intensity > 0.85,
            "pure tone intensity should be high near 0 dBFS"
        );
    }

    #[test]
    fn test_frequency_scale_monotonicity() {
        for scale in [
            FrequencyScale::Linear,
            FrequencyScale::Logarithmic,
            FrequencyScale::Mel,
        ] {
            let config = SpectrogramConfig {
                fft_size: 512,
                hop_size: 128,
                freq_scale: scale,
                min_freq_hz: 40.0,
                max_freq_hz: 12000.0,
                num_freq_bins: 64,
                ..Default::default()
            };
            let engine = SpectrogramEngine::new(config).expect("create engine");
            let dummy_samples = vec![0.1f32; 1024];
            let data = engine.compute(&dummy_samples, 48000).expect("compute");

            assert_eq!(data.freq_bins_hz.len(), 64);
            for i in 1..data.freq_bins_hz.len() {
                assert!(
                    data.freq_bins_hz[i] > data.freq_bins_hz[i - 1],
                    "scale {:?} must be strictly monotonically increasing",
                    scale
                );
            }
        }
    }

    #[test]
    fn test_silence_produces_min_db_intensity() {
        let engine = SpectrogramEngine::new(SpectrogramConfig::default()).expect("engine");
        let silence = vec![0.0f32; 4096];
        let data = engine.compute(&silence, 48000).expect("compute");

        for &val in &data.matrix {
            assert_eq!(val, 0.0, "silence must yield zero normalized intensity");
        }
    }
}
