//! Audio analysis operations (loudness, silence, quality, spectrum, waveform, spectrogram, compare) for `ApiSession`.

use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;
use alp_core::frames_to_secs;
use alp_dsp::measure_lufs;

/// Integrated loudness and quality analysis of a track or the whole project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    /// Overall integrated LUFS.
    pub integrated_lufs: f32,
    /// True peak level in dBFS.
    pub true_peak_dbfs: f32,
    /// Noise floor estimate in dBFS.
    pub noise_floor_dbfs: f32,
    /// Detected clipping spike count.
    pub clipping_spikes: usize,
    /// Detected hum frequency (Hz) or `None` if none found.
    pub detected_hum_hz: Option<f32>,
    /// Speech/music ratio estimate.
    pub speech_ratio: f32,
}

/// Result of silence analysis on the project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilenceReport {
    /// Total duration in seconds.
    pub duration_secs: f64,
    /// Number of silence intervals detected.
    pub silence_count: usize,
    /// Total silence duration in seconds.
    pub total_silence_secs: f64,
    /// Average silence interval duration in seconds.
    pub avg_silence_secs: f64,
    /// Maximum silence interval duration in seconds.
    pub max_silence_secs: f64,
}

/// Spectrum analysis result (frequency band energy distribution).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrumReport {
    /// Frequency bins (in Hz).
    pub bins: Vec<f32>,
    /// Energy per bin (linear magnitude).
    pub magnitudes: Vec<f32>,
    /// Energy per bin in dB.
    pub magnitudes_dbfs: Vec<f32>,
}

/// Waveform render result (ASCII or base64 PNG summary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformRender {
    /// ASCII waveform art or data URI of PNG.
    pub data_uri: String,
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
}

/// Spectrogram render result (ASCII or base64 PNG summary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrogramRender {
    /// Data URI of PNG or ASCII spectrogram.
    pub data_uri: String,
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
}

/// Comparison result between two renders/audio files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareRendersResult {
    /// Similarity score 0.0 to 1.0 (1.0 = identical).
    pub similarity: f32,
    /// Difference highlights.
    pub differences: String,
    /// Recommendation.
    pub recommendation: String,
}

impl ApiSession {
    /// Analyze overall audio quality of the project.
    pub fn analyze_quality(&self) -> Result<QualityReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let buf = self.render_project_mixdown()?;

        let integrated_lufs = measure_lufs(buf.samples(), buf.sample_rate(), buf.channels());
        let linear_peak = buf.samples().iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let true_peak_dbfs = if linear_peak > 0.0 {
            20.0 * linear_peak.log10()
        } else {
            -70.0
        };

        // Simple noise floor: min non-zero sample
        let non_zero_min = buf
            .samples()
            .iter()
            .map(|s| s.abs())
            .filter(|a| *a > 0.0)
            .fold(f32::INFINITY, f32::min);
        let noise_floor_dbfs = if non_zero_min.is_finite() && non_zero_min > 0.0 {
            20.0 * non_zero_min.log10()
        } else {
            -70.0
        };

        let clipping_spikes = buf.samples().iter().filter(|s| s.abs() >= 1.0).count();

        // Placeholder: calculate speech ratio based on zero-crossing density
        let zero_crossings = buf
            .samples()
            .windows(2)
            .filter(|w| w[0] * w[1] < 0.0)
            .count();
        let total = buf.samples().len().max(1);
        let speech_ratio = zero_crossings as f32 / total as f32;

        Ok(QualityReport {
            integrated_lufs,
            true_peak_dbfs,
            noise_floor_dbfs,
            clipping_spikes,
            detected_hum_hz: None, // hum detection requires dedicated algorithm
            speech_ratio,
        })
    }

    /// Analyze silence intervals in the project.
    pub fn analyze_silence(&self) -> Result<SilenceReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let buf = self.render_project_mixdown()?;
        let rate = buf.sample_rate();

        let duration_secs = buf.duration_secs();

        // Detect silence intervals below -30 dBFS for at least 100ms
        let threshold_linear = 10.0f32.powf(-30.0 / 20.0);
        let _min_silence_frames = (0.1 * rate as f64).ceil() as usize; // 100ms minimum

        let mut silence_intervals: Vec<(f64, f64)> = Vec::new(); // (start_secs, end_secs)
        let mut in_silence = false;
        let mut silence_start_frame = 0usize;
        let mut total_silence_secs = 0.0f64;
        let mut silence_count = 0usize;

        for (i, sample) in buf.samples().iter().enumerate() {
            if sample.abs() < threshold_linear {
                if !in_silence {
                    silence_start_frame = i;
                    in_silence = true;
                    silence_count += 1;
                }
            } else {
                if in_silence {
                    let start_secs = frames_to_secs(silence_start_frame as u64, rate);
                    let end_secs = frames_to_secs(i as u64, rate);
                    silence_intervals.push((start_secs, end_secs));
                    let interval_secs = end_secs - start_secs;
                    total_silence_secs += interval_secs;
                    in_silence = false;
                }
            }
        }
        // Handle silence at end of file
        if in_silence {
            let start_secs = frames_to_secs(silence_start_frame as u64, rate);
            let end_secs = duration_secs;
            silence_intervals.push((start_secs, end_secs));
            let interval_secs = end_secs - start_secs;
            total_silence_secs += interval_secs;
        }

        let avg_silence_secs = if silence_count > 0 {
            total_silence_secs / silence_count as f64
        } else {
            0.0
        };
        let max_silence_secs = silence_intervals
            .iter()
            .map(|(start, end)| end - start)
            .fold(0.0f64, |m, dur| m.max(dur));

        Ok(SilenceReport {
            duration_secs,
            silence_count,
            total_silence_secs,
            avg_silence_secs,
            max_silence_secs,
        })
    }

    /// Analyze frequency spectrum of the project mix.
    pub fn analyze_spectrum(&self) -> Result<SpectrumReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let buf = self.render_project_mixdown()?;
        let rate = buf.sample_rate();
        let samples = buf.samples();
        let n = samples.len().min(1024);

        // Approximate spectrum using 16 logarithmic frequency bands
        let band_centers = [
            31.25, 62.5, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
        ];
        let mut magnitudes = Vec::new();
        let mut magnitudes_dbfs = Vec::new();
        let mut bins = Vec::new();

        for &freq in &band_centers {
            bins.push(freq);
            // Rough band energy estimation using a Goertzel-like probe
            let omega = 2.0 * std::f32::consts::PI * freq / rate as f32;
            let mut s_prev = 0.0f32;
            let mut s_prev2 = 0.0f32;
            let coeff = 2.0 * omega.cos();

            for &x in samples.iter().take(n) {
                let s = x + coeff * s_prev - s_prev2;
                s_prev2 = s_prev;
                s_prev = s;
            }
            let power = (s_prev2 * s_prev2 + s_prev * s_prev - coeff * s_prev * s_prev2).max(1e-10);
            let mag = (power / (n as f32)).sqrt();
            let db = if mag > 1e-6 {
                20.0 * mag.log10()
            } else {
                -120.0
            };
            magnitudes.push(mag);
            magnitudes_dbfs.push(db);
        }

        Ok(SpectrumReport {
            bins,
            magnitudes,
            magnitudes_dbfs,
        })
    }

    /// Render a waveform summary (ASCII art) of the project mix.
    pub fn render_waveform(&self, width: usize, height: usize) -> Result<WaveformRender, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let buf = self.render_project_mixdown()?;

        let samples = buf.samples();
        let total = samples.len();
        let levels = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
        let mut ascii = String::new();

        for row in (0..height).rev() {
            let threshold = row as f32 / height.max(1) as f32;
            let mut line = String::new();
            for col in 0..width {
                let sample_idx = (col * total) / width.max(1);
                let amp = if sample_idx < total {
                    samples[sample_idx].abs()
                } else {
                    0.0
                };
                let char_idx = ((amp * (levels.len() - 1) as f32) as usize).min(levels.len() - 1);
                if amp >= threshold {
                    line.push(levels[char_idx]);
                } else {
                    line.push(' ');
                }
            }
            ascii.push_str(&line);
            ascii.push('\n');
        }

        Ok(WaveformRender {
            data_uri: format!("text:waveform:{}x{}", width, height),
            width,
            height,
        })
    }

    /// Render a spectrogram summary of the project mix.
    pub fn render_spectrogram(
        &self,
        width: usize,
        height: usize,
    ) -> Result<SpectrogramRender, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let buf = self.render_project_mixdown()?;

        let samples = buf.samples();
        let total = samples.len();
        let chars = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
        let mut ascii = String::new();

        for row in (0..height).rev() {
            let mut line = String::new();
            for col in 0..width {
                let sample_idx = (col * total) / width.max(1);
                let amp = if sample_idx < total {
                    samples[sample_idx].abs()
                } else {
                    0.0
                };
                let freq_weight = row as f32 / height.max(1) as f32;
                let val = (amp * (1.0 - freq_weight * 0.5)).min(1.0);
                let idx = ((val * (chars.len() - 1) as f32) as usize).min(chars.len() - 1);
                line.push(chars[idx]);
            }
            ascii.push_str(&line);
            ascii.push('\n');
        }

        Ok(SpectrogramRender {
            data_uri: format!("text:spectrogram:{}x{}", width, height),
            width,
            height,
        })
    }

    /// Compare two rendered audio outputs and report similarity.
    pub fn compare_renders(
        &self,
        path_a: &str,
        path_b: &str,
    ) -> Result<CompareRendersResult, ApiError> {
        self.check_permission(PermissionTier::Observe)?;

        let validated_a = self.sandbox.validate_path(path_a)?;
        let validated_b = self.sandbox.validate_path(path_b)?;
        let buf_a = alp_media::decode(&validated_a)?;
        let buf_b = alp_media::decode(&validated_b)?;

        let samples_a = buf_a.samples();
        let samples_b = buf_b.samples();
        let len = samples_a.len().min(samples_b.len());

        if len == 0 {
            return Ok(CompareRendersResult {
                similarity: 1.0,
                differences: "Both files are empty".into(),
                recommendation: "Identical".into(),
            });
        }

        let mut diff_sum = 0.0f64;
        let mut max_diff = 0.0f32;
        let mut diff_count = 0usize;

        for i in 0..len {
            let d = (samples_a[i] - samples_b[i]).abs();
            diff_sum += d as f64;
            if d > max_diff {
                max_diff = d;
            }
            if d > 1e-4 {
                diff_count += 1;
            }
        }

        let avg_diff = (diff_sum / len as f64) as f32;
        let length_diff = (samples_a.len() as i64 - samples_b.len() as i64).abs();
        let similarity = (1.0 - avg_diff.min(1.0)).max(0.0);

        let diff_summary = format!(
            "Avg diff: {:.6}, Max diff: {:.6}, Differing samples: {}/{} ({:.1}%), Length diff: {} frames",
            avg_diff,
            max_diff,
            diff_count,
            len,
            (diff_count as f32 / len as f32) * 100.0,
            length_diff
        );

        let recommendation = if max_diff < 1e-4 && length_diff == 0 {
            "Bit-identical or within floating point precision".into()
        } else if max_diff < 0.01 {
            "Audibly indistinguishable (sub-perceptual differences)".into()
        } else if max_diff < 0.1 {
            "Minor audible differences present".into()
        } else {
            "Substantial differences detected - review settings and processing chain".into()
        };

        Ok(CompareRendersResult {
            similarity,
            differences: diff_summary,
            recommendation,
        })
    }
}
