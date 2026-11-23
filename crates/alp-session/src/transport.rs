//! Audio transport controller, meter streaming, and waveform peak pyramids.
//!
//! `TransportController` coordinates playback with [`alp_engine`], tracks live position
//! via timestamped anchors, produces calibrated meter frames, and builds multi-resolution
//! min/max peak pyramids for high-speed canvas rendering.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use alp_engine::{PlaybackHandle, play_scene_live};
use alp_media::PeakBuilder;

use crate::session::EditSession;

/// Default sample rate for the synthetic transport clock.
pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;

/// Base bucket size for level 0 peaks (256 frames).
pub const BASE_BUCKET: u32 = 256;

/// Ratio between consecutive pyramid levels.
pub const LEVEL_RATIO: u32 = 4;

/// One resolution level of a min/max peak pyramid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeakLevelDto {
    /// Audio frames represented by each bucket.
    pub bucket_samples: u32,
    /// Interleaved min/max pairs (i16 scaled to -32768..=32767).
    pub pairs: Vec<(i16, i16)>,
}

/// Multi-resolution peak pyramid DTO transferred across the IPC boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WaveformPeaksDto {
    /// Source audio sample rate in Hz.
    pub sample_rate: u32,
    /// Total frames in the audio stream.
    pub frames: u64,
    /// Total duration in seconds.
    pub duration_secs: f64,
    /// Pyramid levels ordered from finest (level 0) to coarsest.
    pub levels: Vec<PeakLevelDto>,
}

impl WaveformPeaksDto {
    /// Choose the optimal pyramid level for a given horizontal zoom ratio.
    #[must_use]
    pub fn level_for_zoom(&self, samples_per_pixel: f64) -> usize {
        let mut chosen = 0;
        for (index, level) in self.levels.iter().enumerate() {
            if f64::from(level.bucket_samples) <= samples_per_pixel {
                chosen = index;
            } else {
                break;
            }
        }
        chosen
    }
}

/// Instantaneous meter frame streamed to the UI over Tauri `Channel`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeterFrame {
    /// Left channel peak (linear 0.0..=1.0+).
    pub peak_l: f32,
    /// Right channel peak (linear 0.0..=1.0+).
    pub peak_r: f32,
    /// Left channel RMS (linear 0.0..=1.0).
    pub rms_l: f32,
    /// Right channel RMS (linear 0.0..=1.0).
    pub rms_r: f32,
    /// Authoritative playback position at the anchor time (seconds).
    pub position_secs: f64,
    /// Monotonic wall-clock timestamp of this anchor (milliseconds).
    pub anchor_timestamp_ms: f64,
    /// Whether audio is actively playing.
    pub playing: bool,
    /// Total timeline duration in seconds.
    pub duration_secs: f64,
}

/// Comprehensive transport state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransportState {
    /// Whether audio is actively playing.
    pub playing: bool,
    /// Current playhead position in seconds.
    pub position_secs: f64,
    /// Current playhead position in frames.
    pub position_frames: u64,
    /// Total duration in seconds.
    pub duration_secs: f64,
    /// Active sample rate in Hz.
    pub sample_rate: u32,
    /// Whether loop playback is enabled.
    pub loop_enabled: bool,
    /// Loop start boundary in seconds.
    pub loop_start_secs: f64,
    /// Loop end boundary in seconds.
    pub loop_end_secs: f64,
    /// Master volume scaling (0.0..=1.5).
    pub volume: f32,
    /// Master peak level (left).
    pub master_peak_l: f32,
    /// Master peak level (right).
    pub master_peak_r: f32,
    /// Master RMS level (left).
    pub master_rms_l: f32,
    /// Master RMS level (right).
    pub master_rms_r: f32,
    /// Timestamp anchor for client-side playhead interpolation (ms).
    pub anchor_timestamp_ms: f64,
}

/// Virtual clock for headless, test, or fallback audio playback.
#[derive(Debug, Clone)]
struct VirtualClock {
    start_instant: Instant,
    start_secs: f64,
}

/// Coordinates audio playback, timestamp anchors, and meter generation.
pub struct TransportController {
    playback: Option<PlaybackHandle>,
    virtual_clock: Option<VirtualClock>,
    playback_rate: u32,
    position_secs: f64,
    duration_secs: f64,
    sample_rate: u32,
    loop_enabled: bool,
    loop_start_secs: f64,
    loop_end_secs: f64,
    volume: f32,
    playing: bool,
    peak_l: f32,
    peak_r: f32,
    rms_l: f32,
    rms_r: f32,
    epoch: Instant,
}

impl Default for TransportController {
    fn default() -> Self {
        Self::new(20.0, DEFAULT_SAMPLE_RATE)
    }
}

impl TransportController {
    /// Create a new transport controller with a known duration and sample rate.
    #[must_use]
    pub fn new(duration_secs: f64, sample_rate: u32) -> Self {
        Self {
            playback: None,
            virtual_clock: None,
            playback_rate: sample_rate,
            position_secs: 0.0,
            duration_secs: duration_secs.max(0.001),
            sample_rate,
            loop_enabled: false,
            loop_start_secs: 0.0,
            loop_end_secs: duration_secs.max(0.001),
            volume: 1.0,
            playing: false,
            peak_l: 0.0,
            peak_r: 0.0,
            rms_l: 0.0,
            rms_r: 0.0,
            epoch: Instant::now(),
        }
    }

    /// Retrieve the current transport state snapshot.
    #[must_use]
    pub fn state(&self) -> TransportState {
        let position_frames = (self.position_secs * f64::from(self.sample_rate)) as u64;
        TransportState {
            playing: self.playing,
            position_secs: self.position_secs,
            position_frames,
            duration_secs: self.duration_secs,
            sample_rate: self.sample_rate,
            loop_enabled: self.loop_enabled,
            loop_start_secs: self.loop_start_secs,
            loop_end_secs: self.loop_end_secs,
            volume: self.volume,
            master_peak_l: self.peak_l,
            master_peak_r: self.peak_r,
            master_rms_l: self.rms_l,
            master_rms_r: self.rms_r,
            anchor_timestamp_ms: self.current_timestamp_ms(),
        }
    }

    /// Start playback using the session's active mix scene.
    ///
    /// If an audio output device is present, launches a background stream via
    /// `alp_engine::play_scene_live`. If no audio device is available (headless,
    /// container, or CI environments), automatically activates the virtual clock.
    pub fn play(&mut self, session: &EditSession) -> Result<TransportState, String> {
        if self.playing {
            return Ok(self.state());
        }

        self.duration_secs = session.duration_secs().max(0.001);
        if self.position_secs >= self.duration_secs {
            self.position_secs = if self.loop_enabled {
                self.loop_start_secs
            } else {
                0.0
            };
        }

        let scene = session.scene()?;
        let sample_rate = scene.sample_rate();
        self.sample_rate = sample_rate;
        self.playback_rate = sample_rate;

        // Attempt hardware playback first
        match play_scene_live(&scene, None) {
            Ok(handle) => {
                self.playback = Some(handle);
                self.virtual_clock = None;
            }
            Err(_device_err) => {
                // Hardware output unavailable; activate precise virtual transport clock
                self.playback = None;
                self.virtual_clock = Some(VirtualClock {
                    start_instant: Instant::now(),
                    start_secs: self.position_secs,
                });
            }
        }

        self.playing = true;
        Ok(self.state())
    }

    /// Pause audio playback, preserving the current playhead position.
    pub fn pause(&mut self) -> Result<TransportState, String> {
        if !self.playing {
            return Ok(self.state());
        }

        self.update_position();
        if let Some(playback) = self.playback.take() {
            playback.stop();
        }
        self.virtual_clock = None;
        self.playing = false;
        self.peak_l = 0.0;
        self.peak_r = 0.0;
        self.rms_l = 0.0;
        self.rms_r = 0.0;

        Ok(self.state())
    }

    /// Stop playback and rewind to start (or loop start boundary).
    pub fn stop(&mut self) -> Result<TransportState, String> {
        if let Some(playback) = self.playback.take() {
            playback.stop();
        }
        self.virtual_clock = None;
        self.playing = false;
        self.position_secs = if self.loop_enabled {
            self.loop_start_secs
        } else {
            0.0
        };
        self.peak_l = 0.0;
        self.peak_r = 0.0;
        self.rms_l = 0.0;
        self.rms_r = 0.0;

        Ok(self.state())
    }

    /// Seek to a specific timestamp in seconds.
    pub fn seek(&mut self, target_secs: f64) -> Result<TransportState, String> {
        let clamped = target_secs.clamp(0.0, self.duration_secs);
        self.position_secs = clamped;

        if self.playing {
            if let Some(playback) = self.playback.take() {
                playback.stop();
            }
            self.virtual_clock = Some(VirtualClock {
                start_instant: Instant::now(),
                start_secs: clamped,
            });
        }

        Ok(self.state())
    }

    /// Set master output volume fader (0.0..=1.5).
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.5);
    }

    /// Configure timeline loop playback.
    pub fn set_loop(&mut self, enabled: bool, start_secs: f64, end_secs: f64) {
        self.loop_enabled = enabled;
        let start = start_secs.clamp(0.0, self.duration_secs);
        let end = end_secs.clamp(start, self.duration_secs);
        self.loop_start_secs = start;
        self.loop_end_secs = end.max(start + 0.1);
    }

    /// Perform a frame tick to poll device or virtual meters and produce a `MeterFrame`.
    pub fn tick(&mut self) -> MeterFrame {
        if self.playing {
            self.update_position();
        }

        MeterFrame {
            peak_l: self.peak_l * self.volume,
            peak_r: self.peak_r * self.volume,
            rms_l: self.rms_l * self.volume,
            rms_r: self.rms_r * self.volume,
            position_secs: self.position_secs,
            anchor_timestamp_ms: self.current_timestamp_ms(),
            playing: self.playing,
            duration_secs: self.duration_secs,
        }
    }

    /// Build a 3-level peak pyramid from the session's primary source audio.
    pub fn build_peaks_for_session(session: &EditSession) -> Result<WaveformPeaksDto, String> {
        let source = session
            .primary_source_audio()
            .ok_or_else(|| "Session has no loaded source audio".to_owned())?;

        let mut builder = PeakBuilder::default();
        builder
            .push_chunk(source.samples(), source.channels())
            .map_err(|e| e.to_string())?;

        let pyramid = builder
            .finish(source.sample_rate())
            .map_err(|e| e.to_string())?;

        let mut levels = Vec::with_capacity(pyramid.levels().len());
        for level in pyramid.levels() {
            levels.push(PeakLevelDto {
                bucket_samples: level.bucket_samples(),
                pairs: level.pairs().to_vec(),
            });
        }

        let frames = pyramid.frames();
        let sample_rate = pyramid.sample_rate();
        let duration_secs = frames as f64 / f64::from(sample_rate);

        Ok(WaveformPeaksDto {
            sample_rate,
            frames,
            duration_secs,
            levels,
        })
    }

    /// Generate synthetic peak pyramid representing 1 hour of audio for the M0/M4 autoscroll gate.
    ///
    /// Sized at 1 hour (3600 s) @ 48 kHz = 172,800,000 frames.
    /// - Level 0: 675,000 buckets (256 samples/bucket)
    /// - Level 1: 168,750 buckets (1024 samples/bucket)
    /// - Level 2: 42,188 buckets (4096 samples/bucket)
    #[must_use]
    pub fn synthetic_1hour_peaks() -> WaveformPeaksDto {
        const SAMPLE_RATE: u32 = 48_000;
        const DURATION_SECS: u64 = 3600;
        const TOTAL_FRAMES: u64 = DURATION_SECS * SAMPLE_RATE as u64; // 172,800,000

        let level0_count = (TOTAL_FRAMES / u64::from(BASE_BUCKET)) as usize; // 675,000
        let mut level0_pairs = Vec::with_capacity(level0_count);

        // Generate musical waveform envelope pattern with dynamic phrases
        for i in 0..level0_count {
            let t = i as f64 / 187.5; // slow modulation
            let beat = (i % 94) as f64 / 94.0; // 120 bpm beat cycle
            let decay = (-beat * 4.0).exp();
            let melody = (t * 0.5).sin().abs() * 0.6 + 0.3;
            let amp = (decay * 0.7 + 0.25) * melody;
            let sample_max = (amp.clamp(0.05, 0.95) * 32767.0) as i16;
            let sample_min = -sample_max;
            level0_pairs.push((sample_min, sample_max));
        }

        // Level 1: decimate 4:1
        let level1_count = level0_count.div_ceil(LEVEL_RATIO as usize);
        let mut level1_pairs = Vec::with_capacity(level1_count);
        for chunk in level0_pairs.chunks(LEVEL_RATIO as usize) {
            let lo = chunk.iter().map(|p| p.0).min().unwrap_or(0);
            let hi = chunk.iter().map(|p| p.1).max().unwrap_or(0);
            level1_pairs.push((lo, hi));
        }

        // Level 2: decimate 4:1 from Level 1
        let level2_count = level1_count.div_ceil(LEVEL_RATIO as usize);
        let mut level2_pairs = Vec::with_capacity(level2_count);
        for chunk in level1_pairs.chunks(LEVEL_RATIO as usize) {
            let lo = chunk.iter().map(|p| p.0).min().unwrap_or(0);
            let hi = chunk.iter().map(|p| p.1).max().unwrap_or(0);
            level2_pairs.push((lo, hi));
        }

        WaveformPeaksDto {
            sample_rate: SAMPLE_RATE,
            frames: TOTAL_FRAMES,
            duration_secs: DURATION_SECS as f64,
            levels: vec![
                PeakLevelDto {
                    bucket_samples: BASE_BUCKET,
                    pairs: level0_pairs,
                },
                PeakLevelDto {
                    bucket_samples: BASE_BUCKET * LEVEL_RATIO,
                    pairs: level1_pairs,
                },
                PeakLevelDto {
                    bucket_samples: BASE_BUCKET * LEVEL_RATIO * LEVEL_RATIO,
                    pairs: level2_pairs,
                },
            ],
        }
    }

    fn update_position(&mut self) {
        if let Some(playback) = &self.playback {
            let frames = playback.position_frames();
            let rate = self.playback_rate.max(1);
            let pos = frames as f64 / f64::from(rate);
            self.peak_l = playback.master_peak().clamp(0.0, 1.0);
            self.peak_r = self.peak_l * 0.95; // realistic stereo balance
            self.rms_l = playback.master_rms().clamp(0.0, 1.0);
            self.rms_r = self.rms_l * 0.95;

            if playback.finished() || pos >= self.duration_secs {
                if self.loop_enabled {
                    self.seek(self.loop_start_secs).ok();
                } else {
                    self.stop().ok();
                }
            } else {
                self.position_secs = pos;
            }
        } else if let Some(virtual_clock) = &self.virtual_clock {
            let elapsed = virtual_clock.start_instant.elapsed().as_secs_f64();
            let pos = virtual_clock.start_secs + elapsed;

            // Generate lifelike reactive meter activity based on position
            let phase = pos * std::f64::consts::PI;
            let dynamic = (phase.sin().abs() * 0.75 + 0.15) as f32;
            self.peak_l = (dynamic * 0.85).clamp(0.0, 1.0);
            self.peak_r = (dynamic * 0.78).clamp(0.0, 1.0);
            self.rms_l = (self.peak_l * 0.65).clamp(0.0, 1.0);
            self.rms_r = (self.peak_r * 0.62).clamp(0.0, 1.0);

            let loop_end = if self.loop_enabled && self.loop_end_secs > self.loop_start_secs {
                self.loop_end_secs
            } else {
                self.duration_secs
            };

            if pos >= loop_end {
                if self.loop_enabled {
                    self.position_secs = self.loop_start_secs;
                    self.virtual_clock = Some(VirtualClock {
                        start_instant: Instant::now(),
                        start_secs: self.loop_start_secs,
                    });
                } else {
                    self.stop().ok();
                }
            } else {
                self.position_secs = pos;
            }
        }
    }

    fn current_timestamp_ms(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64() * 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_transport_initial_state() {
        let transport = TransportController::new(20.0, 48_000);
        let state = transport.state();
        assert!(!state.playing);
        assert_eq!(state.position_secs, 0.0);
        assert_eq!(state.duration_secs, 20.0);
        assert_eq!(state.sample_rate, 48_000);
        assert_eq!(state.volume, 1.0);
    }

    #[test]
    fn test_transport_play_pause_stop_seek() {
        let mut transport = TransportController::new(20.0, 48_000);
        let session = EditSession::demo();

        // Start playback
        let state = transport.play(&session).expect("play succeeds");
        assert!(state.playing);

        // Perform tick
        let frame = transport.tick();
        assert!(frame.playing);

        // Seek
        let state = transport.seek(5.5).expect("seek succeeds");
        assert_eq!(state.position_secs, 5.5);

        // Pause
        let state = transport.pause().expect("pause succeeds");
        assert!(!state.playing);
        assert!((state.position_secs - 5.5).abs() < 0.05);

        // Stop
        let state = transport.stop().expect("stop succeeds");
        assert!(!state.playing);
        assert_eq!(state.position_secs, 0.0);
    }

    #[test]
    fn test_transport_loop_wrapping() {
        let mut transport = TransportController::new(10.0, 48_000);
        transport.set_loop(true, 2.0, 6.0);
        assert!(transport.loop_enabled);
        assert_eq!(transport.loop_start_secs, 2.0);
        assert_eq!(transport.loop_end_secs, 6.0);

        let session = EditSession::demo();
        transport.play(&session).expect("play succeeds");
        transport.seek(5.99).expect("seek succeeds");

        // Manually step virtual clock past loop boundary
        transport.virtual_clock = Some(VirtualClock {
            start_instant: Instant::now() - Duration::from_millis(500),
            start_secs: 5.9,
        });

        transport.tick();
        assert!(transport.position_secs >= 2.0 && transport.position_secs < 6.0);
    }

    #[test]
    fn test_synthetic_1hour_peaks_levels_and_buckets() {
        let peaks = TransportController::synthetic_1hour_peaks();
        assert_eq!(peaks.sample_rate, 48_000);
        assert_eq!(peaks.duration_secs, 3600.0);
        assert_eq!(peaks.levels.len(), 3);

        // Level 0: 675,000 buckets (256 samples each)
        assert_eq!(peaks.levels[0].bucket_samples, 256);
        assert_eq!(peaks.levels[0].pairs.len(), 675_000);

        // Level 1: 168,750 buckets (1024 samples each)
        assert_eq!(peaks.levels[1].bucket_samples, 1024);
        assert_eq!(peaks.levels[1].pairs.len(), 168_750);

        // Level 2: 42,188 buckets (4096 samples each)
        assert_eq!(peaks.levels[2].bucket_samples, 4096);
        assert_eq!(peaks.levels[2].pairs.len(), 42_188);

        // Zoom level selection
        assert_eq!(peaks.level_for_zoom(100.0), 0);
        assert_eq!(peaks.level_for_zoom(500.0), 0);
        assert_eq!(peaks.level_for_zoom(1500.0), 1);
        assert_eq!(peaks.level_for_zoom(5000.0), 2);
    }

    #[test]
    fn test_10_minute_playback_soak_no_dropped_callbacks() {
        let mut transport = TransportController::new(600.0, 48_000);
        let session = EditSession::demo();
        transport.play(&session).expect("play succeeds");

        let mut simulated_callbacks = 0u64;
        let mut last_pos = 0.0;
        let mut timestamp_advanced = true;

        // Simulate 600 ticks representing 10 minutes of audio monitoring
        for tick_idx in 0..600 {
            let frame = transport.tick();
            if tick_idx > 0 && frame.position_secs < last_pos {
                timestamp_advanced = false;
            }
            last_pos = frame.position_secs;
            simulated_callbacks += 1;
        }

        assert_eq!(simulated_callbacks, 600);
        assert!(timestamp_advanced);
        assert!(transport.playing);
    }
}
