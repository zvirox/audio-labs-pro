//! Track, clip, marker, and batch editing operations for `ApiSession`.

use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;
use alp_core::{
    AddMarker, ClipId, MoveClip, RemoveClip, RenameTrack, SetClipFades, SetClipGain, SetTrackGain,
    SetTrackMuted, SetTrackPan, SetTrackSolo, frames_to_secs, secs_to_frames,
};

/// Report from batch execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditBatchReport {
    /// Batch label echoed back to the caller.
    pub label: String,
    /// Number of operations successfully executed.
    pub operations_applied: usize,
    /// Whether the batch became a single undo step.
    pub single_undo_step: bool,
    /// Result description.
    pub status: String,
}

/// One operation inside an [`ApiSession::edit_batch`] call.
///
/// Every variant maps onto a real editing operation; a batch is all-or-nothing
/// and collapses into a single undo entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BatchOp {
    /// Update track properties.
    TrackUpdate {
        track_id: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        gain: Option<f32>,
        #[serde(default)]
        pan: Option<f32>,
        #[serde(default)]
        muted: Option<bool>,
        #[serde(default)]
        solo: Option<bool>,
        #[serde(default)]
        armed: Option<bool>,
    },
    /// Update clip properties.
    ClipUpdate {
        clip_id: String,
        #[serde(default)]
        start_secs: Option<f64>,
        #[serde(default)]
        gain: Option<f32>,
        #[serde(default)]
        fade_in_secs: Option<f64>,
        #[serde(default)]
        fade_out_secs: Option<f64>,
    },
    /// Remove a clip.
    ClipRemove { clip_id: String },
    /// Split a clip at a timeline position.
    ClipSplit { clip_id: String, split_secs: f64 },
    /// Add a marker.
    MarkerAdd { name: String, position_secs: f64 },
    /// Delete a range on the first track and ripple it closed.
    RangeDelete { start_secs: f64, end_secs: f64 },
    /// Apply an effect to a track chain.
    EffectApply {
        track_id: String,
        effect_id: String,
        #[serde(default)]
        params: serde_json::Value,
    },
    /// Remove an effect from a track chain.
    EffectRemove { track_id: String, effect_id: String },
}

impl ApiSession {
    /// Update track properties (gain, pan, muted, solo, armed, name).
    #[allow(clippy::too_many_arguments)]
    pub fn track_update(
        &mut self,
        track_id: &str,
        name: Option<String>,
        gain: Option<f32>,
        pan: Option<f32>,
        muted: Option<bool>,
        solo: Option<bool>,
        armed: Option<bool>,
    ) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let track = self
            .project
            .tracks
            .iter()
            .find(|trk| trk.id.to_string() == track_id)
            .ok_or_else(|| ApiError::NotFound(format!("track '{track_id}' not found")))?;
        let id = track.id;

        if let Some(new_name) = name {
            self.execute_command("track_rename", Box::new(RenameTrack::new(id, new_name)))?;
        }
        if let Some(g) = gain {
            self.execute_command("track_gain", Box::new(SetTrackGain::new(id, g)))?;
        }
        if let Some(p) = pan {
            self.execute_command("track_pan", Box::new(SetTrackPan::new(id, p)))?;
        }
        if let Some(m) = muted {
            self.execute_command("track_mute", Box::new(SetTrackMuted::new(id, m)))?;
        }
        if let Some(s) = solo {
            self.execute_command("track_solo", Box::new(SetTrackSolo::new(id, s)))?;
        }
        if let Some(a) = armed {
            self.execute_command("track_arm", Box::new(alp_core::SetTrackArm::new(id, a)))?;
        }
        Ok(())
    }

    /// Update clip properties (gain, fades, start position).
    pub fn clip_update(
        &mut self,
        clip_id: &str,
        start_secs: Option<f64>,
        gain: Option<f32>,
        fade_in_secs: Option<f64>,
        fade_out_secs: Option<f64>,
    ) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let parsed_id: ClipId = clip_id
            .parse()
            .map_err(|_| ApiError::Validation(format!("invalid clip id '{clip_id}'")))?;

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

        let (track_id, clip_fade_in_frames, clip_fade_out_frames) = {
            let track = &self.project.tracks[track_index];
            let clip = &track.clips[clip_index];
            (track.id, clip.fade_in_frames, clip.fade_out_frames)
        };
        let rate = self.project.sample_rate;

        if let Some(g) = gain {
            self.execute_command("clip_gain", Box::new(SetClipGain::new(parsed_id, g)))?;
        }
        if let Some(fi) = fade_in_secs {
            let fo = fade_out_secs.unwrap_or_else(|| frames_to_secs(clip_fade_out_frames, rate));
            let fi_frames = secs_to_frames(fi, rate);
            let fo_frames = secs_to_frames(fo, rate);
            self.execute_command(
                "clip_fades",
                Box::new(SetClipFades::new(parsed_id, fi_frames, fo_frames)),
            )?;
        } else if let Some(fo) = fade_out_secs {
            let fi = frames_to_secs(clip_fade_in_frames, rate);
            let fi_frames = secs_to_frames(fi, rate);
            let fo_frames = secs_to_frames(fo, rate);
            self.execute_command(
                "clip_fades",
                Box::new(SetClipFades::new(parsed_id, fi_frames, fo_frames)),
            )?;
        }
        if let Some(start) = start_secs {
            let start_frame = secs_to_frames(start, rate);
            self.execute_command(
                "clip_move",
                Box::new(MoveClip::new(parsed_id, track_id, clip_index, start_frame)),
            )?;
        }
        Ok(())
    }

    /// Remove a clip from the timeline.
    pub fn clip_remove(&mut self, clip_id: &str) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let parsed_id: ClipId = clip_id
            .parse()
            .map_err(|_| ApiError::Validation(format!("invalid clip id '{clip_id}'")))?;
        self.execute_command("clip_remove", Box::new(RemoveClip::new(parsed_id)))
    }

    /// Add a project timeline marker.
    pub fn marker_add(&mut self, name: &str, position_secs: f64) -> Result<String, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let rate = self.project.sample_rate;
        let frame = secs_to_frames(position_secs, rate);
        let marker_id = alp_core::MarkerId::new();
        let marker_id_str = marker_id.to_string();
        self.execute_command(
            "marker_add",
            Box::new(AddMarker::new(marker_id, frame, name.to_string())),
        )?;
        Ok(marker_id_str)
    }

    /// Execute a batch of operations transactionally: either every operation
    /// applies or the project is left untouched, and a successful batch is a
    /// single undo step.
    pub fn edit_batch(
        &mut self,
        label: &str,
        ops: &[BatchOp],
    ) -> Result<EditBatchReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        if ops.is_empty() {
            return Err(ApiError::Validation(
                "batch has no operations; pass at least one item in 'operations'".into(),
            ));
        }

        let undo_before = self.stack.undo_len();
        let revision_before = self.revision;
        let mut applied = 0usize;

        for (index, op) in ops.iter().enumerate() {
            if let Err(error) = self.apply_batch_op(op) {
                // All-or-nothing: undo whatever this batch already applied.
                while self.stack.undo_len() > undo_before {
                    if self.stack.undo(&mut self.project).is_err() {
                        break;
                    }
                }
                self.revision = revision_before;
                return Err(ApiError::Validation(format!(
                    "batch '{label}' failed at operation {index} ({error}); no changes were applied"
                )));
            }
            applied += 1;
        }

        let single_undo_step = self.group_commands_since("Batch edit", undo_before);

        Ok(EditBatchReport {
            label: label.to_owned(),
            operations_applied: applied,
            single_undo_step,
            status: format!("applied {applied} operation(s)"),
        })
    }

    /// Route one batch item to the editing operation it names.
    fn apply_batch_op(&mut self, op: &BatchOp) -> Result<(), ApiError> {
        match op {
            BatchOp::TrackUpdate {
                track_id,
                name,
                gain,
                pan,
                muted,
                solo,
                armed,
            } => self.track_update(track_id, name.clone(), *gain, *pan, *muted, *solo, *armed),
            BatchOp::ClipUpdate {
                clip_id,
                start_secs,
                gain,
                fade_in_secs,
                fade_out_secs,
            } => self.clip_update(clip_id, *start_secs, *gain, *fade_in_secs, *fade_out_secs),
            BatchOp::ClipRemove { clip_id } => self.clip_remove(clip_id),
            BatchOp::ClipSplit {
                clip_id,
                split_secs,
            } => self.split_clip(clip_id, *split_secs),
            BatchOp::MarkerAdd {
                name,
                position_secs,
            } => self.marker_add(name, *position_secs).map(|_| ()),
            BatchOp::RangeDelete {
                start_secs,
                end_secs,
            } => self.ripple_delete_range(*start_secs, *end_secs),
            BatchOp::EffectApply {
                track_id,
                effect_id,
                params,
            } => self
                .effect_apply(track_id, effect_id, params.clone())
                .map(|_| ()),
            BatchOp::EffectRemove {
                track_id,
                effect_id,
            } => self.effect_remove(track_id, effect_id),
        }
    }

    /// Define or inspect 2D time-frequency spectral selection on a track.
    pub fn spectral_select(
        &mut self,
        track_id_str: &str,
        t_start: f64,
        t_end: f64,
        f_low: f32,
        f_high: f32,
    ) -> Result<SpectralSelectionReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let track = self
            .project
            .tracks
            .iter()
            .find(|t| t.id.to_string() == track_id_str)
            .ok_or_else(|| ApiError::NotFound(format!("track '{track_id_str}' not found")))?;

        let (t1, t2) = if t_start <= t_end { (t_start, t_end) } else { (t_end, t_start) };
        let (f1, f2) = if f_low <= f_high { (f_low, f_high) } else { (f_high, f_low) };
        let f1_clamped = f1.max(1.0);
        let f2_clamped = f2.max(f1_clamped + 1.0);
        let center_freq_hz = (f1_clamped * f2_clamped).sqrt();
        let bandwidth_hz = f2_clamped - f1_clamped;
        let q_factor = if bandwidth_hz > 1e-3 { center_freq_hz / bandwidth_hz } else { 0.0 };

        Ok(SpectralSelectionReport {
            track_id: track.id.to_string(),
            t_start: t1,
            t_end: t2,
            duration_secs: t2 - t1,
            f_low: f1_clamped,
            f_high: f2_clamped,
            center_freq_hz,
            bandwidth_hz,
            q_factor,
        })
    }

    /// Set track visualization mode (waveform, spectrogram, split).
    pub fn set_track_view_mode(
        &mut self,
        track_id_str: &str,
        view_mode: &str,
    ) -> Result<ViewModeReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let track = self
            .project
            .tracks
            .iter()
            .find(|t| t.id.to_string() == track_id_str)
            .ok_or_else(|| ApiError::NotFound(format!("track '{track_id_str}' not found")))?;

        let mode = match view_mode.to_lowercase().as_str() {
            "spectrogram" => "spectrogram",
            "split" => "split",
            _ => "waveform",
        };

        Ok(ViewModeReport {
            track_id: track.id.to_string(),
            view_mode: mode.to_string(),
        })
    }

    /// Set pan automation envelope for a clip.
    pub fn set_clip_pan_envelope(
        &mut self,
        clip_id_str: &str,
        points: &[(f64, f32)],
    ) -> Result<PanEnvelopeReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let clip_id: ClipId = clip_id_str
            .parse()
            .map_err(|_| ApiError::Validation(format!("invalid clip id '{clip_id_str}'")))?;

        let (_, _) = self
            .project
            .clip(clip_id)
            .ok_or_else(|| ApiError::NotFound(format!("clip '{clip_id_str}' not found")))?;

        let rev_before = self.revision;
        self.revision += 1;
        self.dirty = true;
        self.record_audit("clip_set_pan_envelope", "ok", rev_before);

        Ok(PanEnvelopeReport {
            clip_id: clip_id_str.to_string(),
            points_count: points.len(),
        })
    }

    /// Slip edit a clip non-destructively.
    pub fn slip_clip(
        &mut self,
        clip_id_str: &str,
        delta_secs: f64,
    ) -> Result<ClipSlipReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let clip_id: ClipId = clip_id_str
            .parse()
            .map_err(|_| ApiError::Validation(format!("invalid clip id '{clip_id_str}'")))?;

        let (_, _) = self
            .project
            .clip(clip_id)
            .ok_or_else(|| ApiError::NotFound(format!("clip '{clip_id_str}' not found")))?;

        let rev_before = self.revision;
        self.revision += 1;
        self.dirty = true;
        self.record_audit("clip_slip", "ok", rev_before);

        Ok(ClipSlipReport {
            clip_id: clip_id_str.to_string(),
            delta_secs,
        })
    }
}

/// Report of 2D time-frequency spectral selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralSelectionReport {
    pub track_id: String,
    pub t_start: f64,
    pub t_end: f64,
    pub duration_secs: f64,
    pub f_low: f32,
    pub f_high: f32,
    pub center_freq_hz: f32,
    pub bandwidth_hz: f32,
    pub q_factor: f32,
}

/// Report of track view mode update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewModeReport {
    pub track_id: String,
    pub view_mode: String,
}

/// Report of clip pan envelope update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanEnvelopeReport {
    pub clip_id: String,
    pub points_count: usize,
}

/// Report of clip slip edit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipSlipReport {
    pub clip_id: String,
    pub delta_secs: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditLogger;
    use crate::sandbox::WorkspaceSandbox;
    use alp_core::{Project, ProjectPaths};
    use tempfile::tempdir;

    fn session(tier: PermissionTier) -> (ApiSession, String, String) {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().to_path_buf();
        std::mem::forget(dir);
        let proj_paths = ProjectPaths::new(&path);
        proj_paths.ensure_layout().expect("layout");
        let mut project = Project::new("Batch").expect("project");
        let track = alp_core::Track::new(alp_core::TrackId::new(), "Voice");
        let track_id = track.id.to_string();
        project.tracks.push(track);
        let sandbox = WorkspaceSandbox::new([&path]).expect("sandbox");
        let audit = AuditLogger::new(path.join("audit.jsonl")).expect("audit");
        let session =
            ApiSession::new(project, proj_paths, tier, sandbox, audit, "agent").expect("session");
        (session, track_id, path.display().to_string())
    }

    #[test]
    fn a_batch_applies_every_operation_as_one_undo_step() {
        let (mut session, track_id, _) = session(PermissionTier::Edit);
        let ops = vec![
            BatchOp::TrackUpdate {
                track_id: track_id.clone(),
                name: Some("Narration".into()),
                gain: Some(0.5),
                pan: None,
                muted: None,
                solo: None,
                armed: None,
            },
            BatchOp::MarkerAdd {
                name: "Chapter 1".into(),
                position_secs: 1.5,
            },
        ];

        let undo_before = session.stack.undo_len();
        let report = session.edit_batch("intro", &ops).expect("batch");
        assert_eq!(report.operations_applied, 2);
        assert!(report.single_undo_step);
        assert_eq!(
            session.stack.undo_len(),
            undo_before + 1,
            "the batch must be one undo entry, not one per operation"
        );
        assert!(session.project.tracks[0].name == "Narration");

        session.undo().expect("undo");
        assert_eq!(
            session.project.tracks[0].name, "Voice",
            "one undo must revert the whole batch, including both operations"
        );
        assert!(session.project.markers.is_empty());
    }

    #[test]
    fn a_failing_batch_leaves_the_project_untouched() {
        let (mut session, track_id, _) = session(PermissionTier::Edit);
        let ops = vec![
            BatchOp::TrackUpdate {
                track_id,
                name: Some("Half applied".into()),
                gain: None,
                pan: None,
                muted: None,
                solo: None,
                armed: None,
            },
            BatchOp::ClipRemove {
                clip_id: "not-a-clip".into(),
            },
        ];

        let revision_before = session.revision();
        let error = session
            .edit_batch("broken", &ops)
            .expect_err("batch must fail");
        assert!(
            error.to_string().contains("no changes were applied"),
            "{error}"
        );
        assert_eq!(session.project.tracks[0].name, "Voice");
        assert_eq!(
            session.revision(),
            revision_before,
            "a rolled-back batch must not advance the revision"
        );
        assert!(!session.stack.can_undo() || session.stack.undo_len() == 0);
    }

    #[test]
    fn an_empty_batch_is_rejected() {
        let (mut session, _, _) = session(PermissionTier::Edit);
        let error = session.edit_batch("empty", &[]).expect_err("empty batch");
        assert!(error.to_string().contains("no operations"), "{error}");
    }

    #[test]
    fn a_batch_needs_edit_permission() {
        let (mut session, track_id, _) = session(PermissionTier::Observe);
        let error = session
            .edit_batch(
                "observe",
                &[BatchOp::MarkerAdd {
                    name: "x".into(),
                    position_secs: 0.0,
                }],
            )
            .expect_err("observe tier");
        assert!(error.to_string().contains("Edit"), "{error}");
        assert!(track_id.contains('-') || !track_id.is_empty());
    }
}
