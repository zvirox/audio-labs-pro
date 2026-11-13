//! Audio analysis operations (loudness, silence, quality, spectrum, waveform, spectrogram, compare) for `ApiSession`.
//!
//! Every measurement here is taken from a real render of the project. The
//! waveform and spectrogram renderers return the art they computed: an agent
//! that asks for a picture gets the actual picture, not a stub locator.

use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;
use alp_core::frames_to_secs;
use alp_dsp::measure_lufs;

/// Frame length for level statistics (20 ms).
const LEVEL_FRAME_MS: u32 = 20;

/// Level below which a 20 ms frame counts as inactive, in dBFS.
const ACTIVE_FRAME_DBFS: f32 = -45.0;

/// Floor reported when a buffer is digital silence.
const SILENCE_FLOOR_DBFS: f32 = -120.0;

/// Integrated loudness and quality analysis of a track or the whole project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    /// Overall integrated LUFS.
    pub integrated_lufs: f32,
    /// Sample peak level in dBFS. Inter-sample (true) peak is not measured yet.
    pub peak_dbfs: f32,
    /// 10th-percentile frame level in dBFS: the noise floor estimate.
    pub noise_floor_dbfs: f32,
    /// Detected clipping spike count.
    pub clipping_spikes: usize,
    /// Detected hum frequency (Hz) or `None` if none found.
    pub detected_hum_hz: Option<f32>,
    /// Fraction of 20 ms frames above the activity threshold.
    pub active_ratio: f32,
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
    /// Threshold in dBFS used to define silence.
    pub threshold_dbfs: f32,
    /// Minimum run length in seconds; shorter dips are ignored.
    pub min_silence_secs: f64,
}

/// Spectrum analysis result (frequency band energy distribution).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrumReport {
    /// Band centre frequencies (in Hz).
    pub bins: Vec<f32>,
    /// Average energy per bin (linear magnitude).
    pub magnitudes: Vec<f32>,
    /// Average energy per bin in dB.
    pub magnitudes_dbfs: Vec<f32>,
    /// Number of analysis blocks averaged over the whole render.
    pub blocks: usize,
    /// Sample rate of the analysed render.
    pub sample_rate: u32,
}

/// Waveform render result: an ASCII envelope of the project mix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformRender {
    /// ASCII waveform art; rows are separated by newlines.
    pub ascii: String,
    /// Width in cells (one cell per column).
    pub width: usize,
    /// Height in cells.
    pub height: usize,
    /// Peak amplitude of the rendered mix (linear).
    pub peak: f32,
    /// Duration of the rendered mix in seconds.
    pub duration_secs: f64,
}

/// Spectrogram render result: an ASCII time/frequency map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrogramRender {
    /// ASCII spectrogram art; rows are separated by newlines, low frequencies at the bottom.
    pub ascii: String,
    /// Width in cells (time columns).
    pub width: usize,
    /// Height in cells (frequency bands).
    pub height: usize,
    /// Band centre frequencies (Hz), low to high.
    pub band_centers_hz: Vec<f32>,
    /// Column count and dynamic range used for the dB→character mapping.
    pub range_db: f32,
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

/// Per-frame level statistics shared by the loudness and quality reports.
pub(crate) struct LevelStats {
    /// Sample peak in dBFS.
    pub peak_dbfs: f32,
    /// 10th-percentile frame level in dBFS.
    pub noise_floor_dbfs: f32,
    /// Fraction of frames above [`ACTIVE_FRAME_DBFS`].
    pub active_ratio: f32,
    /// Sample count at or above full scale.
    pub clipping_spikes: usize,
}

/// Measure frame levels of an interleaved buffer.
fn frame_levels_dbfs(samples: &[f32], rate: u32, channels: u16) -> Vec<f32> {
    let channels = usize::from(channels.max(1));
    let frame_len = ((rate as usize * LEVEL_FRAME_MS as usize) / 1000).max(1) * channels;
    samples
        .chunks(frame_len)
        .map(|chunk| {
            let sum: f64 = chunk.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
            let rms = (sum / chunk.len().max(1) as f64).sqrt() as f32;
            20.0 * rms.max(1e-6).log10()
        })
        .collect()
}

/// Linear-interpolated percentile of an unsorted slice in `0.0..=100.0`.
fn percentile(values: &[f32], percent: f32) -> f32 {
    if values.is_empty() {
        return SILENCE_FLOOR_DBFS;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let position = (percent.clamp(0.0, 100.0) / 100.0) * (sorted.len() - 1) as f32;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let weight = position - lower as f32;
    sorted[lower] * (1.0 - weight) + sorted[upper] * weight
}

/// Measure the level statistics of a rendered buffer.
pub(crate) fn level_stats(samples: &[f32], rate: u32, channels: u16) -> LevelStats {
    let linear_peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let peak_dbfs = if linear_peak > 0.0 {
        20.0 * linear_peak.log10()
    } else {
        SILENCE_FLOOR_DBFS
    };
    let levels = frame_levels_dbfs(samples, rate, channels);
    let noise_floor_dbfs = if levels.is_empty() {
        SILENCE_FLOOR_DBFS
    } else {
        percentile(&levels, 10.0).max(SILENCE_FLOOR_DBFS)
    };
    let active = levels.iter().filter(|db| **db > ACTIVE_FRAME_DBFS).count();
    let active_ratio = if levels.is_empty() {
        0.0
    } else {
        active as f32 / levels.len() as f32
    };
    LevelStats {
        peak_dbfs,
        noise_floor_dbfs,
        active_ratio,
        clipping_spikes: samples.iter().filter(|s| s.abs() >= 1.0).count(),
    }
}

/// Goertzel power of `signal` at `frequency`, normalised per sample.
fn goertzel_power(signal: &[f32], rate: u32, frequency: f32) -> f32 {
    if signal.is_empty() || frequency <= 0.0 || frequency >= rate as f32 / 2.0 {
        return 0.0;
    }
    let omega = 2.0 * std::f32::consts::PI * frequency / rate as f32;
    let coeff = 2.0 * omega.cos();
    let mut prev = 0.0f32;
    let mut prev2 = 0.0f32;
    for sample in signal {
        let current = sample + coeff * prev - prev2;
        prev2 = prev;
        prev = current;
    }
    let power = (prev2 * prev2 + prev * prev - coeff * prev * prev2).max(1e-12);
    power / signal.len() as f32
}

/// Log-spaced band centres between `low_hz` and `high_hz`.
fn log_spaced_centers(low_hz: f32, high_hz: f32, bands: usize) -> Vec<f32> {
    let bands = bands.max(1);
    let ratio = (high_hz / low_hz).max(1.0001);
    (0..bands)
        .map(|i| low_hz * ratio.powf((i as f32 + 0.5) / bands as f32))
        .collect()
}

impl ApiSession {
    /// Analyze overall audio quality of the project.
    pub fn analyze_quality(&self) -> Result<QualityReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let buf = self.render_project_mixdown()?;
        let stats = level_stats(buf.samples(), buf.sample_rate(), buf.channels());

        Ok(QualityReport {
            integrated_lufs: measure_lufs(buf.samples(), buf.sample_rate(), buf.channels()),
            peak_dbfs: stats.peak_dbfs,
            noise_floor_dbfs: stats.noise_floor_dbfs,
            clipping_spikes: stats.clipping_spikes,
            // Hum detection needs a dedicated narrowband test; reporting
            // `None` is honest, guessing a frequency would not be.
            detected_hum_hz: None,
            active_ratio: stats.active_ratio,
        })
    }

    /// Analyze silence intervals in the project. A dip counts only when it is
    /// at least 100 ms long, which is what the build plan specifies.
    pub fn analyze_silence(&self) -> Result<SilenceReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let buf = self.render_project_mixdown()?;
        let rate = buf.sample_rate();
        let channels = usize::from(buf.channels().max(1));
        let threshold_dbfs = -30.0f32;
        let threshold_linear = 10.0f32.powf(threshold_dbfs / 20.0);
        let min_silence_secs = 0.1f64;
        let min_silence_frames = (min_silence_secs * rate as f64).ceil() as u64;

        let frames = buf.samples().len() / channels;
        let mut intervals: Vec<(u64, u64)> = Vec::new();
        let mut run_start: Option<u64> = None;

        for frame in 0..frames {
            let peak = (0..channels)
                .map(|c| buf.samples()[frame * channels + c].abs())
                .fold(0.0f32, f32::max);
            let silent = peak < threshold_linear;
            match (silent, run_start) {
                (true, None) => run_start = Some(frame as u64),
                (false, Some(start)) => {
                    if frame as u64 - start >= min_silence_frames {
                        intervals.push((start, frame as u64));
                    }
                    run_start = None;
                }
                _ => {}
            }
        }
        if let Some(start) = run_start
            && frames as u64 - start >= min_silence_frames
        {
            intervals.push((start, frames as u64));
        }

        let total_silence_secs: f64 = intervals
            .iter()
            .map(|(start, end)| frames_to_secs(end - start, rate))
            .sum();
        let max_silence_secs = intervals
            .iter()
            .map(|(start, end)| frames_to_secs(end - start, rate))
            .fold(0.0f64, f64::max);
        let avg_silence_secs = if intervals.is_empty() {
            0.0
        } else {
            total_silence_secs / intervals.len() as f64
        };

        Ok(SilenceReport {
            duration_secs: buf.duration_secs(),
            silence_count: intervals.len(),
            total_silence_secs,
            avg_silence_secs,
            max_silence_secs,
            threshold_dbfs,
            min_silence_secs,
        })
    }

    /// Analyze the frequency spectrum of the project mix. Sixteen log-spaced
    /// bands are averaged over blocks spread across the whole render, so the
    /// report describes the file rather than its first 20 ms.
    pub fn analyze_spectrum(&self) -> Result<SpectrumReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let buf = self.render_project_mixdown()?;
        let rate = buf.sample_rate();
        let channels = usize::from(buf.channels().max(1));
        let frames = buf.samples().len() / channels;
        let bins = log_spaced_centers(50.0, (rate as f32 / 2.0).min(16_000.0), 16);

        // Mono fold-down keeps the band energies comparable across channels.
        let mono: Vec<f32> = (0..frames)
            .map(|f| {
                (0..channels)
                    .map(|c| buf.samples()[f * channels + c])
                    .sum::<f32>()
                    / channels as f32
            })
            .collect();

        let block_frames = 4096.min(mono.len()).max(1);
        let block_count = (mono.len() / block_frames).clamp(1, 64);
        let stride = (mono.len().saturating_sub(block_frames) / block_count.max(1)).max(1);

        let mut magnitudes = vec![0.0f32; bins.len()];
        let mut blocks = 0usize;
        for block in 0..block_count {
            let start = block * stride;
            let end = (start + block_frames).min(mono.len());
            if start >= end {
                break;
            }
            let window = &mono[start..end];
            for (index, center) in bins.iter().enumerate() {
                magnitudes[index] += goertzel_power(window, rate, *center).sqrt();
            }
            blocks += 1;
        }
        if blocks > 1 {
            for magnitude in &mut magnitudes {
                *magnitude /= blocks as f32;
            }
        }
        let magnitudes_dbfs = magnitudes
            .iter()
            .map(|m| if *m > 1e-6 { 20.0 * m.log10() } else { -120.0 })
            .collect();

        Ok(SpectrumReport {
            bins,
            magnitudes,
            magnitudes_dbfs,
            blocks,
            sample_rate: rate,
        })
    }

    /// Render a waveform envelope (ASCII art) of the project mix. Each column
    /// reports the peak of the frames it covers, so transient detail survives
    /// downsampling.
    pub fn render_waveform(&self, width: usize, height: usize) -> Result<WaveformRender, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let buf = self.render_project_mixdown()?;
        let columns = width.clamp(8, 400);
        let rows = height.clamp(4, 200);
        let channels = usize::from(buf.channels().max(1));
        let frames = buf.samples().len() / channels;

        let mut peaks = Vec::with_capacity(columns);
        for column in 0..columns {
            let start = column * frames / columns;
            let end = ((column + 1) * frames / columns).max(start + 1).min(frames);
            let mut peak = 0.0f32;
            for frame in start..end {
                for channel in 0..channels {
                    peak = peak.max(buf.samples()[frame * channels + channel].abs());
                }
            }
            peaks.push(peak);
        }

        let mut ascii = String::new();
        for row in (0..rows).rev() {
            let threshold = (row as f32 + 0.5) / rows as f32;
            for peak in &peaks {
                ascii.push(if *peak >= threshold { '#' } else { ' ' });
            }
            ascii.push('\n');
        }

        Ok(WaveformRender {
            ascii,
            width: columns,
            height: rows,
            peak: buf.peak(),
            duration_secs: buf.duration_secs(),
        })
    }

    /// Render a spectrogram (ASCII time/frequency map) of the project mix.
    /// Each cell is the Goertzel energy of one log-spaced band in one time
    /// column, so the picture shows real frequency content over time.
    pub fn render_spectrogram(
        &self,
        width: usize,
        height: usize,
    ) -> Result<SpectrogramRender, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let buf = self.render_project_mixdown()?;
        let columns = width.clamp(8, 400);
        let rows = height.clamp(4, 128);
        let rate = buf.sample_rate();
        let channels = usize::from(buf.channels().max(1));
        let frames = buf.samples().len() / channels;
        let nyquist = rate as f32 / 2.0;
        let centers = log_spaced_centers(50.0, nyquist.min(16_000.0), rows);

        let mono: Vec<f32> = (0..frames)
            .map(|f| {
                (0..channels)
                    .map(|c| buf.samples()[f * channels + c])
                    .sum::<f32>()
                    / channels as f32
            })
            .collect();

        // One column per time slice, each holding the dB energy of every band.
        // Normalisation happens against the loudest cell once all are measured.
        let mut columns_db: Vec<Vec<f32>> = Vec::with_capacity(columns);
        let mut loudest = -120.0f32;
        if !mono.is_empty() {
            for column in 0..columns {
                let start = column * mono.len() / columns;
                let natural_end = ((column + 1) * mono.len() / columns).max(start + 1);
                // Keep at least one full 2048-frame window so low bands resolve.
                let end = (start + 2048).max(natural_end).min(mono.len());
                let window = &mono[start.min(mono.len() - 1)..end];
                let mut bands = Vec::with_capacity(centers.len());
                for center in &centers {
                    let power = goertzel_power(window, rate, *center);
                    let db = 10.0 * power.max(1e-12).log10();
                    loudest = loudest.max(db);
                    bands.push(db);
                }
                columns_db.push(bands);
            }
        } else {
            columns_db.resize(columns, vec![-120.0f32; rows]);
        }

        let range_db = 60.0f32;
        let levels = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
        let mut ascii = String::new();
        for band in (0..rows).rev() {
            for bands in &columns_db {
                let db = bands.get(band).copied().unwrap_or(-120.0);
                let normalized = ((db - (loudest - range_db)) / range_db).clamp(0.0, 1.0);
                let index = (normalized * (levels.len() - 1) as f32).round() as usize;
                ascii.push(levels[index.min(levels.len() - 1)]);
            }
            ascii.push('\n');
        }

        Ok(SpectrogramRender {
            ascii,
            width: columns,
            height: rows,
            band_centers_hz: centers,
            range_db,
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
            "Avg diff: {:.6}, Max diff: {:.6}, Differing samples: {}/{} ({:.1}%), Length diff: {} frames, Sample rates: {} Hz vs {} Hz",
            avg_diff,
            max_diff,
            diff_count,
            len,
            (diff_count as f32 / len as f32) * 100.0,
            length_diff,
            buf_a.sample_rate(),
            buf_b.sample_rate()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_interpolates_between_neighbours() {
        let values = [0.0, 10.0, 20.0, 30.0, 40.0];
        assert!((percentile(&values, 0.0) - 0.0).abs() < 1e-6);
        assert!((percentile(&values, 50.0) - 20.0).abs() < 1e-6);
        assert!((percentile(&values, 100.0) - 40.0).abs() < 1e-6);
        assert!((percentile(&values, 25.0) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn level_stats_reports_a_real_peak_and_activity_ratio() {
        let rate = 48_000;
        let mut samples = vec![0.0f32; rate as usize];
        // Half a second of a -6 dBFS-ish sine, half digital silence.
        for (i, sample) in samples.iter_mut().take(rate as usize / 2).enumerate() {
            *sample = 0.5 * (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / rate as f32).sin();
        }
        let stats = level_stats(&samples, rate, 1);
        assert!(
            (stats.peak_dbfs - (-6.02)).abs() < 0.2,
            "peak {}",
            stats.peak_dbfs
        );
        assert!(
            stats.noise_floor_dbfs <= -60.0,
            "floor {}",
            stats.noise_floor_dbfs
        );
        assert!(
            (stats.active_ratio - 0.5).abs() < 0.1,
            "active {}",
            stats.active_ratio
        );
        assert_eq!(stats.clipping_spikes, 0);

        samples[10] = 1.5;
        assert_eq!(level_stats(&samples, rate, 1).clipping_spikes, 1);
    }

    #[test]
    fn goertzel_power_peaks_at_the_probe_frequency() {
        let rate = 48_000;
        let signal: Vec<f32> = (0..4096)
            .map(|i| (i as f32 * 1000.0 * 2.0 * std::f32::consts::PI / rate as f32).sin())
            .collect();
        let on_tone = goertzel_power(&signal, rate, 1000.0);
        let off_tone = goertzel_power(&signal, rate, 2000.0);
        assert!(
            on_tone > off_tone * 10.0,
            "on-tone {on_tone} should dominate off-tone {off_tone}"
        );
    }

    #[test]
    fn log_spaced_centers_are_ordered_and_inside_the_range() {
        let centers = log_spaced_centers(50.0, 16_000.0, 16);
        assert_eq!(centers.len(), 16);
        assert!(centers.iter().all(|c| *c > 50.0 && *c < 16_000.0));
        assert!(centers.windows(2).all(|w| w[0] < w[1]));
    }
}
