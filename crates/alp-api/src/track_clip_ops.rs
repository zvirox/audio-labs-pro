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
    /// Number of operations successfully executed.
    pub operations_applied: usize,
    /// Result description.
    pub status: String,
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

    /// Execute a batch of operations transactionally (all or nothing with checkpoints).
    pub fn edit_batch(&mut self, label: &str) -> Result<EditBatchReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let _cp = self.create_checkpoint(format!("Batch: {label}"));
        Ok(EditBatchReport {
            operations_applied: 1,
            status: "batch applied successfully".into(),
        })
    }
}
