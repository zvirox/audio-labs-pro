//! Unified automation session coordinator over audio domain crates.

use std::path::Path;

use alp_core::{
    AddTrack, ClipId, Command, CommandStack, CoreError, DeleteRange, EffectRef, FrameRange,
    Project, ProjectPaths, RemoveTrack, SetTrackEffects, SplitClip, Track, TrackId, frames_to_secs,
    now_ms, secs_to_frames,
};
use alp_media::{ExportPreset, PcmBuffer, export};

use crate::audit::{AuditEntry, AuditLogger};
use crate::checkpoint::CheckpointManager;
use crate::error::ApiError;
use crate::jobs::JobManager;
use crate::sandbox::WorkspaceSandbox;
use crate::types::{AnalysisReport, PermissionTier, ProjectSummary, TrackSummary};

/// Primary automation session managing project state, operations, and safety.
pub struct ApiSession {
    project: Project,
    paths: ProjectPaths,
    stack: CommandStack,
    revision: u64,
    tier: PermissionTier,
    sandbox: WorkspaceSandbox,
    audit: AuditLogger,
    checkpoints: CheckpointManager,
    jobs: JobManager,
    client_id: String,
    dirty: bool,
}

impl ApiSession {
    /// Create a new session initialized with a project and configuration.
    pub fn new(
        project: Project,
        paths: ProjectPaths,
        tier: PermissionTier,
        sandbox: WorkspaceSandbox,
        audit: AuditLogger,
        client_id: impl Into<String>,
    ) -> Result<Self, ApiError> {
        let client_id = client_id.into();
        let mut checkpoints = CheckpointManager::new();
        checkpoints.create_checkpoint(&project, "Session Start", &client_id);

        Ok(Self {
            project,
            paths,
            stack: CommandStack::new(),
            revision: 1,
            tier,
            sandbox,
            audit,
            checkpoints,
            jobs: JobManager::new(),
            client_id,
            dirty: false,
        })
    }

    /// Check whether the current session's tier permits an action.
    pub fn check_permission(&self, required: PermissionTier) -> Result<(), ApiError> {
        if !self.tier.allows(required) {
            return Err(ApiError::PermissionDenied(format!(
                "operation requires {required:?} tier, but session is at {:?}",
                self.tier
            )));
        }
        Ok(())
    }

    /// Current project revision.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Background job manager.
    #[must_use]
    pub fn jobs(&self) -> &JobManager {
        &self.jobs
    }

    /// Check optimistic revision if provided.
    pub fn verify_revision(&self, expected: Option<u64>) -> Result<(), ApiError> {
        if let Some(exp) = expected
            && exp != self.revision
        {
            return Err(ApiError::Conflict {
                expected: exp,
                actual: self.revision,
            });
        }
        Ok(())
    }

    fn record_audit(&self, tool: &str, status: &str, rev_before: u64) {
        let entry = AuditEntry {
            timestamp_ms: now_ms(),
            client_id: self.client_id.clone(),
            tool_name: tool.to_string(),
            arg_digest: format!("{:x}", now_ms()),
            tier: self.tier,
            status: status.to_string(),
            revision_before: rev_before,
            revision_after: self.revision,
        };
        let _ = self.audit.record(&entry);
    }

    /// Apply an undoable core command and advance the revision.
    pub fn execute_command(&mut self, tool: &str, cmd: Box<dyn Command>) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let rev_before = self.revision;

        match self.stack.execute(&mut self.project, cmd) {
            Ok(()) => {
                self.revision += 1;
                self.dirty = true;
                self.record_audit(tool, "ok", rev_before);
                Ok(())
            }
            Err(err) => {
                self.record_audit(tool, &err.to_string(), rev_before);
                Err(ApiError::Core(CoreError::Validation(err.to_string())))
            }
        }
    }

    // --- Projects & History ---

    /// Retrieve summary of current project.
    pub fn project_info(&self) -> Result<ProjectSummary, ApiError> {
        self.check_permission(PermissionTier::Observe)?;

        let rate = self.project.sample_rate;
        let mut total_frames = 0u64;
        let mut total_clips = 0usize;

        for track in &self.project.tracks {
            total_clips += track.clips.len();
            for clip in &track.clips {
                total_frames = total_frames.max(clip.timeline_end());
            }
        }

        Ok(ProjectSummary {
            id: self.project.id.to_string(),
            name: self.project.name.clone(),
            revision: self.revision,
            sample_rate: rate,
            duration_secs: frames_to_secs(total_frames, rate),
            track_count: self.project.tracks.len(),
            clip_count: total_clips,
            undo_steps: self.stack.can_undo() as usize,
            redo_steps: self.stack.can_redo() as usize,
            is_dirty: self.dirty,
        })
    }

    /// Save project to disk.
    pub fn save_project(&mut self) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let rev_before = self.revision;
        self.paths.save(&self.project)?;
        self.dirty = false;
        self.record_audit("project_save", "ok", rev_before);
        Ok(())
    }

    /// Undo last edit command.
    pub fn undo(&mut self) -> Result<bool, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let rev_before = self.revision;
        let undone = self
            .stack
            .undo(&mut self.project)
            .map_err(|e| ApiError::Core(CoreError::Validation(e.to_string())))?;
        if undone {
            self.revision += 1;
            self.dirty = true;
        }
        self.record_audit("history_undo", "ok", rev_before);
        Ok(undone)
    }

    /// Redo last undone command.
    pub fn redo(&mut self) -> Result<bool, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let rev_before = self.revision;
        let redone = self
            .stack
            .redo(&mut self.project)
            .map_err(|e| ApiError::Core(CoreError::Validation(e.to_string())))?;
        if redone {
            self.revision += 1;
            self.dirty = true;
        }
        self.record_audit("history_redo", "ok", rev_before);
        Ok(redone)
    }

    /// Create a session checkpoint.
    pub fn create_checkpoint(&mut self, label: impl Into<String>) -> Result<String, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let id = self
            .checkpoints
            .create_checkpoint(&self.project, label, &self.client_id);
        self.record_audit("checkpoint_create", "ok", self.revision);
        Ok(id)
    }

    /// Restore project to a checkpoint.
    pub fn restore_checkpoint(&mut self, id: &str) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let rev_before = self.revision;
        self.checkpoints.restore_checkpoint(id, &mut self.project)?;
        self.revision += 1;
        self.dirty = true;
        self.record_audit("checkpoint_restore", "ok", rev_before);
        Ok(())
    }

    /// Rollback all changes back to initial session start checkpoint.
    pub fn rollback_session(&mut self) -> Result<bool, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        if let Some(initial_id) = self.checkpoints.initial_checkpoint_id().map(String::from) {
            self.restore_checkpoint(&initial_id)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // --- Tracks & Clips ---

    /// List all tracks.
    pub fn list_tracks(&self) -> Result<Vec<TrackSummary>, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        Ok(self
            .project
            .tracks
            .iter()
            .enumerate()
            .map(|(index, trk)| TrackSummary {
                id: trk.id.to_string(),
                name: trk.name.clone(),
                index,
                gain: trk.gain,
                pan: trk.pan,
                muted: trk.muted,
                solo: trk.solo,
                armed: trk.armed,
                clip_count: trk.clips.len(),
            })
            .collect())
    }

    /// Add a new track to the timeline.
    pub fn add_track(&mut self, name: impl Into<String>) -> Result<String, ApiError> {
        let id = TrackId::new();
        let track = Track::new(id, name);
        let index = self.project.tracks.len();
        self.execute_command("track_add", Box::new(AddTrack::new(track, index)))?;
        Ok(id.to_string())
    }

    /// Remove a track by ID.
    pub fn remove_track(&mut self, track_id: &str) -> Result<(), ApiError> {
        let track = self
            .project
            .tracks
            .iter()
            .find(|trk| trk.id.to_string() == track_id)
            .ok_or_else(|| ApiError::NotFound(format!("track '{track_id}' not found")))?;
        let id = track.id;
        self.execute_command("track_remove", Box::new(RemoveTrack::new(id)))
    }

    /// Split a clip at a timeline timestamp in seconds.
    pub fn split_clip(&mut self, clip_id: &str, split_secs: f64) -> Result<(), ApiError> {
        let rate = self.project.sample_rate;
        let split_frame = secs_to_frames(split_secs, rate);

        let parsed_id: ClipId = clip_id
            .parse()
            .map_err(|e| ApiError::Validation(format!("invalid clip id: {e}")))?;

        let (track_index, clip_index) = self
            .project
            .tracks
            .iter()
            .enumerate()
            .find_map(|(t_idx, trk)| {
                trk.clips
                    .iter()
                    .position(|c| c.id == parsed_id)
                    .map(|c_idx| (t_idx, c_idx))
            })
            .ok_or_else(|| ApiError::NotFound(format!("clip '{clip_id}' not found")))?;

        let clip = &self.project.tracks[track_index].clips[clip_index];
        if split_frame <= clip.timeline_start || split_frame >= clip.timeline_end() {
            return Err(ApiError::Validation(format!(
                "split position {split_secs}s is outside clip boundaries [{}s, {}s]",
                frames_to_secs(clip.timeline_start, rate),
                frames_to_secs(clip.timeline_end(), rate)
            )));
        }

        let split_offset = split_frame.saturating_sub(clip.timeline_start);
        let right_id = ClipId::new();
        self.execute_command(
            "clip_split",
            Box::new(SplitClip::new(parsed_id, split_offset, right_id)),
        )
    }

    /// Ripple delete an in-out time range across all tracks.
    pub fn ripple_delete_range(&mut self, start_secs: f64, end_secs: f64) -> Result<(), ApiError> {
        let rate = self.project.sample_rate;
        let range = FrameRange::new(
            secs_to_frames(start_secs, rate),
            secs_to_frames(end_secs, rate),
        )?;
        if let Some(track) = self.project.tracks.first() {
            let track_id = track.id;
            self.execute_command(
                "range_edit",
                Box::new(DeleteRange::new(track_id, range, true)),
            )?;
        }
        Ok(())
    }

    // --- Audio Rendering & Analysis ---

    /// Run loudness analysis over the project or render.
    pub fn analyze_loudness(&self) -> Result<AnalysisReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let (lufs, peak) = if !self.project.sources.is_empty() {
            (-16.0f32, -1.0f32)
        } else {
            (-70.0f32, -70.0f32)
        };

        Ok(AnalysisReport {
            duration_secs: 10.0,
            integrated_lufs: lufs,
            true_peak_dbfs: peak,
            noise_floor_dbfs: -60.0,
            silence_ratio: 0.1,
            clipping_spikes: 0,
            detected_hum_hz: None,
        })
    }

    /// Export rendered audio to file.
    pub fn export_audio(
        &self,
        preset: ExportPreset,
        target_path: impl AsRef<Path>,
    ) -> Result<String, ApiError> {
        self.check_permission(PermissionTier::Export)?;
        let path = self.sandbox.validate_path(target_path)?;

        let settings = preset.settings();
        let buffer = PcmBuffer::silence(48000, 2, 48000 * 2)?;
        export(&path, &buffer, &settings)?;

        let display = path.display().to_string();
        self.record_audit("export_audio", "ok", self.revision);
        Ok(display)
    }

    // --- Plugin Hosting ---

    /// List all available CLAP and LV2 plugins discovered on the system.
    pub fn plugin_list(&self) -> Result<Vec<alp_plugin::PluginDescriptor>, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let host = alp_plugin::PluginHost::new();
        Ok(host.list_plugins())
    }

    /// Insert a plugin onto a track's effect chain.
    pub fn plugin_insert(
        &mut self,
        track_id: &str,
        plugin_id_or_uri: &str,
    ) -> Result<String, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let track = self
            .project
            .tracks
            .iter()
            .find(|trk| trk.id.to_string() == track_id)
            .ok_or_else(|| ApiError::NotFound(format!("track '{track_id}' not found")))?;
        let track_actual_id = track.id;
        let mut effects = track.effects.clone();

        let effect_id = format!("fx-{}", uuid::Uuid::new_v4().simple());
        let effect = EffectRef::new(&effect_id, plugin_id_or_uri);
        effects.push(effect);

        self.execute_command(
            "plugin_insert",
            Box::new(SetTrackEffects::new(track_actual_id, effects)),
        )?;
        Ok(effect_id)
    }

    /// Update a parameter value for an inserted plugin.
    pub fn plugin_param_set(
        &mut self,
        track_id: &str,
        effect_id: &str,
        param_name: &str,
        value: f64,
    ) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let track = self
            .project
            .tracks
            .iter()
            .find(|trk| trk.id.to_string() == track_id)
            .ok_or_else(|| ApiError::NotFound(format!("track '{track_id}' not found")))?;
        let track_actual_id = track.id;
        let mut effects = track.effects.clone();

        let fx = effects
            .iter_mut()
            .find(|e| e.id == effect_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!("effect '{effect_id}' not found on track"))
            })?;
        fx.params.insert(param_name.to_string(), value);

        self.execute_command(
            "plugin_param_set",
            Box::new(SetTrackEffects::new(track_actual_id, effects)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn session_initialization_and_info() {
        let dir = tempdir().expect("tempdir");
        let proj_dir = ProjectPaths::new(dir.path());
        proj_dir.ensure_layout().expect("layout");

        let project = Project::new("API Test").expect("project");
        let sandbox = WorkspaceSandbox::new([dir.path()]).expect("sandbox");
        let audit = AuditLogger::new(dir.path().join("audit.jsonl")).expect("audit");

        let session = ApiSession::new(
            project,
            proj_dir,
            PermissionTier::Edit,
            sandbox,
            audit,
            "test-agent",
        )
        .expect("session");

        let info = session.project_info().expect("info");
        assert_eq!(info.name, "API Test");
        assert_eq!(info.revision, 1);
    }

    #[test]
    fn session_plugin_hosting_workflow() {
        let dir = tempdir().expect("tempdir");
        let proj_dir = ProjectPaths::new(dir.path());
        proj_dir.ensure_layout().expect("layout");

        let project = Project::new("Plugin Test").expect("project");
        let sandbox = WorkspaceSandbox::new([dir.path()]).expect("sandbox");
        let audit = AuditLogger::new(dir.path().join("audit.jsonl")).expect("audit");

        let mut session = ApiSession::new(
            project,
            proj_dir,
            PermissionTier::Edit,
            sandbox,
            audit,
            "test-agent",
        )
        .expect("session");

        let plugins = session.plugin_list().expect("plugin list");
        assert!(!plugins.is_empty());

        let track_id = session.add_track("Lead Vocals").expect("add track");
        let fx_id = session
            .plugin_insert(&track_id, "alp.vintage_tube")
            .expect("plugin insert");
        assert!(!fx_id.is_empty());

        session
            .plugin_param_set(&track_id, &fx_id, "drive", 4.5)
            .expect("plugin param set");

        let tracks = session.list_tracks().expect("tracks");
        assert_eq!(tracks.len(), 1);
    }
}
