//! Offline mixdown used by export, tests and the CLI.
//!
//! Rendering is not real-time: it allocates the output buffer up front and runs
//! the same [`Mixer`] as the device callback, so block size can never change the
//! result.

use alp_media::{PcmBuffer, WavBitDepth, write_wav};

use crate::error::EngineError;
use crate::mixer::Mixer;
use crate::scene::MixScene;

/// Default offline block size in frames.
pub const DEFAULT_BLOCK_FRAMES: usize = 1_024;

/// Measurements from one render.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderReport {
    /// Frames rendered.
    pub frames: u64,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Largest absolute sample value.
    pub peak: f32,
    /// Root-mean-square level.
    pub rms: f32,
}

/// Render a scene to an in-memory PCM buffer.
pub fn render_scene(scene: &MixScene, block_frames: usize) -> Result<PcmBuffer, EngineError> {
    let block_frames = block_frames.max(1);
    let channels = scene.channels();
    let mut output = PcmBuffer::silence(scene.sample_rate(), channels, scene.total_frames())?;
    let mut mixer = Mixer::new(scene.clone());
    let block_samples = block_frames * usize::from(channels);
    for block in output.samples_mut().chunks_mut(block_samples) {
        mixer.render_into(block);
    }
    Ok(output)
}

/// Render a scene to a WAV file atomically.
pub fn render_scene_to_wav(
    scene: &MixScene,
    path: impl AsRef<std::path::Path>,
    bit_depth: WavBitDepth,
) -> Result<RenderReport, EngineError> {
    let buffer = render_scene(scene, DEFAULT_BLOCK_FRAMES)?;
    let report = RenderReport {
        frames: buffer.frames(),
        duration_secs: buffer.duration_secs(),
        peak: buffer.peak(),
        rms: buffer.rms(),
    };
    write_wav(path, &buffer, bit_depth)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{SceneClip, SourceAudio};
    use std::sync::Arc;

    fn scene() -> MixScene {
        let samples = (0..4_800)
            .map(|index| (index as f32 / 20.0).sin() * 0.4)
            .collect();
        let source = Arc::new(SourceAudio::from_samples(48_000, 1, samples).expect("source"));
        let clip = SceneClip {
            source,
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
        MixScene::new(48_000, 2, vec![clip]).expect("scene")
    }

    #[test]
    fn block_size_does_not_change_output() {
        let scene = scene();
        let a = render_scene(&scene, 64).expect("render");
        let b = render_scene(&scene, 1_024).expect("render");
        assert_eq!(a, b);
    }

    #[test]
    fn renders_ten_tracks() {
        use alp_core::{Clip, FrameRange, Project, Source, SourceId, Track, TrackId};
        use std::collections::HashMap;
        let mut project = Project::new("ten-track").expect("project");
        let source_id = SourceId::from_u128(1);
        project.sources.push(
            Source::new(source_id, "tone", "media/tone.wav", 48_000, 1, 4_800).expect("source"),
        );
        for index in 0..10 {
            let mut track = Track::new(TrackId::from_u128(index + 10), "Track");
            track.gain = 0.1;
            track.clips.push(Clip::new(
                alp_core::ClipId::from_u128(index + 100),
                source_id,
                FrameRange::new(0, 4_800).expect("range"),
                0,
            ));
            project.tracks.push(track);
        }
        let mut sources = HashMap::new();
        sources.insert(
            source_id,
            Arc::new(SourceAudio::from_samples(48_000, 1, vec![0.5; 4_800]).expect("audio")),
        );
        let scene = MixScene::from_project(&project, &sources).expect("scene");
        let buffer = render_scene(&scene, 1_024).expect("render");
        assert_eq!(buffer.frames(), 4_800);
        assert!(buffer.peak() > 0.0);
    }

    #[test]
    fn render_reports_audio() {
        let scene = scene();
        let buffer = render_scene(&scene, DEFAULT_BLOCK_FRAMES).expect("render");
        assert_eq!(buffer.frames(), 4_800);
        assert!(buffer.peak() > 0.3);
        assert!(buffer.rms() > 0.1);
    }

    #[test]
    fn writes_a_wav_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("render.wav");
        let report = render_scene_to_wav(&scene(), &path, WavBitDepth::Int16).expect("render");
        assert!(path.is_file());
        assert_eq!(report.frames, 4_800);
    }
}
