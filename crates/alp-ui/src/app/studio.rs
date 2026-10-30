//! Voice studio operations: profiles, speech rendering, stories and captures.
//!
//! Every expensive render runs on a [`JobQueue`] worker and hands its result
//! back through an `Arc<Mutex<..>>` slot drained from [`Message::Frame`]. The UI
//! thread only writes project audio through `EditSession`, so undo keeps
//! working for takes inserted into the timeline.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use alp_engine::{MixScene, SceneClip, SourceAudio};
use alp_media::{ExportSettings, PcmBuffer, WavBitDepth, decode, export};
use alp_voice::{
    BuiltinSttBackend, ConsentRecord, InferClient, SttBackend, TtsRequest, VoiceDesignRequest,
    VoiceKind, VoiceProfile, design_voice,
};

use super::{App, CaptureItem, GeneratedTake, GenerationKind, HistoryItem, RenderOutput};

/// Shorten a take for the docked player title.
fn preview_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() > 52 {
        let short: String = trimmed.chars().take(52).collect();
        format!("{short}…")
    } else {
        trimmed.to_owned()
    }
}

/// Human consent statement recorded for cloned voices.
const CONSENT_STATEMENT: &str =
    "I confirm that I have explicit permission from this speaker to clone their voice locally.";

impl App {
    /// The currently selected voice profile.
    pub(super) fn profile(&self) -> Option<&VoiceProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.id == self.selected_profile)
    }

    /// Display name for a profile id.
    pub(super) fn profile_name(&self, id: &str) -> String {
        self.profiles
            .iter()
            .find(|profile| profile.id == id)
            .map_or_else(|| id.to_owned(), |profile| profile.name.clone())
    }

    /// Load a profile's metadata into the editor fields.
    pub(super) fn select_profile(&mut self, id: &str) {
        if let Some(profile) = self.profiles.iter().find(|profile| profile.id == id) {
            self.selected_profile = profile.id.clone();
            self.profile_name = profile.name.clone();
            self.profile_tags = profile.tags.join(", ");
            self.voice_status = format!(
                "{} · {} · {:.2}x",
                profile.name,
                voice_kind_label(profile.kind),
                profile.default_params.speed
            );
        }
    }

    /// Re-read profiles from the store, keeping the current selection when possible.
    pub(super) fn reload_profiles(&mut self) {
        let Some(store) = &self.profile_store else {
            return;
        };
        let profiles = store.list_profiles();
        if !profiles.is_empty() {
            self.profiles = profiles;
        }
        if !self.profiles.iter().any(|p| p.id == self.selected_profile)
            && let Some(first) = self.profiles.first()
        {
            let id = first.id.clone();
            self.select_profile(&id);
        }
    }

    /// Render the current prompt with the selected profile on a worker thread.
    pub(super) fn start_generation(&mut self) {
        if self.generating {
            self.show_toast("A render is already running");
            return;
        }
        let text = self.voice_text.trim().to_owned();
        if text.is_empty() {
            self.show_toast("Type something to speak first");
            return;
        }
        let Some(profile) = self.profile().cloned() else {
            self.show_toast("Select a voice profile first");
            return;
        };
        if profile.kind == VoiceKind::Cloned && profile.consent.is_none() {
            self.show_toast("Cloned profiles need a consent record");
            self.consent_dialog_open = true;
            return;
        }

        let path = self.take_path("take");
        let slot = Arc::clone(&self.generation_slot);
        if let Ok(mut guard) = slot.lock() {
            *guard = None;
        }
        self.generation_kind = GenerationKind::Take;
        self.pending_story = None;
        self.generating = true;
        self.voice_status = format!("Rendering with {}…", profile.name);

        let request = TtsRequest {
            text: text.clone(),
            voice_id: profile.id.clone(),
            speed: profile.default_params.speed.max(0.25),
        };
        let voice_name = profile.name.clone();
        self.jobs.spawn("Generate speech", move |reporter| {
            let buffer =
                super::model_actions::synthesize_via_tier(&request, &mut |line: String| {
                    reporter.detail(line)
                })
                .map_err(|error| {
                    let message = error.to_string();
                    if let Ok(mut guard) = slot.lock() {
                        *guard = Some(Err(message.clone()));
                    }
                    message
                })?;
            reporter.progress(80.0);
            let settings = ExportSettings::wav(WavBitDepth::Int24);
            if let Err(error) = export(&path, &buffer, &settings) {
                let message = error.to_string();
                if let Ok(mut guard) = slot.lock() {
                    *guard = Some(Err(message.clone()));
                }
                return Err(message);
            }
            let take = GeneratedTake {
                text,
                voice_name,
                path,
                duration_secs: buffer.duration_secs() as f32,
                kind: "Generated",
            };
            if let Ok(mut guard) = slot.lock() {
                *guard = Some(Ok(RenderOutput::Take(take)));
            }
            reporter.progress(100.0);
            Ok("Speech rendered".into())
        });
    }

    /// Drain a finished background render.
    pub(super) fn poll_generation(&mut self) {
        let result = self
            .generation_slot
            .lock()
            .ok()
            .and_then(|mut guard| guard.take());
        let Some(result) = result else {
            return;
        };
        self.generating = false;
        match result {
            Ok(RenderOutput::Take(take)) => {
                self.voice_status = format!("{} · {:.2}s", take.voice_name, take.duration_secs);
                self.push_history(take);
            }
            Ok(RenderOutput::Story { take, lines }) => {
                if let Some(index) = self.pending_story
                    && let Some(story) = self.stories.get_mut(index)
                {
                    for (line_index, buffer) in lines {
                        if let Some(line) = story.lines.get_mut(line_index) {
                            line.set_audio(buffer);
                        }
                    }
                }
                self.pending_story = None;
                self.voice_status = format!("Story rendered · {:.2}s", take.duration_secs);
                self.push_history(take);
            }
            Err(error) => {
                self.pending_story = None;
                self.voice_status = format!("Render failed: {error}");
                self.show_toast(format!("Render failed: {error}"));
            }
        }
    }

    fn push_history(&mut self, take: GeneratedTake) {
        self.player_title = preview_text(&take.text);
        self.media_duration = take.duration_secs.max(0.01);
        let item = HistoryItem {
            text: take.text,
            voice_name: take.voice_name,
            path: take.path,
            duration_secs: take.duration_secs,
            created: clock_label(),
            kind: take.kind,
        };
        self.history.insert(0, item);
        self.history.truncate(200);
        self.show_toast(format!("Rendered {} take", take.kind.to_lowercase()));
    }

    fn take_path(&self, prefix: &str) -> PathBuf {
        self.media_root
            .join(format!("{prefix}-{}.wav", alp_core::now_ms()))
    }

    /// Play a finished file through the engine.
    pub(super) fn play_file(&mut self, path: &Path) -> Result<(), String> {
        let buffer = decode(path).map_err(|error| error.to_string())?;
        self.play_buffer(&buffer)
    }

    /// Play an in-memory buffer with the one-clip preview scene.
    pub(super) fn play_buffer(&mut self, buffer: &PcmBuffer) -> Result<(), String> {
        let source = Arc::new(SourceAudio::from_pcm(buffer));
        let frames = source.frames();
        if frames == 0 {
            return Err("the take is empty".into());
        }
        let clip = SceneClip {
            source,
            source_start: 0,
            source_end: frames,
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
        let scene = MixScene::new(buffer.sample_rate(), buffer.channels(), vec![clip])
            .map_err(|error| error.to_string())?;
        if let Some(handle) = self.playback.take() {
            handle.stop();
        }
        match alp_engine::play_scene_live(&scene, None) {
            Ok(handle) => {
                self.playback = Some(handle);
                self.playback_rate = buffer.sample_rate();
                self.playback_meters = None;
                self.media_duration = buffer.duration_secs() as f32;
                self.position_secs = 0.0;
                self.playing = true;
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        }
    }

    /// Play a generated take from the history feed.
    pub(super) fn play_history(&mut self, index: usize) {
        let Some(item) = self.history.get(index) else {
            return;
        };
        let path = item.path.clone();
        self.player_title = preview_text(&item.text);
        match self.play_file(&path) {
            Ok(()) => self.show_toast("Previewing generated take"),
            Err(error) => self.show_toast(format!("Cannot preview: {error}")),
        }
    }

    /// Insert a generated take into the timeline as an undoable import.
    pub(super) fn insert_history(&mut self, index: usize) {
        let Some(item) = self.history.get(index) else {
            return;
        };
        let path = item.path.clone();
        match decode(&path) {
            Ok(buffer) => {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("voice-take.wav")
                    .to_owned();
                let fallback = path.to_string_lossy().to_string();
                self.session.import_buffer(buffer, &name, &fallback);
                self.sync_edit_fields();
                self.show_toast("Inserted take into the timeline");
            }
            Err(error) => self.show_toast(format!("Cannot insert: {error}")),
        }
    }

    /// Delete a generated take and its file.
    pub(super) fn delete_history(&mut self, index: usize) {
        if index >= self.history.len() {
            return;
        }
        let item = self.history.remove(index);
        let _ = std::fs::remove_file(item.path);
        self.show_toast("Removed generated take");
    }

    /// Create a designed profile from the name and description fields.
    pub(super) fn create_designed_profile(&mut self) {
        let name = self.profile_name.trim();
        if name.is_empty() {
            self.show_toast("Give the voice a name first");
            return;
        }
        let request = VoiceDesignRequest {
            name: name.to_owned(),
            prompt: self.profile_tags.clone(),
            tags: self
                .profile_tags
                .split(',')
                .map(|tag| tag.trim().to_owned())
                .filter(|tag| !tag.is_empty())
                .collect(),
        };
        let mut profile = design_voice(&request);
        profile.id = format!("design-{}", alp_core::now_ms());
        profile.kind = VoiceKind::Designed;
        profile.created_at = alp_core::now_ms() / 1_000;
        self.persist_profile(profile);
        self.show_toast("Designed voice saved to the library");
    }

    /// Route the Create Voice dialog to the right creation path.
    ///
    /// The record tab needs a sample and a consent record; the other tabs are
    /// visible but explain what is still missing instead of pretending to work.
    pub(super) fn submit_create_voice(&mut self) {
        if self.profile_name.trim().is_empty() {
            self.show_toast("Give the voice a name first");
            return;
        }
        match self.create_voice_tab {
            super::CreateVoiceTab::Record => {
                if self.record_path.is_none() {
                    self.show_toast("Record a reference sample first");
                    return;
                }
                if !self.create_voice_consent {
                    self.show_toast("Declare the speaker's permission to continue");
                    return;
                }
                self.consent_dialog_open = true;
                self.show_toast("Confirm the consent record to finish this voice");
            }
            super::CreateVoiceTab::Upload => {
                self.show_toast("Drop a WAV on the window, record it, then clone the take");
            }
            super::CreateVoiceTab::SystemAudio => {
                self.show_toast("System audio capture is not available in this build");
            }
        }
    }

    /// Create a cloned profile from the last recording, requiring consent.
    pub(super) fn create_cloned_profile(&mut self) {
        if self.consent_speaker.trim().is_empty() || self.consent_attester.trim().is_empty() {
            self.show_toast("Consent needs the speaker and the attesting engineer");
            return;
        }
        let Some(record_path) = self.record_path.clone() else {
            self.show_toast("Record or import a reference sample first");
            return;
        };
        let buffer = match decode(&record_path) {
            Ok(buffer) => buffer,
            Err(error) => {
                self.show_toast(format!("Cannot read reference: {error}"));
                return;
            }
        };
        let report = alp_voice::analyze_reference_quality(&buffer);
        if !report.passed {
            let advice = report.advice.first().cloned().unwrap_or_default();
            self.show_toast(format!("Reference rejected: {advice}"));
            return;
        }
        let Ok(consent) = ConsentRecord::new(
            &self.consent_speaker,
            &self.consent_attester,
            CONSENT_STATEMENT,
        ) else {
            self.show_toast("Consent record is incomplete");
            return;
        };
        let id = format!("clone-{}", alp_core::now_ms());
        let mut profile = VoiceProfile::builtin(
            &id,
            self.profile_name.trim(),
            vec!["cloned".into(), "custom".into()],
        );
        profile.kind = VoiceKind::Cloned;
        profile.backend = "builtin-acoustic".into();
        profile.transcript = Some(self.voice_text.clone());
        profile.created_at = alp_core::now_ms() / 1_000;
        profile.consent = Some(consent);

        if let Some(store) = &self.profile_store {
            let directory = store.root_dir().join(&id);
            if let Err(error) = std::fs::create_dir_all(&directory) {
                self.show_toast(format!("Cannot create profile folder: {error}"));
                return;
            }
            let target = directory.join("reference.wav");
            if let Err(error) = std::fs::copy(&record_path, &target) {
                self.show_toast(format!("Cannot copy reference: {error}"));
                return;
            }
            profile.reference_audio = Some(PathBuf::from("reference.wav"));
        }
        self.persist_profile(profile);
        self.consent_dialog_open = false;
        self.create_voice_open = false;
        self.show_toast("Cloned voice saved with a consent record");
    }

    fn persist_profile(&mut self, profile: VoiceProfile) {
        let id = profile.id.clone();
        if let Some(store) = &self.profile_store
            && let Err(error) = store.save_profile(&profile)
        {
            self.show_toast(format!("Cannot save profile: {error}"));
            return;
        }
        self.profiles.retain(|existing| existing.id != id);
        self.profiles.push(profile.clone());
        self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
        self.select_profile(&id);
    }

    /// Delete a profile from the library.
    pub(super) fn delete_profile(&mut self, id: &str) {
        if self
            .profiles
            .iter()
            .any(|profile| profile.id == id && profile.kind == VoiceKind::Builtin)
        {
            self.show_toast("Built-in voices cannot be deleted");
            return;
        }
        if let Some(store) = &self.profile_store {
            match store.delete_profile(id) {
                Ok(true) => {}
                Ok(false) => {}
                Err(error) => {
                    self.show_toast(format!("Cannot delete profile: {error}"));
                    return;
                }
            }
        }
        self.profiles.retain(|profile| profile.id != id);
        if self.selected_profile == id {
            if let Some(first) = self.profiles.first() {
                let first_id = first.id.clone();
                self.select_profile(&first_id);
            } else {
                self.selected_profile.clear();
            }
        }
        self.show_toast("Voice profile deleted");
    }

    /// Export one profile as a `.alpvoice` bundle.
    pub(super) fn export_profile(&mut self, id: &str) {
        let Some(store) = &self.profile_store else {
            self.show_toast("Profile store unavailable");
            return;
        };
        let path = self
            .media_root
            .parent()
            .unwrap_or(&self.media_root)
            .join("exports")
            .join(format!("{id}.alpvoice"));
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            self.show_toast(format!("Cannot create export folder: {error}"));
            return;
        }
        match store.export_bundle(id, &path) {
            Ok(()) => self.show_toast(format!("Exported {}", path.display())),
            Err(error) => self.show_toast(format!("Export failed: {error}")),
        }
    }

    /// Import a `.alpvoice` bundle.
    pub(super) fn import_profile(&mut self, path: PathBuf) {
        let Some(store) = &self.profile_store else {
            self.show_toast("Profile store unavailable");
            return;
        };
        match store.import_bundle(&path) {
            Ok(profile) => {
                self.reload_profiles();
                self.select_profile(&profile.id);
                self.show_toast(format!("Imported {}", profile.name));
            }
            Err(error) => self.show_toast(format!("Import failed: {error}")),
        }
    }

    /// Add the typed line to the selected story.
    pub(super) fn story_add_line(&mut self) {
        let text = self.story_text.trim().to_owned();
        if text.is_empty() {
            self.show_toast("Type a line first");
            return;
        }
        let voice = self.selected_profile.clone();
        if let Some(story) = self.stories.get_mut(self.selected_story) {
            let index = story.lines.len();
            let line = story.add_line(text);
            line.voice_id = voice;
            self.story_text.clear();
            self.selected_story_line = Some(index);
        }
    }

    /// Delete one line from the selected story.
    pub(super) fn story_delete_line(&mut self, index: usize) {
        if let Some(story) = self.stories.get_mut(self.selected_story)
            && index < story.lines.len()
        {
            story.lines.remove(index);
            self.selected_story_line = None;
        }
    }

    /// Create a fresh story and select it.
    pub(super) fn story_new(&mut self) {
        let title = format!("Story {}", self.stories.len() + 1);
        let mut script = alp_voice::Script::new(title, &self.selected_profile);
        script.add_line("A new story begins here.");
        self.stories.insert(0, script);
        self.selected_story = 0;
        self.selected_story_line = None;
        self.show_toast("Created a new story");
    }

    /// Persist the selected story as JSON.
    pub(super) fn story_save(&mut self) {
        let Some(story) = self.stories.get(self.selected_story) else {
            return;
        };
        let slug = slugify(&story.title);
        let path = self.story_dir.join(format!("{slug}.json"));
        match serde_json::to_string_pretty(story)
            .map_err(|error| error.to_string())
            .and_then(|text| std::fs::write(&path, text).map_err(|error| error.to_string()))
        {
            Ok(()) => self.show_toast(format!("Saved {}", path.display())),
            Err(error) => self.show_toast(format!("Save failed: {error}")),
        }
    }

    /// Render every pending line of the selected story and stitch one take.
    pub(super) fn story_render(&mut self) {
        if self.generating {
            self.show_toast("A render is already running");
            return;
        }
        let Some(story) = self.stories.get(self.selected_story).cloned() else {
            return;
        };
        if story.lines.is_empty() {
            self.show_toast("Add a line before rendering");
            return;
        }
        let path = self.take_path("story");
        let slot = Arc::clone(&self.generation_slot);
        if let Ok(mut guard) = slot.lock() {
            *guard = None;
        }
        self.generation_kind = GenerationKind::Story;
        self.pending_story = Some(self.selected_story);
        self.generating = true;
        let title = story.title.clone();
        let total = story.lines.len().max(1);

        self.jobs.spawn("Render story", move |reporter| {
            let mut rendered = story;
            for (index, line) in rendered.lines.iter_mut().enumerate() {
                if reporter.is_cancelled() {
                    return Err("cancelled".into());
                }
                if line.audio.is_some() {
                    continue;
                }
                reporter.detail(format!("Rendering line {} of {}", index + 1, total));
                reporter.progress((index as f32 / total as f32) * 90.0);
                let request = TtsRequest {
                    text: line.text.clone(),
                    voice_id: line.voice_id.clone(),
                    speed: line.speed.max(0.25),
                };
                match super::model_actions::synthesize_via_tier(&request, &mut |line: String| {
                    reporter.detail(line)
                }) {
                    Ok(buffer) => line.set_audio(buffer),
                    Err(error) => {
                        let message = error.to_string();
                        if let Ok(mut guard) = slot.lock() {
                            *guard = Some(Err(message.clone()));
                        }
                        return Err(message);
                    }
                }
            }
            let stitched = rendered.stitch(0.22).map_err(|error| {
                let message = error.to_string();
                if let Ok(mut guard) = slot.lock() {
                    *guard = Some(Err(message.clone()));
                }
                message
            })?;
            let settings = ExportSettings::wav(WavBitDepth::Int24);
            export(&path, &stitched, &settings).map_err(|error| error.to_string())?;
            let lines = rendered
                .lines
                .iter()
                .enumerate()
                .filter_map(|(index, line)| line.audio.clone().map(|audio| (index, audio)))
                .collect();
            let take = GeneratedTake {
                text: format!("{title} · {} lines", rendered.lines.len()),
                voice_name: "Story mix".into(),
                path,
                duration_secs: stitched.duration_secs() as f32,
                kind: "Story",
            };
            if let Ok(mut guard) = slot.lock() {
                *guard = Some(Ok(RenderOutput::Story { take, lines }));
            }
            reporter.progress(100.0);
            Ok("Story rendered".into())
        });
    }

    /// Audit the last recording against the reference-quality gates.
    pub(super) fn audit_reference(&mut self) {
        let Some(path) = self.record_path.clone() else {
            self.reference_audit_status = Some("No reference recording yet".into());
            self.show_toast("Record or import a reference sample first");
            return;
        };
        let buffer = match decode(&path) {
            Ok(buffer) => buffer,
            Err(error) => {
                self.reference_audit_status = Some(format!("Cannot read reference: {error}"));
                return;
            }
        };
        let report = alp_voice::analyze_reference_quality(&buffer);
        let verdict = if report.passed { "PASSED" } else { "FAILED" };
        self.reference_audit_status = Some(format!(
            "{verdict} · {:.1}s · peak {:.1} dBFS · noise {:.1} dBFS",
            report.duration_secs, report.peak_dbfs, report.noise_floor_dbfs
        ));
        self.show_toast(
            report
                .advice
                .first()
                .cloned()
                .unwrap_or_else(|| format!("Reference quality audit: {verdict}")),
        );
    }

    /// Remember a finished recording as a capture row.
    pub(super) fn record_capture(&mut self, path: &Path, duration_secs: f32, peak: f32) {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map_or_else(|| "capture".to_owned(), ToOwned::to_owned);
        self.captures.insert(
            0,
            CaptureItem {
                name,
                path: Some(path.to_path_buf()),
                duration_secs,
                transcript: String::new(),
                peak,
            },
        );
        self.selected_capture = 0;
    }

    /// Transcribe the selected capture, daemon-first: the `alp-infer` worker
    /// serves whichever transcription tier is active (Moonshine when loaded,
    /// its built-in segmenter otherwise), and the in-process acoustic
    /// backend covers an absent daemon — mirroring `synthesize_via_tier`.
    /// The toast names the engine that actually ran.
    pub(super) fn transcribe_capture(&mut self) {
        let Some(capture) = self.captures.get(self.selected_capture) else {
            self.show_toast("Select a capture first");
            return;
        };
        let Some(path) = capture.path.clone() else {
            return;
        };
        let buffer = match decode(&path) {
            Ok(buffer) => buffer,
            Err(error) => {
                self.show_toast(format!("Cannot read capture: {error}"));
                return;
            }
        };

        let mut result: Option<(Vec<_>, String)> = None;
        let client = InferClient::new();
        if client.is_alive() {
            match client.transcribe(&buffer) {
                Ok(segments) => {
                    // Ask the same daemon which tier served the request: a
                    // neural model is named; the daemon-side built-in keeps
                    // its placeholder-words admission.
                    let note = match client.status().ok().and_then(|s| s.active_stt_model) {
                        Some(model) => format!(" — transcribed by {model}"),
                        None => " — built-in STT emits placeholder words".to_owned(),
                    };
                    result = Some((segments, note));
                }
                Err(error) => {
                    self.show_toast(format!(
                        "daemon transcription failed ({error}); using the built-in segmenter"
                    ));
                }
            }
        }

        let (segments, note) = match result {
            Some(segments) => segments,
            None => {
                let backend = BuiltinSttBackend::new();
                match backend.transcribe(&buffer) {
                    Ok(segments) => (
                        segments,
                        " — built-in STT emits placeholder words".to_owned(),
                    ),
                    Err(error) => {
                        self.show_toast(format!("Transcription failed: {error}"));
                        return;
                    }
                }
            }
        };

        self.transcript = alp_voice::Transcript { segments };
        let count = self.transcript.segments.len();
        if let Some(capture) = self.captures.get_mut(self.selected_capture) {
            capture.transcript = self.transcript.to_plain_text();
        }
        self.show_toast(format!("Detected {count} speech region(s){note}"));
    }

    /// Write the current transcript to `exports/` as SRT or plain text.
    pub(super) fn export_transcript(&mut self, srt: bool) {
        if self.transcript.segments.is_empty() {
            self.show_toast("The transcript is empty");
            return;
        }
        let rate = self.session.project().sample_rate;
        let (extension, body) = if srt {
            ("srt", self.transcript.to_srt(rate))
        } else {
            ("txt", self.transcript.to_plain_text())
        };
        let path = self
            .media_root
            .parent()
            .unwrap_or(&self.media_root)
            .join("exports")
            .join(format!("transcript-{}.{extension}", alp_core::now_ms()));
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            self.show_toast(format!("Cannot create export folder: {error}"));
            return;
        }
        match std::fs::write(&path, body) {
            Ok(()) => self.show_toast(format!("Wrote {}", path.display())),
            Err(error) => self.show_toast(format!("Cannot write transcript: {error}")),
        }
    }

    /// Accept a model license so it can be installed later.
    pub(super) fn accept_model_license(&mut self, id: &str) {
        let model_id = alp_voice::ModelId::new(id);
        match self.model_manager.accept_license(&model_id) {
            Ok(()) => self.show_toast(format!("Accepted the {id} license")),
            Err(error) => self.show_toast(format!("License not accepted: {error}")),
        }
    }

    /// Remove an installed model from disk.
    pub(super) fn remove_model(&mut self, id: &str) {
        let model_id = alp_voice::ModelId::new(id);
        match self.model_manager.remove(&model_id) {
            Ok(()) => self.show_toast(format!("Removed {id}")),
            Err(error) => self.show_toast(format!("Cannot remove model: {error}")),
        }
    }

    /// Toggle offline-only model handling.
    pub(super) fn toggle_offline_models(&mut self) {
        self.models_offline = !self.models_offline;
        self.model_manager.set_offline(self.models_offline);
        self.show_toast(if self.models_offline {
            "Models are offline-only; nothing is downloaded"
        } else {
            "Model downloads are allowed by policy"
        });
    }
}

fn voice_kind_label(kind: VoiceKind) -> &'static str {
    match kind {
        VoiceKind::Builtin => "built-in",
        VoiceKind::Cloned => "cloned",
        VoiceKind::Designed => "designed",
    }
}

fn slugify(title: &str) -> String {
    let slug: String = title
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        "story".into()
    } else {
        slug
    }
}

fn clock_label() -> String {
    let secs = alp_core::now_ms() / 1_000;
    format!("{:02}:{:02} UTC", (secs / 3_600) % 24, (secs / 60) % 60)
}

#[cfg(test)]
mod tests {
    use super::{clock_label, slugify, voice_kind_label};
    use alp_voice::VoiceKind;

    #[test]
    fn slugify_lowercases_and_replaces_punctuation() {
        assert_eq!(slugify("Morning Walk"), "morning-walk");
        assert_eq!(slugify("Hello, World!"), "hello--world");
        assert_eq!(slugify("  spaced  "), "spaced");
    }

    #[test]
    fn slugify_falls_back_to_story_when_nothing_survives() {
        assert_eq!(slugify(""), "story");
        assert_eq!(slugify("---"), "story");
        assert_eq!(slugify("é"), "story");
    }

    #[test]
    fn clock_label_is_short_utc_time() {
        let label = clock_label();
        assert_eq!(label.len(), 9, "unexpected label {label}");
        assert!(label.ends_with(" UTC"), "unexpected label {label}");
        assert_eq!(&label[2..3], ":");
    }

    #[test]
    fn voice_kinds_have_labels() {
        assert_eq!(voice_kind_label(VoiceKind::Builtin), "built-in");
        assert_eq!(voice_kind_label(VoiceKind::Cloned), "cloned");
        assert_eq!(voice_kind_label(VoiceKind::Designed), "designed");
    }
}
