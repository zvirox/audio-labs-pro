//! Voice Activity Detection (VAD) for speech segmentation and live dictation.

use alp_core::FrameRange;
use alp_media::PcmBuffer;

/// Configuration for Voice Activity Detection.
#[derive(Debug, Clone, PartialEq)]
pub struct VadConfig {
    /// Energy threshold in dB FS below which signal is considered silence (e.g. -45.0 dB).
    pub threshold_db: f32,
    /// Hangover duration in milliseconds to prevent truncating word endings during brief pauses.
    pub hangover_ms: u32,
    /// Minimum duration in milliseconds for an activity burst to be classified as speech.
    pub min_speech_ms: u32,
    /// Analysis window size in milliseconds (typically 10ms to 30ms).
    pub frame_size_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            threshold_db: -42.0,
            hangover_ms: 250,
            min_speech_ms: 80,
            frame_size_ms: 20,
        }
    }
}

/// Streaming Voice Activity Detector.
#[derive(Debug, Clone)]
pub struct VoiceActivityDetector {
    config: VadConfig,
    sample_rate: u32,
    linear_threshold: f32,
    hangover_frames: u64,
    min_speech_frames: u64,
    consecutive_silence_frames: u64,
    consecutive_speech_frames: u64,
    in_speech: bool,
}

impl VoiceActivityDetector {
    /// Create a new VAD instance for a specific sample rate.
    #[must_use]
    pub fn new(sample_rate: u32, config: VadConfig) -> Self {
        let linear_threshold = 10.0f32.powf(config.threshold_db / 20.0);
        let hangover_frames = (config.hangover_ms as f64 * 0.001 * sample_rate as f64) as u64;
        let min_speech_frames = (config.min_speech_ms as f64 * 0.001 * sample_rate as f64) as u64;

        Self {
            config,
            sample_rate,
            linear_threshold,
            hangover_frames,
            min_speech_frames,
            consecutive_silence_frames: 0,
            consecutive_speech_frames: 0,
            in_speech: false,
        }
    }

    /// Return the VAD configuration.
    #[must_use]
    pub const fn config(&self) -> &VadConfig {
        &self.config
    }

    /// Return the configured sample rate.
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Process a block of samples (mono or interleaved) and return whether speech was detected.
    pub fn process_block(&mut self, samples: &[f32]) -> bool {
        if samples.is_empty() {
            return self.in_speech;
        }

        let mut sum_sq = 0.0f32;
        let mut zero_crossings = 0;
        let mut prev = samples[0];

        for &s in samples {
            sum_sq += s * s;
            if (s >= 0.0 && prev < 0.0) || (s < 0.0 && prev >= 0.0) {
                zero_crossings += 1;
            }
            prev = s;
        }

        let rms = (sum_sq / samples.len() as f32).sqrt();
        let zcr = zero_crossings as f32 / samples.len() as f32;

        // Speech has sufficient RMS energy and reasonable zero-crossing rate
        let is_active = rms >= self.linear_threshold && (0.002..=0.65).contains(&zcr);

        if is_active {
            self.consecutive_speech_frames += samples.len() as u64;
            self.consecutive_silence_frames = 0;
            if self.consecutive_speech_frames >= self.min_speech_frames {
                self.in_speech = true;
            }
        } else {
            self.consecutive_silence_frames += samples.len() as u64;
            if self.consecutive_silence_frames >= self.hangover_frames {
                self.in_speech = false;
                self.consecutive_speech_frames = 0;
            }
        }

        self.in_speech
    }

    /// Current speech state.
    #[must_use]
    pub fn is_speech(&self) -> bool {
        self.in_speech
    }

    /// Return the consecutive speech frames tracked so far.
    #[must_use]
    pub const fn consecutive_speech_frames(&self) -> u64 {
        self.consecutive_speech_frames
    }

    /// Reset internal detector state.
    pub fn reset(&mut self) {
        self.consecutive_silence_frames = 0;
        self.consecutive_speech_frames = 0;
        self.in_speech = false;
    }
}

/// Segment a complete audio buffer into discrete speech intervals.
#[must_use]
pub fn segment_speech(buffer: &PcmBuffer, config: &VadConfig) -> Vec<FrameRange> {
    if buffer.frames() == 0 {
        return Vec::new();
    }

    let sample_rate = buffer.sample_rate();
    let channels = buffer.channels() as usize;
    let window_frames =
        (config.frame_size_ms as f64 * 0.001 * sample_rate as f64).max(64.0) as usize;
    let window_samples = window_frames * channels;

    let mut detector = VoiceActivityDetector::new(sample_rate, config.clone());
    let mut segments = Vec::new();

    let samples = buffer.samples();
    let total_samples = samples.len();
    let mut pos = 0;
    let mut speech_start_frame: Option<u64> = None;

    while pos < total_samples {
        let chunk_len = window_samples.min(total_samples - pos);
        let chunk = &samples[pos..pos + chunk_len];
        let current_frame = (pos / channels) as u64;
        let end_of_chunk = current_frame + (chunk_len / channels) as u64;

        let was_speech = detector.is_speech();
        let is_speech = detector.process_block(chunk);

        if !was_speech && is_speech {
            let start = end_of_chunk.saturating_sub(detector.consecutive_speech_frames());
            speech_start_frame = Some(start);
        } else if was_speech
            && !is_speech
            && let Some(start) = speech_start_frame.take()
            && let Ok(range) = FrameRange::new(start, end_of_chunk)
        {
            segments.push(range);
        }

        pos += chunk_len;
    }

    // Flush any pending trailing speech segment
    if let Some(start) = speech_start_frame {
        let end = buffer.frames();
        if let Ok(range) = FrameRange::new(start, end) {
            segments.push(range);
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_speech_burst_between_silence() {
        let sample_rate = 48_000;
        let mut samples = Vec::new();

        // 0.5s silence
        samples.resize(24_000, 0.0);

        // 1.0s synthetic speech (200Hz tone at -12 dBFS)
        let amp = 0.25f32;
        for i in 0..48_000 {
            let s =
                amp * (2.0 * std::f32::consts::PI * 200.0 * (i as f32 / sample_rate as f32)).sin();
            samples.push(s);
        }

        // 0.5s silence
        samples.resize(samples.len() + 24_000, 0.0);

        let buf = PcmBuffer::new(sample_rate, 1, samples).expect("buffer");
        let config = VadConfig::default();
        let segments = segment_speech(&buf, &config);

        assert_eq!(segments.len(), 1, "expected exactly 1 speech segment");
        let seg = &segments[0];

        // Speech starts around frame 24,000 and ends around 72,000 (+hangover)
        assert!(seg.start >= 23_000 && seg.start <= 26_000);
        assert!(seg.end >= 72_000 && seg.end <= 90_000);
    }
}
