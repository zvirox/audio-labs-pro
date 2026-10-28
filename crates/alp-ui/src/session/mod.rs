//! Editing and autosave session for Audio Labs Pro.
//!
//! The session owns the live project, undo stack, selection and clipboard. Every
//! timeline mutation is a command from `alp-core`; the views only read state and
//! send messages. Recorded or imported audio is copied into the project and kept
//! as an immutable source snapshot so playback, rendering and editing never
//! decode from the UI thread. Edits are autosaved through `ProjectPaths`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use alp_core::{
    AddClip, AddSource, Clip, ClipFragment, ClipId, Clipboard, Command, CommandStack, FrameRange,
    Project, ProjectPaths, PunchIn, Selection, Source, SourceId, Track, TrackId, secs_to_frames,
};
use alp_engine::SourceAudio;
use alp_media::{PcmBuffer, WavBitDepth, decode, write_wav};

mod export;
mod import;
mod operations;
#[cfg(test)]
mod tests;

pub use self::import::ImportedAudio;

/// Default target for the normalize command.
pub const NORMALIZE_TARGET: f32 = 0.98;

/// The demo source and timeline length in seconds.
const DEMO_SECONDS: f64 = 20.0;

/// A multitrack editing session with undoable commands.
pub struct EditSession {
    project: Project,
    stack: CommandStack,
    selection: Selection,
    clipboard: Clipboard,
    selected_clip: Option<ClipId>,
    track_id: TrackId,
    audio: HashMap<SourceId, Arc<SourceAudio>>,
    status: String,
    last_export: Option<String>,
    import_rx: Option<Receiver<Result<ImportedAudio, String>>>,
    paths: Option<ProjectPaths>,
    dirty: bool,
    last_autosave: Instant,
}

impl EditSession {
    /// Build a valid demo project so the shell can exercise real commands
    /// without reading a user file or opening an audio device.
    #[must_use]
    pub fn demo() -> Self {
        Self::demo_inner(None)
    }

    /// Open the desktop project at `root`, recovering an autosave first.
    ///
    /// A missing or unreadable project falls back to the synthetic demo. New
    /// projects are written with their demo media so later launches can reload
    /// them without regenerating audio.
    #[must_use]
    pub fn open_or_demo(root: PathBuf) -> Self {
        let paths = ProjectPaths::new(root);
        let mut recovery_note = None;
        if paths.has_autosave() {
            match paths.recover_autosave() {
                Ok(true) => recovery_note = Some("Recovered autosave".to_owned()),
                Ok(false) => {}
                Err(error) => recovery_note = Some(format!("Autosave rejected: {error}")),
            }
        }
        if paths.project_file().is_file() {
            match paths.load() {
                Ok(project) => match Self::from_project(paths.clone(), project, recovery_note) {
                    Ok(session) => return session,
                    Err(error) => {
                        let mut session = Self::demo_inner(Some(paths));
                        session.status = format!("Project could not be opened: {error}");
                        return session;
                    }
                },
                Err(error) => {
                    let mut session = Self::demo_inner(Some(paths));
                    session.status = format!("Project could not be read: {error}");
                    return session;
                }
            }
        }
        let mut session = Self::demo_inner(Some(paths.clone()));
        if let Err(error) = session.persist_new_project() {
            session.status = format!("Project could not be created: {error}");
        }
        session
    }

    fn demo_inner(paths: Option<ProjectPaths>) -> Self {
        let mut project = Project::new("Phase 4 demo").expect("valid default project");
        let source_id = SourceId::new();
        let track_id = TrackId::new();
        let frame_count = secs_to_frames(DEMO_SECONDS, project.sample_rate);
        let buffer = demo_buffer(project.sample_rate, frame_count);
        project.sources.push(
            Source::new(
                source_id,
                "demo-take.wav",
                "media/demo-take.wav",
                project.sample_rate,
                1,
                frame_count,
            )
            .expect("demo source")
            .with_peak(buffer.peak()),
        );
        let mut track = Track::new(track_id, "Take 1");
        let first = Clip::new(
            ClipId::new(),
            source_id,
            FrameRange::new(0, secs_to_frames(12.0, project.sample_rate)).expect("range"),
            0,
        );
        let second = Clip::new(
            ClipId::new(),
            source_id,
            FrameRange::new(
                secs_to_frames(13.0, project.sample_rate),
                secs_to_frames(19.0, project.sample_rate),
            )
            .expect("range"),
            secs_to_frames(14.0, project.sample_rate),
        );
        track.clips.push(first.clone());
        track.clips.push(second);
        project.tracks.push(track);
        project.validate().expect("demo project is valid");

        let mut audio = HashMap::new();
        audio.insert(source_id, Arc::new(SourceAudio::from_pcm(&buffer)));
        let rate = project.sample_rate;
        Self {
            project,
            stack: CommandStack::new(),
            selection: Selection::between(
                track_id,
                secs_to_frames(2.0, rate),
                secs_to_frames(4.0, rate),
                1,
            ),
            clipboard: Clipboard::default(),
            selected_clip: Some(first.id),
            track_id,
            audio,
            status: "Demo project ready · all edits are undoable".into(),
            last_export: None,
            import_rx: None,
            paths,
            dirty: false,
            last_autosave: Instant::now(),
        }
    }

    fn from_project(
        paths: ProjectPaths,
        project: Project,
        recovery_note: Option<String>,
    ) -> Result<Self, String> {
        let mut audio = HashMap::new();
        for source in &project.sources {
            let path = paths
                .resolve_relative(&source.relative_path)
                .map_err(|error| format!("{}: {error}", source.relative_path))?;
            let buffer =
                decode(&path).map_err(|error| format!("decode {}: {error}", path.display()))?;
            audio.insert(source.id, Arc::new(SourceAudio::from_pcm(&buffer)));
        }
        let track_id = project
            .tracks
            .first()
            .map(|track| track.id)
            .ok_or_else(|| "project has no tracks".to_owned())?;
        let selected_clip = project
            .track(track_id)
            .and_then(|track| track.clips.first())
            .map(|clip| clip.id);
        let selection = selected_clip.map_or_else(
            || Selection::at(track_id, 0),
            |id| {
                let (_, clip) = project.clip(id).expect("clip just read");
                Selection::between(
                    track_id,
                    clip.timeline_start,
                    project.clip_timeline_end(clip),
                    1,
                )
            },
        );
        let status = recovery_note.unwrap_or_else(|| "Project loaded".to_owned());
        Ok(Self {
            project,
            stack: CommandStack::new(),
            selection,
            clipboard: Clipboard::default(),
            selected_clip,
            track_id,
            audio,
            status,
            last_export: None,
            import_rx: None,
            paths: Some(paths),
            dirty: false,
            last_autosave: Instant::now(),
        })
    }

    fn persist_new_project(&mut self) -> Result<(), String> {
        let Some(paths) = &self.paths else {
            return Ok(());
        };
        paths.ensure_layout().map_err(|error| error.to_string())?;
        let source = self
            .project
            .sources
            .first()
            .ok_or_else(|| "demo project has no source".to_owned())?;
        let audio = self
            .audio
            .get(&source.id)
            .ok_or_else(|| "demo source audio is missing".to_owned())?;
        let buffer = PcmBuffer::new(
            audio.sample_rate(),
            audio.channels(),
            audio.samples().to_vec(),
        )
        .map_err(|error| error.to_string())?;
        let file = paths.media_dir().join("demo-take.wav");
        write_wav(&file, &buffer, WavBitDepth::Float32).map_err(|error| error.to_string())?;
        paths
            .create(&self.project)
            .map_err(|error| error.to_string())
    }

    /// Whether a debounced autosave is waiting for a frame tick.
    #[must_use]
    pub fn needs_autosave(&self) -> bool {
        self.dirty && self.paths.is_some()
    }

    /// Write a pending autosave after the debounce interval.
    pub fn poll_autosave(&mut self) {
        if self.needs_autosave() && self.last_autosave.elapsed() >= Duration::from_millis(750) {
            self.flush_autosave();
        }
    }

    /// Write the current project to the recovery copy immediately.
    pub fn flush_autosave(&mut self) {
        if !self.dirty {
            return;
        }
        let Some(paths) = &self.paths else {
            return;
        };
        match paths.autosave(&self.project) {
            Ok(()) => {
                self.dirty = false;
                self.last_autosave = Instant::now();
            }
            Err(error) => {
                self.last_autosave = Instant::now();
                self.status = format!("Autosave failed: {error}");
            }
        }
    }

    /// Store a decoded buffer in the project and return its relative path.
    fn store_audio(
        &self,
        buffer: &PcmBuffer,
        name: &str,
        source_id: SourceId,
    ) -> Result<Option<String>, String> {
        let Some(paths) = &self.paths else {
            return Ok(None);
        };
        paths.ensure_layout().map_err(|error| error.to_string())?;
        let slug = safe_file_stem(name);
        let file = paths.media_dir().join(format!("{source_id}-{slug}.wav"));
        write_wav(&file, buffer, WavBitDepth::Float32).map_err(|error| error.to_string())?;
        Ok(Some(format!(
            "media/{}",
            file.file_name()
                .map_or_else(|| "audio.wav".into(), |name| name.to_string_lossy())
        )))
    }

    /// The live project.
    #[must_use]
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Timeline selection.
    #[must_use]
    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// Currently selected clip.
    #[must_use]
    pub fn selected_clip(&self) -> Option<ClipId> {
        self.selected_clip
    }

    /// Clip lookup for the inspector.
    #[must_use]
    pub fn clip(&self, id: ClipId) -> Option<&Clip> {
        self.project.clip(id).map(|(_, clip)| clip)
    }

    /// Track that owns the session.
    #[must_use]
    pub fn track_id(&self) -> TrackId {
        self.track_id
    }

    /// Status line shown in the inspector.
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// Full path of the last successful export.
    #[must_use]
    pub fn last_export(&self) -> Option<&str> {
        self.last_export.as_deref()
    }

    /// Whether undo is available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.stack.can_undo()
    }

    /// Whether redo is available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.stack.can_redo()
    }

    /// Timeline duration in seconds.
    #[must_use]
    pub fn duration_secs(&self) -> f64 {
        alp_core::frames_to_secs(self.project.duration_frames(), self.project.sample_rate)
    }

    /// Number of clips in the single editing track.
    #[must_use]
    pub fn clip_count(&self) -> usize {
        self.project
            .track(self.track_id)
            .map_or(0, |track| track.clips.len())
    }

    /// Decode a file synchronously and add it as a new clip.
    pub fn import_file(&mut self, path: &Path) {
        match decode(path) {
            Ok(buffer) => {
                let name = path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Imported audio".into());
                self.import_buffer(buffer, &name, &path.to_string_lossy());
            }
            Err(error) => self.status = format!("Import failed: {error}"),
        }
    }

    /// Decode a take and punch it into the selected range.
    pub fn punch_in_file(&mut self, path: &Path) -> bool {
        match decode(path) {
            Ok(buffer) => {
                let name = path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Punch take".into());
                self.punch_in_buffer(buffer, &name, &path.to_string_lossy())
            }
            Err(error) => {
                self.status = format!("Punch-in decode failed: {error}");
                false
            }
        }
    }

    /// Replace the selected range with a recorded buffer, trimmed or padded to
    /// the selection length.
    pub fn punch_in_buffer(&mut self, buffer: PcmBuffer, name: &str, fallback_path: &str) -> bool {
        let range = self.selection.range();
        if range.is_empty() {
            self.status = "Select a range before punch-in".into();
            return false;
        }
        let sample_rate = buffer.sample_rate();
        let channels = buffer.channels();
        let mut samples = buffer.into_samples();
        let target =
            usize::try_from(range.len().saturating_mul(u64::from(channels))).unwrap_or(usize::MAX);
        samples.resize(target, 0.0);
        samples.truncate(target);
        let Some(buffer) = PcmBuffer::new(sample_rate, channels, samples).ok() else {
            self.status = "Punch-in buffer is invalid".into();
            return false;
        };
        let peak = buffer.peak();
        let source_id = SourceId::new();
        let relative_path = match self.store_audio(&buffer, name, source_id) {
            Ok(Some(path)) => path,
            Ok(None) => fallback_path.to_owned(),
            Err(error) => {
                self.status = format!("Punch-in failed: {error}");
                return false;
            }
        };
        let source = match Source::new(
            source_id,
            name,
            relative_path,
            buffer.sample_rate(),
            buffer.channels(),
            buffer.frames(),
        ) {
            Ok(source) => source.with_peak(peak),
            Err(error) => {
                self.status = format!("Punch-in failed: {error}");
                return false;
            }
        };
        let source_index = self.project.sources.len();
        if let Err(error) = self.stack.execute(
            &mut self.project,
            Box::new(AddSource::new(source, source_index)),
        ) {
            self.status = format!("Punch-in failed: {error}");
            return false;
        }
        self.audio
            .insert(source_id, Arc::new(SourceAudio::from_pcm(&buffer)));
        let fragment = ClipFragment {
            source_id,
            source_range: match FrameRange::new(0, buffer.frames()) {
                Ok(range) => range,
                Err(error) => {
                    self.status = format!("Punch-in failed: {error}");
                    return false;
                }
            },
            offset: 0,
            gain: 1.0,
            fade_in_frames: 0,
            fade_out_frames: 0,
            envelope: Vec::new(),
            reversed: false,
            inverted: false,
        };
        let inserted = self.run(
            Box::new(PunchIn::new(self.track_id, range, &[fragment])),
            "Punch in",
        );
        if inserted {
            self.selected_clip = None;
        } else {
            let _ = self.stack.undo(&mut self.project);
        }
        inserted
    }

    /// Add decoded audio at the end of the timeline across armed tracks.
    pub fn import_buffer(&mut self, buffer: PcmBuffer, name: &str, fallback_path: &str) {
        let source_id = SourceId::new();
        let relative_path = match self.store_audio(&buffer, name, source_id) {
            Ok(Some(path)) => path,
            Ok(None) => fallback_path.to_owned(),
            Err(error) => {
                self.status = format!("Import failed: {error}");
                return;
            }
        };
        let source = match Source::new(
            source_id,
            name,
            relative_path,
            buffer.sample_rate(),
            buffer.channels(),
            buffer.frames(),
        ) {
            Ok(source) => source.with_peak(buffer.peak()),
            Err(error) => {
                self.status = format!("Import failed: {error}");
                return;
            }
        };

        let target_tracks: Vec<TrackId> = self
            .project
            .tracks
            .iter()
            .filter(|track| track.armed)
            .map(|track| track.id)
            .collect();
        let target_tracks = if target_tracks.is_empty() {
            vec![self.track_id]
        } else {
            target_tracks
        };

        let history_before = self.stack.undo_len();
        let source_index = self.project.sources.len();
        if let Err(error) = self.stack.execute(
            &mut self.project,
            Box::new(AddSource::new(source, source_index)),
        ) {
            self.status = format!("Import failed: {error}");
            return;
        }

        let start = self.project.duration_frames();
        let range = match FrameRange::new(0, buffer.frames()) {
            Ok(range) => range,
            Err(error) => {
                while self.stack.undo_len() > history_before {
                    let _ = self.stack.undo(&mut self.project);
                }
                self.status = format!("Import failed: {error}");
                return;
            }
        };

        let target_clip_indexes = target_tracks
            .iter()
            .map(|track_id| {
                self.project
                    .track(*track_id)
                    .map_or(0, |track| track.clips.len())
            })
            .collect::<Vec<_>>();

        let mut first_clip = None;
        let mut first_track = self.track_id;
        let mut import_failed: Option<String> = None;
        for (index, track_id) in target_tracks.iter().enumerate() {
            let clip_id = ClipId::new();
            let clip = Clip::new(clip_id, source_id, range, start);
            let clip_index = target_clip_indexes[index];
            match self.stack.execute(
                &mut self.project,
                Box::new(AddClip::new(*track_id, clip.clone(), clip_index)),
            ) {
                Ok(()) => {
                    if first_clip.is_none() {
                        first_clip = Some(clip);
                        first_track = *track_id;
                    }
                }
                Err(error) => {
                    import_failed = Some(format!("Import failed: {error}"));
                    break;
                }
            }
        }

        if let Some(error) = import_failed {
            while self.stack.undo_len() > history_before {
                let _ = self.stack.undo(&mut self.project);
            }
            self.status = error;
            return;
        }

        self.audio
            .insert(source_id, Arc::new(SourceAudio::from_pcm(&buffer)));
        if let Some(clip) = first_clip {
            self.selected_clip = Some(clip.id);
            if let Some((_, clip)) = self.project.clip(clip.id) {
                let end = self.project.clip_timeline_end(clip);
                self.selection = Selection::between(first_track, clip.timeline_start, end, 1);
            }
        }
        self.status = format!(
            "Imported {} · {:.2}s{}",
            name,
            alp_core::frames_to_secs(buffer.frames(), buffer.sample_rate()),
            if target_tracks.len() > 1 {
                format!(" · {} tracks", target_tracks.len())
            } else {
                String::new()
            }
        );
        self.dirty = true;
    }

    fn run(&mut self, command: Box<dyn Command>, label: &str) -> bool {
        match self.stack.execute(&mut self.project, command) {
            Ok(()) => {
                self.after_change(label);
                true
            }
            Err(error) => {
                self.status = format!("{label} failed: {error}");
                false
            }
        }
    }

    fn after_change(&mut self, label: &str) {
        if let Err(error) = self.project.validate() {
            let _ = self.stack.undo(&mut self.project);
            self.status = format!("{label} rejected: {error}");
            return;
        }
        if let Some(id) = self.selected_clip
            && self.project.clip(id).is_none()
        {
            self.selected_clip = None;
        }
        self.status = format!("{label} · {} clip(s)", self.clip_count());
        self.dirty = true;
    }
}

fn demo_buffer(sample_rate: u32, frame_count: u64) -> PcmBuffer {
    let samples: Vec<f32> = (0..frame_count)
        .map(|frame| {
            let time = alp_core::frames_to_secs(frame, sample_rate);
            let tone = (std::f32::consts::TAU * 220.0 * time as f32).sin() * 0.28;
            let overtone = (std::f32::consts::TAU * 440.0 * time as f32).sin() * 0.08;
            let envelope = (time as f32 * std::f32::consts::PI / DEMO_SECONDS as f32)
                .sin()
                .abs();
            (tone + overtone) * envelope
        })
        .collect();
    PcmBuffer::new(sample_rate, 1, samples).expect("demo buffer")
}

fn safe_file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect();
    let stem = stem.trim_matches('-').to_lowercase();
    if stem.is_empty() {
        "audio".into()
    } else {
        stem.chars().take(48).collect()
    }
}

fn short_id(id: ClipId) -> String {
    id.to_string().chars().take(8).collect()
}
