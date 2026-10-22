//! Project commands: create, inspect, render and play.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use alp_core::{
    AddClip, AddSource, AddTrack, Clip, ClipId, CommandStack, FrameRange, Project, ProjectPaths,
    Source, SourceId, Track, TrackId, secs_to_frames,
};
use alp_engine::{MixScene, SourceAudio, play_scene, render_scene_to_wav};
use alp_media::{WavBitDepth, decode, probe};
use anyhow::{Context, Result, bail};

/// Create a project directory with one empty track.
pub fn create(directory: &Path, name: Option<String>, sample_rate: u32) -> Result<()> {
    let paths = ProjectPaths::new(directory);
    if paths.project_file().exists() {
        bail!("{} already has a project.json", directory.display());
    }
    let name = name.unwrap_or_else(|| {
        directory
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Untitled")
            .to_owned()
    });
    let mut project = Project::new_with_rate(name, sample_rate)?;
    let mut stack = CommandStack::new();
    stack
        .execute(
            &mut project,
            Box::new(AddTrack::new(Track::new(TrackId::new(), "Track 1"), 0)),
        )
        .map_err(|error| anyhow::anyhow!("create track: {error}"))?;
    paths.create(&project)?;

    println!("created {}", paths.root().display());
    println!("project:     {} ({})", project.name, project.id);
    println!("sample rate: {} Hz", project.sample_rate);
    println!("file:        {}", paths.project_file().display());
    Ok(())
}

/// Print a project summary.
pub fn info(project_path: &Path, json: bool) -> Result<()> {
    let (paths, project) = ProjectPaths::open(project_path)
        .with_context(|| format!("open {}", project_path.display()))?;
    let duration = project.duration_frames() as f64 / f64::from(project.sample_rate);

    if json {
        let sources: Vec<_> = project
            .sources
            .iter()
            .map(|source| {
                serde_json::json!({
                    "id": source.id.to_string(),
                    "name": source.name,
                    "path": source.relative_path,
                    "sample_rate": source.sample_rate,
                    "channels": source.channels,
                    "frames": source.frames,
                })
            })
            .collect();
        let tracks: Vec<_> = project
            .tracks
            .iter()
            .map(|track| {
                serde_json::json!({
                    "id": track.id.to_string(),
                    "name": track.name,
                    "gain": track.gain,
                    "pan": track.pan,
                    "muted": track.muted,
                    "solo": track.solo,
                    "clips": track.clips.len(),
                })
            })
            .collect();
        let value = serde_json::json!({
            "path": paths.root().display().to_string(),
            "id": project.id.to_string(),
            "name": project.name,
            "sample_rate": project.sample_rate,
            "duration_secs": duration,
            "duration_frames": project.duration_frames(),
            "sources": sources,
            "tracks": tracks,
            "clips": project.clip_count(),
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!("project:     {} ({})", project.name, project.id);
    println!("path:        {}", paths.root().display());
    println!("sample rate: {} Hz", project.sample_rate);
    println!(
        "duration:    {duration:.3}s ({} frames)",
        project.duration_frames()
    );
    println!(
        "sources:     {} · tracks: {} · clips: {}",
        project.sources.len(),
        project.tracks.len(),
        project.clip_count()
    );
    for source in &project.sources {
        println!(
            "  source {} · {} · {} Hz · {} ch · {} frames",
            source.name, source.relative_path, source.sample_rate, source.channels, source.frames
        );
    }
    for track in &project.tracks {
        println!(
            "  track  {} · gain {:.2} · pan {:+.2} · {} clips{}",
            track.name,
            track.gain,
            track.pan,
            track.clips.len(),
            if track.muted { " · muted" } else { "" }
        );
    }
    Ok(())
}

/// Copy a media file into a project and add it as a clip.
pub fn import(
    project_path: &Path,
    input: &Path,
    track_name: Option<String>,
    at_seconds: f64,
    display_name: Option<String>,
) -> Result<()> {
    let (paths, mut project) = ProjectPaths::open(project_path)
        .with_context(|| format!("open {}", project_path.display()))?;
    paths.ensure_layout()?;

    let info = probe(input).with_context(|| format!("probe {}", input.display()))?;
    let frames = match info.frames {
        Some(frames) => frames,
        None => decode(input)
            .with_context(|| format!("decode {}", input.display()))?
            .frames(),
    };
    if frames == 0 {
        bail!("{} contains no audio", input.display());
    }

    let source_id = SourceId::new();
    let file_name = input
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "audio.wav".to_owned());
    let relative_path = format!("media/{source_id}-{file_name}");
    let destination = paths.root().join(&relative_path);
    std::fs::copy(input, &destination)
        .with_context(|| format!("copy {} to {}", input.display(), destination.display()))?;

    let name = display_name.unwrap_or_else(|| {
        input
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Imported audio".to_owned())
    });
    let source = Source::new(
        source_id,
        name,
        relative_path,
        info.sample_rate,
        info.channels,
        frames,
    )?;

    let mut stack = CommandStack::new();
    let source_index = project.sources.len();
    stack
        .execute(&mut project, Box::new(AddSource::new(source, source_index)))
        .map_err(|error| anyhow::anyhow!("add source: {error}"))?;

    let track_id = if let Some(requested) = track_name {
        project
            .tracks
            .iter()
            .find(|track| track.name == requested)
            .map(|track| track.id)
            .ok_or_else(|| anyhow::anyhow!("track {requested:?} not found"))?
    } else if let Some(track) = project.tracks.first() {
        track.id
    } else {
        let id = TrackId::new();
        stack
            .execute(
                &mut project,
                Box::new(AddTrack::new(Track::new(id, "Track 1"), 0)),
            )
            .map_err(|error| anyhow::anyhow!("add track: {error}"))?;
        id
    };

    let timeline_start = secs_to_frames(at_seconds, project.sample_rate);
    let clip_id = ClipId::new();
    let clip = Clip::new(
        clip_id,
        source_id,
        FrameRange::new(0, frames)?,
        timeline_start,
    );
    let clip_index = project.track(track_id).map_or(0, |track| track.clips.len());
    stack
        .execute(
            &mut project,
            Box::new(AddClip::new(track_id, clip, clip_index)),
        )
        .map_err(|error| anyhow::anyhow!("add clip: {error}"))?;

    project.touch();
    paths.save(&project)?;
    println!("imported {}", input.display());
    println!("source:      {} ({})", source_id, frames);
    println!("clip:        {} on track {}", clip_id, track_id);
    println!("at:          {timeline_start} frames ({at_seconds:.3}s)");
    println!("file:        {}", destination.display());
    Ok(())
}

/// Render a project to a WAV file.
pub fn render(project_path: &Path, output: &Path, bit_depth: &str) -> Result<()> {
    let depth: WavBitDepth = bit_depth.parse()?;
    let (_, project, scene) = load_scene(project_path)?;
    let started = Instant::now();
    let report = render_scene_to_wav(&scene, output, depth).context("render scene to WAV")?;
    println!("rendered {}", output.display());
    println!("project:     {} ({})", project.name, project.id);
    println!("frames:      {}", report.frames);
    println!("duration:    {:.3}s", report.duration_secs);
    println!("peak:        {:.4}", report.peak);
    println!("rms:         {:.4}", report.rms);
    println!("format:      {}", depth.label());
    println!("elapsed:     {:.3}s", started.elapsed().as_secs_f64());
    Ok(())
}

/// Play a project on the default output device.
pub fn play(project_path: &Path, seconds: Option<f64>) -> Result<()> {
    let (_, project, scene) = load_scene(project_path)?;
    println!(
        "playing {} · {:.3}s · {} Hz · {} clips",
        project.name,
        scene.duration_secs(),
        scene.sample_rate(),
        scene.clips().len()
    );
    let report = play_scene(&scene, seconds).context("play scene")?;
    println!(
        "callbacks={} frames={} elapsed={:.3}s finished={} stream_error={}",
        report.callbacks, report.frames, report.elapsed_secs, report.finished, report.stream_error
    );
    if report.stream_error {
        bail!("the audio backend reported a stream error");
    }
    Ok(())
}

fn load_scene(project_path: &Path) -> Result<(ProjectPaths, Project, MixScene)> {
    let (paths, project) = ProjectPaths::open(project_path)
        .with_context(|| format!("open {}", project_path.display()))?;
    let mut sources = HashMap::new();
    for source in &project.sources {
        let path = paths.resolve_relative(&source.relative_path)?;
        let buffer = decode(&path)
            .with_context(|| format!("decode source {} ({})", source.name, path.display()))?;
        if buffer.sample_rate() != source.sample_rate || buffer.channels() != source.channels {
            tracing::warn!(
                source = %source.id,
                stored_rate = source.sample_rate,
                stored_channels = source.channels,
                actual_rate = buffer.sample_rate(),
                actual_channels = buffer.channels(),
                "source metadata differs from the file; using the decoded format"
            );
        }
        sources.insert(source.id, Arc::new(SourceAudio::from_pcm(&buffer)));
    }
    let scene = MixScene::from_project(&project, &sources).context("build mix scene")?;
    Ok((paths, project, scene))
}

/// Resolve the default render path for a project.
#[must_use]
pub fn default_render_path(project: &ProjectPaths, name: &str) -> PathBuf {
    project.renders_dir().join(name)
}
