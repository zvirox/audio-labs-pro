//! Project-to-scene conversion and preloaded source audio.
//!
//! The audio callback must never decode or allocate, so every source is decoded
//! and resampled before playback. A [`MixScene`] is an immutable, fully prepared
//! snapshot that both the device callback and the offline renderer consume.

use std::collections::HashMap;
use std::sync::Arc;

use alp_core::{EnvelopePoint, FrameRange, Project, SourceId, frames_to_secs, secs_to_frames};
use alp_media::{PcmBuffer, resample};

use crate::error::EngineError;

/// Decoded audio shared by every clip that references it.
#[derive(Debug, Clone)]
pub struct SourceAudio {
    sample_rate: u32,
    channels: u16,
    samples: Arc<[f32]>,
}

impl SourceAudio {
    /// Copy an interleaved buffer into shared storage.
    #[must_use]
    pub fn from_pcm(buffer: &PcmBuffer) -> Self {
        Self {
            sample_rate: buffer.sample_rate(),
            channels: buffer.channels(),
            samples: Arc::from(buffer.samples()),
        }
    }

    /// Build from an owned interleaved sample vector.
    pub fn from_samples(
        sample_rate: u32,
        channels: u16,
        samples: Vec<f32>,
    ) -> Result<Self, EngineError> {
        let buffer = PcmBuffer::new(sample_rate, channels, samples)?;
        Ok(Self::from_pcm(&buffer))
    }

    /// Native sample rate.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Native channel count.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Length in frames.
    #[must_use]
    pub fn frames(&self) -> u64 {
        self.samples.len() as u64 / u64::from(self.channels)
    }

    /// Interleaved samples.
    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Convert back to an owned PCM buffer (used by offline resampling).
    fn to_pcm(&self) -> Result<PcmBuffer, EngineError> {
        PcmBuffer::new(self.sample_rate, self.channels, self.samples.to_vec())
            .map_err(EngineError::from)
    }
}

/// One clip prepared for playback.
#[derive(Debug, Clone)]
pub struct SceneClip {
    /// Preloaded source audio.
    pub source: Arc<SourceAudio>,
    /// First source frame, inclusive.
    pub source_start: u64,
    /// One past the last source frame.
    pub source_end: u64,
    /// First timeline frame.
    pub timeline_start: u64,
    /// Linear clip gain including track gain.
    pub gain: f32,
    /// Fade-in length in frames.
    pub fade_in_frames: u64,
    /// Fade-out length in frames.
    pub fade_out_frames: u64,
    /// Volume automation relative to the clip start.
    pub envelope: Vec<EnvelopePoint>,
    /// Play the source range backwards.
    pub reversed: bool,
    /// Flip the sample sign during playback.
    pub inverted: bool,
    /// Track pan position, `-1.0` left to `1.0` right.
    pub pan: f32,
    /// Owning track id, if this clip belongs to a timeline track.
    pub track_id: Option<alp_core::TrackId>,
}

impl SceneClip {
    /// Clip length in frames.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.source_end.saturating_sub(self.source_start)
    }

    /// Whether the clip contains no frames.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.source_end <= self.source_start
    }

    /// First frame after the clip on the timeline.
    #[must_use]
    pub fn timeline_end(&self) -> u64 {
        self.timeline_start.saturating_add(self.len())
    }

    /// Timeline range covered by the clip.
    #[must_use]
    pub fn timeline_range(&self) -> FrameRange {
        FrameRange {
            start: self.timeline_start,
            end: self.timeline_end(),
        }
    }
}

/// An immutable playback snapshot.
#[derive(Debug, Clone)]
pub struct MixScene {
    sample_rate: u32,
    channels: u16,
    clips: Vec<SceneClip>,
    total_frames: u64,
}

impl MixScene {
    /// Assemble a scene from prepared clips.
    pub fn new(
        sample_rate: u32,
        channels: u16,
        clips: Vec<SceneClip>,
    ) -> Result<Self, EngineError> {
        if sample_rate == 0 || channels == 0 {
            return Err(EngineError::InvalidScene(
                "sample rate and channel count must be non-zero".into(),
            ));
        }
        for clip in &clips {
            if clip.is_empty() {
                return Err(EngineError::InvalidScene("clip has no frames".into()));
            }
            if clip.source_end > clip.source.frames() {
                return Err(EngineError::InvalidScene(
                    "clip reads past the end of its source".into(),
                ));
            }
            if !clip.gain.is_finite() || clip.gain < 0.0 {
                return Err(EngineError::InvalidScene("clip gain is invalid".into()));
            }
            if !clip.pan.is_finite() || !(-1.0..=1.0).contains(&clip.pan) {
                return Err(EngineError::InvalidScene("clip pan is invalid".into()));
            }
            if clip.fade_in_frames.saturating_add(clip.fade_out_frames) > clip.len() {
                return Err(EngineError::InvalidScene(
                    "clip fades are longer than the clip".into(),
                ));
            }
        }
        let total_frames = clips.iter().map(SceneClip::timeline_end).max().unwrap_or(0);
        Ok(Self {
            sample_rate,
            channels,
            clips,
            total_frames,
        })
    }

    /// Build a scene from a project and preloaded sources.
    ///
    /// Muted tracks are skipped; if any track is soloed only soloed tracks are
    /// included.
    pub fn from_project(
        project: &Project,
        sources: &HashMap<SourceId, Arc<SourceAudio>>,
    ) -> Result<Self, EngineError> {
        Self::from_tracks(project, sources, None)
    }

    /// Build a scene for one track only.
    pub fn track_scene(
        project: &Project,
        track_id: alp_core::TrackId,
        sources: &HashMap<SourceId, Arc<SourceAudio>>,
    ) -> Result<Self, EngineError> {
        Self::from_tracks(project, sources, Some(track_id))
    }

    fn from_tracks(
        project: &Project,
        sources: &HashMap<SourceId, Arc<SourceAudio>>,
        track_filter: Option<alp_core::TrackId>,
    ) -> Result<Self, EngineError> {
        let any_solo = project.tracks.iter().any(|track| track.solo);
        let mut clips = Vec::new();
        let mut resampled_cache: HashMap<SourceId, Arc<SourceAudio>> = HashMap::new();

        for track in &project.tracks {
            if let Some(track_id) = track_filter
                && track.id != track_id
            {
                continue;
            }
            if track.muted || (any_solo && !track.solo) {
                continue;
            }
            for clip in &track.clips {
                let source = sources
                    .get(&clip.source_id)
                    .ok_or_else(|| {
                        EngineError::InvalidScene(format!(
                            "source {} was not preloaded",
                            clip.source_id
                        ))
                    })?
                    .clone();

                let native_rate = source.sample_rate();
                let needs_resample = native_rate != project.sample_rate;
                let source = if !needs_resample {
                    source
                } else if let Some(cached) = resampled_cache.get(&clip.source_id) {
                    cached.clone()
                } else {
                    let converted = remap_source(&source, project.sample_rate)?;
                    resampled_cache.insert(clip.source_id, converted.clone());
                    converted
                };

                let scale = |value: u64| {
                    secs_to_frames(frames_to_secs(value, native_rate), project.sample_rate)
                };
                let source_frames = source.frames();
                let source_start = if needs_resample {
                    scale(clip.source_range.start)
                } else {
                    clip.source_range.start
                }
                .min(source_frames.saturating_sub(1));
                let source_end = if needs_resample {
                    scale(clip.source_range.end)
                } else {
                    clip.source_range.end
                }
                .max(source_start + 1)
                .min(source_frames);
                if source_end <= source_start {
                    return Err(EngineError::InvalidScene(format!(
                        "clip {} is empty after resampling",
                        clip.id
                    )));
                }
                let len = source_end - source_start;
                let fade_in_frames = if needs_resample {
                    scale(clip.fade_in_frames)
                } else {
                    clip.fade_in_frames
                }
                .min(len);
                let fade_out_frames = if needs_resample {
                    scale(clip.fade_out_frames)
                } else {
                    clip.fade_out_frames
                }
                .min(len - fade_in_frames);
                let envelope = if needs_resample {
                    clip.envelope
                        .iter()
                        .map(|point| EnvelopePoint {
                            frame: scale(point.frame),
                            gain: point.gain,
                        })
                        .collect()
                } else {
                    clip.envelope.clone()
                };

                clips.push(SceneClip {
                    source,
                    source_start,
                    source_end,
                    timeline_start: clip.timeline_start,
                    gain: clip.gain * track.gain,
                    fade_in_frames,
                    fade_out_frames,
                    envelope,
                    reversed: clip.reversed,
                    inverted: clip.inverted,
                    pan: track.pan,
                    track_id: Some(track.id),
                });
            }
        }

        Self::new(project.sample_rate, 2, clips)
    }

    /// Sorted, unique timeline-track ids represented by this scene.
    #[must_use]
    pub fn tracks(&self) -> Vec<alp_core::TrackId> {
        let mut ids: Vec<_> = self
            .clips()
            .iter()
            .filter_map(|clip| clip.track_id)
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// Sample rate of the scene.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Output channel count.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Prepared clips.
    #[must_use]
    pub fn clips(&self) -> &[SceneClip] {
        &self.clips
    }

    /// Total length in frames.
    #[must_use]
    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// Total length in seconds.
    #[must_use]
    pub fn duration_secs(&self) -> f64 {
        self.total_frames as f64 / f64::from(self.sample_rate)
    }

    /// Reinterpret the scene at a different output channel count.
    #[must_use]
    pub fn with_channels(&self, channels: u16) -> Self {
        let mut scene = self.clone();
        scene.channels = channels.max(1);
        scene
    }

    /// Resample every source for a device running at `target_rate`.
    pub fn resampled(&self, target_rate: u32) -> Result<Self, EngineError> {
        if target_rate == self.sample_rate {
            return Ok(self.clone());
        }
        if target_rate == 0 {
            return Err(EngineError::InvalidScene("target rate is zero".into()));
        }
        let mut cache: HashMap<usize, Arc<SourceAudio>> = HashMap::new();
        let mut clips = Vec::with_capacity(self.clips.len());
        let scale = |value: u64| -> u64 {
            (value as f64 * f64::from(target_rate) / f64::from(self.sample_rate)).round() as u64
        };
        for clip in &self.clips {
            let key = Arc::as_ptr(&clip.source) as usize;
            let source = if let Some(cached) = cache.get(&key) {
                cached.clone()
            } else {
                let converted = remap_source(&clip.source, target_rate)?;
                cache.insert(key, converted.clone());
                converted
            };
            let source_frames = source.frames();
            let source_start = scale(clip.source_start).min(source_frames.saturating_sub(1));
            let source_end = scale(clip.source_end)
                .max(source_start + 1)
                .min(source_frames);
            let len = source_end.saturating_sub(source_start);
            let fade_in_frames = scale(clip.fade_in_frames).min(len);
            let fade_out_frames = scale(clip.fade_out_frames).min(len - fade_in_frames);
            let timeline_start = scale(clip.timeline_start);
            let mut envelope = clip.envelope.clone();
            for point in &mut envelope {
                point.frame = scale(point.frame);
            }
            clips.push(SceneClip {
                source,
                source_start,
                source_end,
                timeline_start,
                gain: clip.gain,
                fade_in_frames,
                fade_out_frames,
                envelope,
                reversed: clip.reversed,
                inverted: clip.inverted,
                pan: clip.pan,
                track_id: clip.track_id,
            });
        }
        Self::new(target_rate, self.channels, clips)
    }
}

fn remap_source(
    source: &Arc<SourceAudio>,
    target_rate: u32,
) -> Result<Arc<SourceAudio>, EngineError> {
    let pcm = source.to_pcm()?;
    let converted = resample(&pcm, target_rate)?;
    Ok(Arc::new(SourceAudio::from_pcm(&converted)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alp_core::{Clip, ClipId, Source, Track, TrackId};

    fn source(channels: u16) -> Arc<SourceAudio> {
        let frames = 4_800;
        let samples = (0..frames * usize::from(channels))
            .map(|index| (index as f32 / 100.0).sin() * 0.5)
            .collect();
        Arc::new(SourceAudio::from_samples(48_000, channels, samples).expect("source"))
    }

    #[test]
    fn project_conversion_skips_muted_tracks() {
        let mut project = Project::new("scene").expect("project");
        project.sources.push(
            Source::new(
                SourceId::from_u128(1),
                "a.wav",
                "media/a.wav",
                48_000,
                1,
                4_800,
            )
            .expect("source"),
        );
        let mut track = Track::new(TrackId::from_u128(2), "A");
        track.clips.push(Clip::new(
            ClipId::from_u128(3),
            SourceId::from_u128(1),
            FrameRange::new(0, 4_800).expect("range"),
            0,
        ));
        track.muted = true;
        project.tracks.push(track);

        let mut sources = HashMap::new();
        sources.insert(SourceId::from_u128(1), source(1));
        let scene = MixScene::from_project(&project, &sources).expect("scene");
        assert!(scene.clips().is_empty());
        assert_eq!(scene.total_frames(), 0);
    }

    #[test]
    fn resampling_preserves_duration() {
        let clip = SceneClip {
            source: source(2),
            source_start: 0,
            source_end: 4_800,
            timeline_start: 0,
            gain: 1.0,
            fade_in_frames: 0,
            fade_out_frames: 0,
            envelope: Vec::new(),
            reversed: false,
            inverted: false,
            pan: 0.0,
            track_id: None,
        };
        let scene = MixScene::new(48_000, 2, vec![clip]).expect("scene");
        let converted = scene.resampled(96_000).expect("resample");
        assert_eq!(converted.sample_rate(), 96_000);
        assert!(converted.total_frames().abs_diff(9_600) <= 2);
    }

    #[test]
    fn fractional_resampling_does_not_overrun_the_source() {
        let mut clip = SceneClip {
            source: source(1),
            source_start: 0,
            source_end: 4_800,
            timeline_start: 0,
            gain: 1.0,
            fade_in_frames: 100,
            fade_out_frames: 100,
            envelope: Vec::new(),
            reversed: false,
            inverted: false,
            pan: 0.0,
            track_id: None,
        };
        clip.envelope = vec![EnvelopePoint {
            frame: 4_800,
            gain: 1.0,
        }];
        let scene = MixScene::new(48_000, 1, vec![clip]).expect("scene");
        for target in [44_100, 96_000, 192_000] {
            let converted = scene.resampled(target).expect("resample");
            assert_eq!(converted.sample_rate(), target);
            assert_eq!(converted.clips().len(), 1);
            assert!(converted.clips()[0].source_end <= converted.clips()[0].source.frames());
        }
    }

    #[test]
    fn mixed_rate_sources_scale_to_project_frames() {
        let mut project = Project::new("mixed").expect("project");
        project.sources.push(
            Source::new(
                SourceId::from_u128(10),
                "44k.wav",
                "media/44k.wav",
                44_100,
                1,
                44_100,
            )
            .expect("source"),
        );
        let mut track = Track::new(TrackId::from_u128(11), "A");
        track.clips.push(Clip::new(
            ClipId::from_u128(12),
            SourceId::from_u128(10),
            FrameRange::new(0, 44_100).expect("range"),
            0,
        ));
        project.tracks.push(track);

        let audio =
            Arc::new(SourceAudio::from_samples(44_100, 1, vec![0.5; 44_100]).expect("audio"));
        let mut sources = HashMap::new();
        sources.insert(SourceId::from_u128(10), audio);
        let scene = MixScene::from_project(&project, &sources).expect("scene");
        assert!(scene.total_frames().abs_diff(48_000) <= 1);
        assert_eq!(scene.clips()[0].source.sample_rate(), 48_000);
        assert!(scene.clips()[0].len().abs_diff(48_000) <= 1);
        assert_eq!(project.duration_frames(), scene.total_frames());
    }

    #[test]
    fn oversize_ranges_are_rejected() {
        let clip = SceneClip {
            source: source(1),
            source_start: 0,
            source_end: 5_000,
            timeline_start: 0,
            gain: 1.0,
            fade_in_frames: 0,
            fade_out_frames: 0,
            envelope: Vec::new(),
            reversed: false,
            inverted: false,
            pan: 0.0,
            track_id: None,
        };
        assert!(MixScene::new(48_000, 2, vec![clip]).is_err());
    }
}
