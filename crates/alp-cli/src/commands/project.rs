//! Project commands: create, inspect, render and play.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use alp_core::{
    AddClip, AddSource, AddTrack, Clip, ClipId, CommandStack, DeleteRange, FrameRange, Project,
    ProjectPaths, Source, SourceId, Track, TrackId, secs_to_frames,
};
use alp_engine::{MixScene, SourceAudio, play_scene, render_scene};
use alp_media::{
    AudioMetadata, DitherMode, ExportFormat, ExportSettings, WavBitDepth, decode, resample,
    write_wav,
};
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

    let decoded = decode(input).with_context(|| format!("decode {}", input.display()))?;
    if decoded.frames() == 0 {
        bail!("{} contains no audio", input.display());
    }
    let peak = decoded.peak();
    let source_id = SourceId::new();
    let needs_conversion = decoded.sample_rate() != project.sample_rate;
    let (relative_path, source_rate, channels, frames) = if needs_conversion {
        let converted = resample(&decoded, project.sample_rate).with_context(|| {
            format!("resample {} to {} Hz", input.display(), project.sample_rate)
        })?;
        let file_name = format!("{source_id}-import.wav");
        let relative_path = format!("media/{file_name}");
        let destination = paths.root().join(&relative_path);
        write_wav(&destination, &converted, WavBitDepth::Float32)
            .with_context(|| format!("write converted import {}", destination.display()))?;
        println!(
            "converted {} from {} Hz to {} Hz",
            input.display(),
            decoded.sample_rate(),
            project.sample_rate
        );
        (
            relative_path,
            converted.sample_rate(),
            converted.channels(),
            converted.frames(),
        )
    } else {
        let file_name = input
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "audio.wav".to_owned());
        let relative_path = format!("media/{source_id}-{file_name}");
        let destination = paths.root().join(&relative_path);
        std::fs::copy(input, &destination)
            .with_context(|| format!("copy {} to {}", input.display(), destination.display()))?;
        (
            relative_path,
            decoded.sample_rate(),
            decoded.channels(),
            decoded.frames(),
        )
    };

    let name = display_name.unwrap_or_else(|| {
        input
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Imported audio".to_owned())
    });
    let source = Source::new(
        source_id,
        name,
        relative_path.clone(),
        source_rate,
        channels,
        frames,
    )?
    .with_peak(peak);

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
    println!(
        "file:        {}",
        paths.root().join(&relative_path).display()
    );
    Ok(())
}

/// Delete a range on the first track, then write it to autosave only.
///
/// The clean `project.json` is intentionally left untouched so the run models an
/// editor that has autosaved an edit but not yet saved the project.
pub fn delete_range(project_path: &Path, start: f64, end: f64, ripple: bool) -> Result<()> {
    let (paths, mut project) = ProjectPaths::open(project_path)
        .with_context(|| format!("open {}", project_path.display()))?;
    let track_id = project
        .tracks
        .first()
        .map(|track| track.id)
        .ok_or_else(|| anyhow::anyhow!("project has no tracks"))?;
    let range = FrameRange::new(
        secs_to_frames(start.max(0.0), project.sample_rate),
        secs_to_frames(end.max(0.0), project.sample_rate),
    )?;
    let mut stack = CommandStack::new();
    stack
        .execute(
            &mut project,
            Box::new(DeleteRange::new(track_id, range, ripple)),
        )
        .map_err(|error| anyhow::anyhow!("delete range: {error}"))?;
    project.touch();
    paths.autosave(&project)?;
    println!("deleted {start:.3}s..{end:.3}s (ripple={ripple})");
    println!("project:     {} clips in memory", project.clip_count());
    println!("autosave:    {}", paths.autosave_file().display());
    println!(
        "clean file:  {} (not replaced)",
        paths.project_file().display()
    );
    Ok(())
}

/// Write a recovery copy after an edit without replacing the clean project file.
pub fn autosave(project_path: &Path) -> Result<()> {
    let (paths, project) = ProjectPaths::open(project_path)
        .with_context(|| format!("open {}", project_path.display()))?;
    project.validate()?;
    paths.autosave(&project)?;
    println!("autosaved {}", paths.autosave_file().display());
    println!("project:     {} ({})", project.name, project.id);
    println!(
        "recover:     alp-cli recover-project {}",
        paths.root().display()
    );
    Ok(())
}

/// Restore an autosave over project.json after an interrupted session.
pub fn recover_project(project_path: &Path, json: bool) -> Result<()> {
    let root = if project_path.is_file() || project_path.ends_with("project.json") {
        project_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    } else {
        project_path.to_path_buf()
    };
    let paths = ProjectPaths::new(root);
    let recovered = paths
        .recover_autosave()
        .with_context(|| format!("recover autosave in {}", paths.root().display()))?;
    if !recovered {
        bail!("no autosave found at {}", paths.autosave_file().display());
    }
    let project = paths.load()?;
    if json {
        let value = serde_json::json!({
            "path": paths.root().display().to_string(),
            "id": project.id.to_string(),
            "name": project.name,
            "sample_rate": project.sample_rate,
            "clips": project.clip_count(),
            "recovered": true,
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("recovered {}", paths.project_file().display());
        println!("project:     {} ({})", project.name, project.id);
        println!("clips:       {}", project.clip_count());
    }
    Ok(())
}

/// Render a project to WAV or FLAC with optional resampling and tags.
pub fn render(
    project_path: &Path,
    output: &Path,
    format: &str,
    bit_depth: &str,
    sample_rate: Option<u32>,
    dither: &str,
    metadata: AudioMetadata,
) -> Result<()> {
    let format: ExportFormat = format.parse()?;
    let depth: WavBitDepth = bit_depth.parse()?;
    let dither: DitherMode = dither.parse()?;
    let (_, project, scene) = load_scene(project_path)?;
    let started = Instant::now();
    let buffer = render_scene(&scene, alp_engine::DEFAULT_BLOCK_FRAMES).context("render scene")?;
    let settings = ExportSettings {
        format,
        bit_depth: depth,
        sample_rate,
        dither,
        metadata,
    };
    let report = alp_media::export(output, &buffer, &settings).context("export audio")?;
    println!("rendered {}", output.display());
    println!("project:     {} ({})", project.name, project.id);
    println!("frames:      {}", report.frames);
    println!(
        "duration:    {:.3}s",
        report.frames as f64 / f64::from(report.sample_rate)
    );
    println!("peak:        {:.4}", report.peak);
    println!("rms:         {:.4}", report.rms);
    println!(
        "format:      {} · {} bit",
        report.format, report.bits_per_sample
    );
    println!("dither:      {dither}", dither = dither.label());
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

/// Render each track as its own stem file.
pub fn render_stems(
    project_path: &Path,
    output_dir: Option<&Path>,
    format: &str,
    bit_depth: &str,
    sample_rate: Option<u32>,
    dither: &str,
    metadata: alp_media::AudioMetadata,
) -> Result<()> {
    let format: alp_media::ExportFormat =
        format.parse().map_err(|error| anyhow::anyhow!("{error}"))?;
    let depth: alp_media::WavBitDepth = bit_depth
        .parse()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    let dither: alp_media::DitherMode =
        dither.parse().map_err(|error| anyhow::anyhow!("{error}"))?;
    let settings = alp_media::ExportSettings {
        format,
        bit_depth: depth,
        sample_rate,
        dither,
        metadata: metadata.clone(),
    };

    let (paths, project) = ProjectPaths::open(project_path)
        .with_context(|| format!("open {}", project_path.display()))?;
    paths.ensure_layout()?;
    let output_dir = match output_dir {
        Some(path) => path.to_path_buf(),
        None => paths.renders_dir().join("stems"),
    };
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("create {}", output_dir.display()))?;

    let mut sources = HashMap::new();
    for source in &project.sources {
        let path = paths.resolve_relative(&source.relative_path)?;
        let buffer = decode(&path)
            .with_context(|| format!("decode source {} ({})", source.name, path.display()))?;
        sources.insert(source.id, Arc::new(SourceAudio::from_pcm(&buffer)));
    }

    let mut rendered = 0usize;
    let total = project.tracks.len();
    for (index, track) in project.tracks.iter().enumerate() {
        if track.clips.is_empty() {
            continue;
        }
        let scene = alp_engine::MixScene::track_scene(&project, track.id, &sources)
            .with_context(|| format!("build scene for {}", track.name))?;
        if scene.total_frames() == 0 {
            continue;
        }
        let buffer = render_scene(&scene, alp_engine::DEFAULT_BLOCK_FRAMES)
            .with_context(|| format!("render {}", track.name))?;
        let slug: String = track
            .name
            .chars()
            .map(|character| {
                if character.is_alphanumeric() {
                    character
                } else {
                    '-'
                }
            })
            .collect();
        let slug = slug.trim_matches('-');
        let slug = if slug.is_empty() {
            format!("track-{}", index + 1)
        } else {
            slug.to_owned()
        };
        let file_name = format!("{slug}.{}", format.label());
        let output = output_dir.join(file_name);
        let mut track_metadata = metadata.clone();
        if track_metadata.title.is_none() {
            track_metadata.title = Some(track.name.clone());
        }
        let report = alp_media::export(&output, &buffer, &settings).context("export audio")?;
        println!(
            "stem {} · {} · {} frames · {} bit",
            track.name,
            output.display(),
            report.frames,
            report.bits_per_sample
        );
        rendered += 1;
    }

    if rendered == 0 {
        bail!("no non-empty tracks to render");
    }
    println!(
        "rendered {} of {} track stems into {}",
        rendered,
        total,
        output_dir.display()
    );
    Ok(())
}
