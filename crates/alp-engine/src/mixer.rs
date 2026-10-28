//! Real-time scene mixer.
//!
//! [`Mixer::render_into`] is the only code that runs inside the audio callback.
//! It performs no allocation, takes no locks, does no logging and never touches
//! the filesystem. All buffers and cursors are allocated by [`Mixer::new`].

use crate::scene::MixScene;
use alp_core::EnvelopePoint;
use std::sync::Arc;

/// Master bus peak limiter used by offline and live mixing.
#[derive(Debug, Clone)]
pub(crate) struct MasterLimiter {
    threshold: f32,
    release_ms: f32,
    sample_rate: u32,
    channels: u16,
    gain: f32,
}

impl MasterLimiter {
    pub(crate) fn new(threshold_db: f32, release_ms: f32) -> Self {
        Self {
            threshold: 10f32.powf(threshold_db / 20.0),
            release_ms,
            sample_rate: 0,
            channels: 0,
            gain: 1.0,
        }
    }

    pub(crate) fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.sample_rate = sample_rate;
        self.channels = channels.max(1);
        self.gain = 1.0;
    }

    pub(crate) fn process(&mut self, buffer: &mut [f32]) {
        let channels = self.channels.max(1) as usize;
        let mut index = 0;
        while index < buffer.len() {
            let mut peak = 0.0f32;
            for channel in 0..channels {
                if index + channel < buffer.len() {
                    peak = peak.max(buffer[index + channel].abs());
                }
            }
            let target_gain = if peak > self.threshold {
                self.threshold / peak
            } else {
                1.0
            };
            if target_gain < self.gain {
                self.gain = target_gain;
            } else {
                let coeff = (1.0
                    - (-1.0 / (self.release_ms * 0.001 * self.sample_rate as f32)).exp())
                .clamp(0.0, 1.0);
                self.gain = target_gain * coeff + self.gain * (1.0 - coeff);
            }
            for channel in 0..channels {
                if index + channel < buffer.len() {
                    buffer[index + channel] *= self.gain;
                }
            }
            index += channels;
        }
    }
}

/// A per-track level meter with relaxed atomics suitable for the UI thread.
#[derive(Clone)]
pub struct TrackMeter {
    peak: std::sync::Arc<std::sync::atomic::AtomicU32>,
    rms: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl TrackMeter {
    #[must_use]
    fn new() -> Self {
        Self {
            peak: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            rms: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }

    #[must_use]
    pub fn peak(&self) -> f32 {
        f32::from_bits(self.peak.load(std::sync::atomic::Ordering::Relaxed))
    }

    #[must_use]
    pub fn rms(&self) -> f32 {
        f32::from_bits(self.rms.load(std::sync::atomic::Ordering::Relaxed))
    }

    fn update(&self, peak: f32, rms: f32) {
        let peak = peak.clamp(0.0, 1.0);
        let rms = rms.clamp(0.0, 1.0);
        self.peak
            .store(peak.to_bits(), std::sync::atomic::Ordering::Relaxed);
        self.rms
            .store(rms.to_bits(), std::sync::atomic::Ordering::Relaxed);
    }
}

/// Ordered per-track meters plus the master meter.
#[derive(Clone)]
pub struct TrackMeterBank {
    /// Track ids in mixer order.
    pub track_ids: std::vec::Vec<alp_core::TrackId>,
    /// Per-track meters.
    pub track_meters: std::vec::Vec<std::sync::Arc<TrackMeter>>,
    /// Master bus meter.
    pub master: std::sync::Arc<TrackMeter>,
}

impl TrackMeterBank {
    fn new(track_ids: std::vec::Vec<alp_core::TrackId>) -> Self {
        let mut meters = Vec::with_capacity(track_ids.len());
        for _ in &track_ids {
            meters.push(std::sync::Arc::new(TrackMeter::new()));
        }
        Self {
            track_ids,
            track_meters: meters,
            master: std::sync::Arc::new(TrackMeter::new()),
        }
    }
}

/// Mixes a prepared [`MixScene`] into interleaved output.
pub struct Mixer {
    scene: MixScene,
    position: u64,
    envelope_cursors: Vec<usize>,
    master_gain: f32,
    master_limiter: Option<MasterLimiter>,
    track_meter_indices: Vec<Option<usize>>,
    track_meter_bank: Arc<TrackMeterBank>,
    track_peak: Vec<f32>,
    track_sum: Vec<f64>,
    track_samples: Vec<u64>,
    master_plugin_chain: Option<alp_plugin::TrackPluginChain>,
}

impl Mixer {
    /// Prepare cursors for a scene. Allocation happens here, not in the callback.
    #[must_use]
    pub fn new(scene: MixScene) -> Self {
        Self::with_master(scene, 1.0, None, 1.0, 100.0)
    }

    /// Prepare a scene with a master gain and an optional limiter. The limiter is
    /// specified as a threshold in dB FS plus attack and release in milliseconds.
    #[must_use]
    pub fn with_master(
        scene: MixScene,
        master_gain: f32,
        limiter_threshold_db: Option<f32>,
        _attack_ms: f32,
        release_ms: f32,
    ) -> Self {
        let envelope_cursors = scene
            .clips()
            .iter()
            .map(|clip| cursor_at(&clip.envelope, 0))
            .collect();
        let mut master_limiter = None;
        if let Some(threshold_db) = limiter_threshold_db {
            let mut limiter = MasterLimiter::new(threshold_db, release_ms);
            limiter.prepare(scene.sample_rate(), scene.channels());
            master_limiter = Some(limiter);
        }
        let track_ids = scene.tracks();
        let track_meter_bank = Arc::new(TrackMeterBank::new(track_ids));
        let track_meter_indices = scene
            .clips()
            .iter()
            .map(|clip| {
                clip.track_id
                    .and_then(|tid| track_meter_bank.track_ids.iter().position(|id| tid == *id))
            })
            .collect();
        let track_count = track_meter_bank.track_ids.len();
        Self {
            scene,
            position: 0,
            envelope_cursors,
            master_gain,
            master_limiter,
            track_meter_indices,
            track_meter_bank,
            track_peak: vec![0.0; track_count],
            track_sum: vec![0.0; track_count],
            track_samples: vec![0; track_count],
            master_plugin_chain: None,
        }
    }

    /// Attach a plugin processing chain to the master bus.
    pub fn set_master_plugin_chain(&mut self, chain: Option<alp_plugin::TrackPluginChain>) {
        self.master_plugin_chain = chain;
    }

    /// Track and master level meters.
    #[must_use]
    pub fn track_meters(&self) -> Arc<TrackMeterBank> {
        Arc::clone(&self.track_meter_bank)
    }

    /// Current timeline position in frames.
    #[must_use]
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Total scene length in frames.
    #[must_use]
    pub fn total_frames(&self) -> u64 {
        self.scene.total_frames()
    }

    /// Whether the playhead reached the end.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.position >= self.scene.total_frames()
    }

    /// Jump to an absolute frame.
    pub fn seek(&mut self, frame: u64) {
        self.position = frame.min(self.scene.total_frames());
        for (clip, cursor) in self
            .scene
            .clips()
            .iter()
            .zip(self.envelope_cursors.iter_mut())
        {
            let clip_frame = self.position.saturating_sub(clip.timeline_start);
            *cursor = cursor_at(&clip.envelope, clip_frame);
        }
    }

    /// Mix the next `out.len() / channels` frames into `out`.
    ///
    /// `out` must be frame-aligned. Samples after the scene end are zeroed.
    /// Returns the number of frames written.
    ///
    /// # Panics
    ///
    /// Panics if `out.len()` is not a multiple of the scene channel count. This
    /// is a programming error and cannot happen in the device callback, where
    /// the buffer length always matches the stream configuration.
    pub fn render_into(&mut self, out: &mut [f32]) -> usize {
        let channels = usize::from(self.scene.channels());
        assert!(
            out.len().is_multiple_of(channels),
            "output buffer is not frame-aligned"
        );
        out.fill(0.0);
        let frames = out.len() / channels;
        if frames == 0 {
            return 0;
        }
        self.track_peak.fill(0.0);
        self.track_sum.fill(0.0);
        let start = self.position;
        let end = start
            .saturating_add(frames as u64)
            .min(self.scene.total_frames());

        for (clip_index, clip) in self.scene.clips().iter().enumerate() {
            let meter_index = self.track_meter_indices[clip_index];
            let clip_start = clip.timeline_start;
            let clip_end = clip.timeline_end();
            let overlap_start = start.max(clip_start);
            let overlap_end = end.min(clip_end);
            if overlap_start >= overlap_end {
                continue;
            }

            let source_channels = usize::from(clip.source.channels());
            let source_samples = clip.source.samples();
            let clip_len = clip.len();
            let cursor = &mut self.envelope_cursors[clip_index];

            for timeline_frame in overlap_start..overlap_end {
                let clip_frame = timeline_frame - clip_start;
                let source_frame = if clip.reversed {
                    clip.source_end.saturating_sub(1).saturating_sub(clip_frame)
                } else {
                    clip.source_start + clip_frame
                };
                let source_index = source_frame as usize * source_channels;
                if source_index + source_channels > source_samples.len() {
                    continue;
                }

                let mut gain = clip.gain;
                if clip.fade_in_frames > 0 && clip_frame < clip.fade_in_frames {
                    gain *= clip_frame as f32 / clip.fade_in_frames as f32;
                }
                let remaining = clip_len - clip_frame;
                if clip.fade_out_frames > 0 && remaining <= clip.fade_out_frames {
                    gain *= remaining as f32 / clip.fade_out_frames as f32;
                }
                gain *= envelope_gain(&clip.envelope, cursor, clip_frame);
                let polarity = if clip.inverted { -1.0 } else { 1.0 };

                let destination = (timeline_frame - start) as usize * channels;
                if source_channels == 1 {
                    let value = source_samples[source_index] * gain * polarity;
                    for channel in 0..channels {
                        out[destination + channel] += value * pan_gain(clip.pan, channel, channels);
                    }
                    if let Some(meter_index) = meter_index {
                        let channel_value = (value * pan_gain(clip.pan, 0, channels)).abs();
                        self.track_peak[meter_index] =
                            self.track_peak[meter_index].max(channel_value);
                        self.track_sum[meter_index] += f64::from(value) * f64::from(value);
                        self.track_samples[meter_index] += 1;
                    }
                } else {
                    for channel in 0..channels.min(source_channels) {
                        let value = source_samples[source_index + channel]
                            * gain
                            * polarity
                            * pan_gain(clip.pan, channel, channels);
                        out[destination + channel] += value;
                        if let Some(meter_index) = meter_index {
                            self.track_peak[meter_index] =
                                self.track_peak[meter_index].max(value.abs());
                            self.track_sum[meter_index] += f64::from(value) * f64::from(value);
                            self.track_samples[meter_index] += 1;
                        }
                    }
                }
            }
        }

        let written_frames = end.saturating_sub(start) as usize;
        let written_samples = written_frames * channels;
        if self.master_gain != 1.0 {
            for sample in &mut out[..written_samples] {
                *sample *= self.master_gain;
            }
        }
        if let Some(chain) = &mut self.master_plugin_chain {
            let ctx = alp_plugin::ProcessContext {
                sample_rate: self.scene.sample_rate(),
                num_frames: written_frames,
                frame_position: start,
                is_playing: true,
            };
            let _ = chain.process_interleaved(&mut out[..written_samples], channels, &ctx);
        }
        if let Some(limiter) = &mut self.master_limiter {
            limiter.process(&mut out[..written_samples]);
        }

        for (clip_index, track_meter_index) in self.track_meter_indices.iter().enumerate() {
            if let Some(meter_index) = track_meter_index {
                let peak = self.track_peak[*meter_index];
                let frame_count = written_frames.max(1) as u64;
                let rms = (self.track_sum[*meter_index] / frame_count as f64).sqrt() as f32;
                let _ = clip_index;
                self.track_meter_bank.track_meters[*meter_index].update(peak, rms.clamp(0.0, 1.0));
            }
        }
        let (master_peak, master_rms) = if written_samples == 0 {
            (0.0, 0.0)
        } else {
            let mut peak = 0.0f32;
            let mut sum = 0.0f64;
            for sample in &out[..written_samples] {
                peak = peak.max(sample.abs());
                sum += f64::from(*sample) * f64::from(*sample);
            }
            let rms = (sum / written_samples as f64).sqrt() as f32;
            (peak, rms)
        };
        self.track_meter_bank.master.update(master_peak, master_rms);

        self.track_peak.fill(0.0);
        self.track_sum.fill(0.0);
        self.position = end;
        frames
    }
}

fn cursor_at(points: &[EnvelopePoint], frame: u64) -> usize {
    points
        .partition_point(|point| point.frame <= frame)
        .saturating_sub(1)
}

/// Linear balance gain for one output channel.
///
/// `pan` is `-1.0` left, `0.0` center and `1.0` right. Channels beyond a stereo
/// pair are left unpanned until a surround effect chain exists.
#[must_use]
fn pan_gain(pan: f32, channel: usize, channels: usize) -> f32 {
    if channels <= 1 || channel > 1 {
        return 1.0;
    }
    if channel == 0 {
        if pan <= 0.0 { 1.0 } else { 1.0 - pan }
    } else if pan >= 0.0 {
        1.0
    } else {
        1.0 + pan
    }
}

fn envelope_gain(points: &[EnvelopePoint], cursor: &mut usize, frame: u64) -> f32 {
    if points.is_empty() {
        return 1.0;
    }
    if frame <= points[0].frame {
        return points[0].gain;
    }
    while *cursor + 1 < points.len() && points[*cursor + 1].frame <= frame {
        *cursor += 1;
    }
    let current = points[*cursor];
    let Some(next) = points.get(*cursor + 1) else {
        return current.gain;
    };
    if frame >= next.frame || next.frame == current.frame {
        return next.gain;
    }
    let span = (next.frame - current.frame) as f32;
    let offset = (frame - current.frame) as f32;
    current.gain + (next.gain - current.gain) * (offset / span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{SceneClip, SourceAudio};
    use std::sync::Arc;

    fn source(frames: usize) -> Arc<SourceAudio> {
        Arc::new(SourceAudio::from_samples(48_000, 1, vec![1.0; frames]).expect("source"))
    }

    fn clip(start: u64, end: u64, timeline: u64, gain: f32) -> SceneClip {
        SceneClip {
            source: source(end as usize),
            source_start: start,
            source_end: end,
            timeline_start: timeline,
            gain,
            fade_in_frames: 0,
            fade_out_frames: 0,
            envelope: Vec::new(),
            reversed: false,
            inverted: false,
            pan: 0.0,
            track_id: None,
        }
    }

    #[test]
    fn sums_overlapping_clips() {
        let scene = MixScene::new(48_000, 2, vec![clip(0, 50, 0, 0.25), clip(0, 100, 50, 0.5)])
            .expect("scene");
        let mut mixer = Mixer::new(scene);
        let mut output = vec![0.0f32; 100 * 2];
        mixer.render_into(&mut output);
        assert!((output[0] - 0.25).abs() < 1e-6);
        assert!((output[50 * 2] - 0.5).abs() < 1e-6);
        assert!((output[99 * 2] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn applies_linear_fades() {
        let mut faded = clip(0, 100, 0, 1.0);
        faded.fade_in_frames = 10;
        faded.fade_out_frames = 10;
        let scene = MixScene::new(48_000, 1, vec![faded]).expect("scene");
        let mut mixer = Mixer::new(scene);
        let mut output = vec![0.0f32; 100];
        mixer.render_into(&mut output);
        assert!(output[0].abs() < 1e-6);
        assert!((output[5] - 0.5).abs() < 1e-6);
        assert!((output[50] - 1.0).abs() < 1e-6);
        assert!((output[95] - 0.5).abs() < 1e-6);
        assert!((output[99] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn interpolates_the_envelope() {
        let mut automated = clip(0, 100, 0, 1.0);
        automated.envelope = vec![
            EnvelopePoint {
                frame: 0,
                gain: 0.0,
            },
            EnvelopePoint {
                frame: 100,
                gain: 1.0,
            },
        ];
        let scene = MixScene::new(48_000, 1, vec![automated]).expect("scene");
        let mut mixer = Mixer::new(scene);
        let mut output = vec![0.0f32; 100];
        mixer.render_into(&mut output);
        assert!(output[0].abs() < 1e-6);
        assert!((output[50] - 0.5).abs() < 1e-6);
        assert!((output[99] - 0.99).abs() < 1e-6);
    }

    #[test]
    fn reverse_reads_the_clip_backwards() {
        let source = Arc::new(
            SourceAudio::from_samples(48_000, 1, vec![1.0, 2.0, 3.0, 4.0]).expect("source"),
        );
        let mut reversed = clip(0, 4, 0, 1.0);
        reversed.source = source;
        reversed.reversed = true;
        let scene = MixScene::new(48_000, 1, vec![reversed]).expect("scene");
        let mut mixer = Mixer::new(scene);
        let mut output = vec![0.0f32; 4];
        mixer.render_into(&mut output);
        assert_eq!(output, vec![4.0, 3.0, 2.0, 1.0]);
    }

    #[test]
    fn invert_flips_the_sample_sign() {
        let source =
            Arc::new(SourceAudio::from_samples(48_000, 1, vec![0.5, -0.25, 0.75]).expect("source"));
        let mut inverted = clip(0, 3, 0, 1.0);
        inverted.source = source;
        inverted.inverted = true;
        let scene = MixScene::new(48_000, 1, vec![inverted]).expect("scene");
        let mut mixer = Mixer::new(scene);
        let mut output = vec![0.0f32; 3];
        mixer.render_into(&mut output);
        assert!((output[0] + 0.5).abs() < 1e-6);
        assert!((output[1] - 0.25).abs() < 1e-6);
        assert!((output[2] + 0.75).abs() < 1e-6);
    }

    #[test]
    fn mixing_in_blocks_matches_one_call() {
        let scene = MixScene::new(
            48_000,
            2,
            vec![clip(0, 1_000, 0, 0.5), clip(0, 1_000, 500, 0.25)],
        )
        .expect("scene");
        let mut one = Mixer::new(scene.clone());
        let mut whole = vec![0.0f32; 1_500 * 2];
        one.render_into(&mut whole);

        let mut blocked = Mixer::new(scene);
        let mut output = vec![0.0f32; 1_500 * 2];
        for block in output.chunks_mut(128 * 2) {
            blocked.render_into(block);
        }
        assert_eq!(whole, output);
    }

    #[test]
    fn pan_follows_a_balance_law() {
        let mut left = clip(0, 1, 0, 1.0);
        left.pan = -1.0;
        let scene = MixScene::new(48_000, 2, vec![left]).expect("scene");
        let mut mixer = Mixer::new(scene);
        let mut output = vec![0.0f32; 2];
        mixer.render_into(&mut output);
        assert!((output[0] - 1.0).abs() < 1e-6);
        assert!(output[1].abs() < 1e-6);

        let mut right = clip(0, 1, 0, 1.0);
        right.pan = 1.0;
        let scene = MixScene::new(48_000, 2, vec![right]).expect("scene");
        let mut mixer = Mixer::new(scene);
        let mut output = vec![0.0f32; 2];
        mixer.render_into(&mut output);
        assert!(output[0].abs() < 1e-6);
        assert!((output[1] - 1.0).abs() < 1e-6);

        let mut center = clip(0, 1, 0, 1.0);
        center.pan = 0.0;
        let scene = MixScene::new(48_000, 2, vec![center]).expect("scene");
        let mut mixer = Mixer::new(scene);
        let mut output = vec![0.0f32; 2];
        mixer.render_into(&mut output);
        assert!((output[0] - 1.0).abs() < 1e-6);
        assert!((output[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pan_is_ignored_for_mono_output() {
        let mut panned = clip(0, 1, 0, 1.0);
        panned.pan = 1.0;
        let scene = MixScene::new(48_000, 1, vec![panned]).expect("scene");
        let mut mixer = Mixer::new(scene);
        let mut output = vec![0.0f32; 1];
        mixer.render_into(&mut output);
        assert!((output[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn master_gain_and_limiter_cap_output() {
        let sound = Arc::new(SourceAudio::from_samples(48_000, 1, vec![1.0]).expect("source"));
        let mut loud = clip(0, 1, 0, 1.0);
        loud.source = sound;
        let scene = MixScene::new(48_000, 1, vec![loud]).expect("scene");
        let mut mixer = Mixer::with_master(scene, 0.5, Some(-12.0), 1.0, 50.0);
        let mut output = vec![0.0f32; 1];
        mixer.render_into(&mut output);
        assert!(output[0].abs() <= 0.252, "output {}", output[0]);
        assert!(output[0] >= 0.0);
    }
}
