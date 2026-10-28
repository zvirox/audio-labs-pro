//! Timeline, track, and clip editing operations for `EditSession`.

use std::collections::BTreeMap;

use alp_core::{
    AddMarker, ClipId, CrossfadeClips, DeleteRange, DuplicateClip, EnvelopePoint, FrameRange,
    Marker, MarkerId, NormalizeClip, PasteClips, RemoveClip, RemoveMarker, Selection,
    SetClipEnvelope, SetClipFades, SetClipGain, SetClipInverted, SetClipReversed, SetLoopRange,
    SetMasterGain, SetTrackArm, SetTrackGain, SetTrackMuted, SetTrackPan, SetTrackSolo,
    SilenceRange, SplitClip, Track, TrimClip, copy_range, frames_to_secs, secs_to_frames,
};

use super::{EditSession, NORMALIZE_TARGET, short_id};

impl EditSession {
    /// Track count in the project.
    #[must_use]
    pub fn track_count(&self) -> usize {
        self.project.tracks.len()
    }

    /// Read-only slice of all tracks.
    #[must_use]
    pub fn tracks(&self) -> &[Track] {
        &self.project.tracks
    }

    /// Current master gain.
    #[must_use]
    pub fn master_gain(&self) -> f32 {
        self.project.master_gain.clamp(0.0, 4.0)
    }

    /// Set master gain.
    pub fn set_master_gain(&mut self, gain: f32) {
        self.run(
            Box::new(SetMasterGain::new(gain.clamp(0.0, 4.0))),
            "Set master gain",
        );
    }

    /// Set track gain for the track at `track_index`.
    pub fn set_track_gain(&mut self, track_index: usize, gain: f32) {
        if let Some(track) = self.project.tracks.get(track_index) {
            let id = track.id;
            self.run(
                Box::new(SetTrackGain::new(id, gain.clamp(0.0, 2.0))),
                "Set track gain",
            );
        }
    }

    /// Set track pan for the track at `track_index`.
    pub fn set_track_pan(&mut self, track_index: usize, pan: f32) {
        if let Some(track) = self.project.tracks.get(track_index) {
            let id = track.id;
            self.run(
                Box::new(SetTrackPan::new(id, pan.clamp(-1.0, 1.0))),
                "Set track pan",
            );
        }
    }

    /// Toggle track mute state.
    pub fn toggle_track_mute(&mut self, track_index: usize) {
        if let Some(track) = self.project.tracks.get(track_index) {
            let id = track.id;
            self.run(
                Box::new(SetTrackMuted::new(id, !track.muted)),
                "Toggle mute",
            );
        }
    }

    /// Toggle track solo state.
    pub fn toggle_track_solo(&mut self, track_index: usize) {
        if let Some(track) = self.project.tracks.get(track_index) {
            let id = track.id;
            self.run(Box::new(SetTrackSolo::new(id, !track.solo)), "Toggle solo");
        }
    }

    /// Toggle track armed state for recording.
    pub fn toggle_track_arm(&mut self, track_index: usize) {
        if let Some(track) = self.project.tracks.get(track_index) {
            let id = track.id;
            self.run(Box::new(SetTrackArm::new(id, !track.armed)), "Toggle arm");
        }
    }

    /// Move the selected clip to the next track below it.
    pub fn move_selected_clip_to_next_track(&mut self) -> bool {
        let Some(id) = self.selected_clip else {
            self.status = "Select a clip first".into();
            return false;
        };
        let Some((track, clip)) = self.project.clip(id) else {
            return false;
        };
        let source_track_id = track.id;
        let Some(source_index) = self
            .project
            .tracks
            .iter()
            .position(|track| track.id == source_track_id)
        else {
            return false;
        };
        let target_index = source_index + 1;
        if target_index >= self.project.tracks.len() {
            self.status = "Selected clip is already on the last track".into();
            return false;
        }
        let target_track = self.project.tracks[target_index].id;
        self.run(
            Box::new(alp_core::MoveClip::new(
                id,
                target_track,
                0,
                clip.timeline_start,
            )),
            "Move clip",
        )
    }

    /// Move the selected clip to the previous track above it.
    pub fn move_selected_clip_to_prev_track(&mut self) -> bool {
        let Some(id) = self.selected_clip else {
            self.status = "Select a clip first".into();
            return false;
        };
        let Some((track, clip)) = self.project.clip(id) else {
            return false;
        };
        let source_track_id = track.id;
        let Some(source_index) = self
            .project
            .tracks
            .iter()
            .position(|track| track.id == source_track_id)
        else {
            return false;
        };
        if source_index == 0 {
            self.status = "Selected clip is already on the first track".into();
            return false;
        }
        let target_track = self.project.tracks[source_index - 1].id;
        self.run(
            Box::new(alp_core::MoveClip::new(
                id,
                target_track,
                0,
                clip.timeline_start,
            )),
            "Move clip",
        )
    }

    /// Crossfade the selected clip into its adjacent following clip on the same track.
    pub fn crossfade_selected(&mut self, overlap_seconds: f32) -> bool {
        let Some(clip_id) = self.selected_clip else {
            self.status = "Select a clip first".into();
            return false;
        };
        let Some(track) = self.project.track_of_clip(clip_id).cloned() else {
            return false;
        };
        let clips = track.clips.clone();
        let Some(index) = clips.iter().position(|clip| clip.id == clip_id) else {
            return false;
        };
        let Some(right) = clips.get(index + 1) else {
            self.status = "Selected clip needs a following clip".into();
            return false;
        };
        let left = &clips[index];
        let overlap = (overlap_seconds * self.project.sample_rate as f32)
            .round()
            .clamp(1.0, (left.len() as f32).min(right.len() as f32)) as u64;
        self.run(
            Box::new(CrossfadeClips::new(clip_id, right.id, overlap)),
            "Crossfade",
        )
    }

    /// Select a clip and make its timeline range the edit selection.
    pub fn select_clip(&mut self, id: ClipId) {
        let Some((_, clip)) = self.project.clip(id) else {
            self.status = "Clip is no longer present".into();
            return;
        };
        self.selected_clip = Some(id);
        let end = self.project.clip_timeline_end(clip);
        self.selection = Selection::between(self.track_id, clip.timeline_start, end, 1);
        self.status = format!("Selected clip {}", short_id(id));
    }

    /// Set the selection from seconds, clamped to the timeline.
    pub fn set_selection(&mut self, start: f64, end: f64) {
        let duration = self.duration_secs().max(0.001);
        let start = start.clamp(0.0, duration);
        let end = end.clamp(0.0, duration);
        self.selection = Selection::between(
            self.track_id,
            secs_to_frames(start, self.project.sample_rate),
            secs_to_frames(end, self.project.sample_rate),
            1,
        );
        self.status = format!(
            "Selection {:.2}s → {:.2}s",
            frames_to_secs(self.selection.range().start, self.project.sample_rate),
            frames_to_secs(self.selection.range().end, self.project.sample_rate)
        );
    }

    /// Copy the current selection.
    pub fn copy(&mut self) {
        match copy_range(&self.project, self.track_id, self.selection.range()) {
            Ok(clipboard) => {
                self.status = format!(
                    "Copied {} clip(s) · {:.2}s",
                    clipboard.len(),
                    frames_to_secs(clipboard.frames(), self.project.sample_rate)
                );
                self.clipboard = clipboard;
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    /// Cut the current selection: copy then delete with ripple.
    pub fn cut(&mut self) {
        self.copy();
        if self.clipboard.is_empty() {
            return;
        }
        let range = self.selection.range();
        self.run(
            Box::new(DeleteRange::new(self.track_id, range, true)),
            "Cut selection",
        );
    }

    /// Paste the clipboard at the selection start.
    pub fn paste(&mut self) {
        if self.clipboard.is_empty() {
            self.status = "Clipboard is empty".into();
            return;
        }
        let at = self.selection.range().start;
        self.run(
            Box::new(PasteClips::new(
                self.track_id,
                at,
                self.clipboard.fragments(),
            )),
            "Paste",
        );
        self.selected_clip = None;
    }

    /// Delete the selection, or the selected clip when the range is collapsed.
    pub fn delete(&mut self, ripple: bool) {
        if self.selection.is_empty()
            && let Some(id) = self.selected_clip
        {
            self.run(Box::new(RemoveClip::new(id)), "Delete clip");
            self.selected_clip = None;
            return;
        }
        let range = self.selection.range();
        self.run(
            Box::new(DeleteRange::new(self.track_id, range, ripple)),
            "Delete selection",
        );
    }

    /// Delete an explicit frame range on the active track (with or without ripple).
    pub fn delete_range(&mut self, range: FrameRange, ripple: bool) {
        self.run(
            Box::new(DeleteRange::new(self.track_id, range, ripple)),
            if ripple {
                "Ripple delete range"
            } else {
                "Delete range"
            },
        );
    }

    /// Replace the selection with silence, keeping later clips in place.
    pub fn silence(&mut self) {
        let range = self.selection.range();
        self.run(
            Box::new(SilenceRange::new(self.track_id, range)),
            "Silence selection",
        );
    }

    /// Lower the selected clip's envelope over the selection.
    pub fn dip_selection(&mut self) {
        let Some(id) = self.selected_clip else {
            self.status = "Select a clip first".into();
            return;
        };
        let Some(clip) = self.clip(id).cloned() else {
            self.status = "Selected clip no longer exists".into();
            return;
        };
        let range = self.selection.range();
        if range.is_empty() {
            self.status = "Select a range inside the clip".into();
            return;
        }
        let start = range.start.max(clip.timeline_start) - clip.timeline_start;
        let end = range.end.min(self.project.clip_timeline_end(&clip)) - clip.timeline_start;
        if end <= start {
            self.status = "Selection does not overlap the selected clip".into();
            return;
        }
        let len = clip.len();
        let mut points = BTreeMap::new();
        points.insert(0, 1.0);
        if start > 1 {
            points.insert(start - 1, 1.0);
        }
        points.insert(start, 0.15);
        points.insert(end.min(len), 0.15);
        if end < len {
            points.insert((end + 1).min(len), 1.0);
            points.insert(len, 1.0);
        }
        let envelope = points
            .into_iter()
            .map(|(frame, gain)| EnvelopePoint { frame, gain })
            .collect();
        if self.run(Box::new(SetClipEnvelope::new(id, envelope)), "Dip envelope") {
            self.status = format!(
                "Envelope dip {:.2}s",
                frames_to_secs(end - start, self.project.sample_rate)
            );
        }
    }

    /// Clear the selected clip's envelope.
    pub fn reset_envelope(&mut self) {
        let Some(id) = self.selected_clip else {
            self.status = "Select a clip first".into();
            return;
        };
        self.run(
            Box::new(SetClipEnvelope::new(id, Vec::new())),
            "Reset envelope",
        );
    }

    /// Trim the selected clip to the overlapping part of the selection.
    pub fn trim(&mut self) {
        let Some(id) = self.selected_clip else {
            self.status = "Select a clip first".into();
            return;
        };
        let Some(clip) = self.clip(id).cloned() else {
            self.status = "Selected clip no longer exists".into();
            return;
        };
        let range = self.selection.range();
        if range.is_empty() {
            self.status = "Select a range inside the clip".into();
            return;
        }
        let start = range.start.max(clip.timeline_start);
        let end = range.end.min(self.project.clip_timeline_end(&clip));
        if end <= start {
            self.status = "Selection does not overlap the selected clip".into();
            return;
        }
        let offset_start = start - clip.timeline_start;
        let offset_end = end - clip.timeline_start;
        let (source_start, source_end) = if clip.reversed {
            (
                clip.source_range.end - offset_end,
                clip.source_range.end - offset_start,
            )
        } else {
            (
                clip.source_range.start + offset_start,
                clip.source_range.start + offset_end,
            )
        };
        let Ok(source_range) = FrameRange::new(source_start, source_end) else {
            self.status = "Trim range is invalid".into();
            return;
        };
        self.run(
            Box::new(TrimClip::new(id, source_range, start)),
            "Trim clip",
        );
    }

    /// Split the selected clip at the selection cursor.
    pub fn split(&mut self) {
        let Some(id) = self.selected_clip else {
            self.status = "Select a clip first".into();
            return;
        };
        let Some(clip) = self.clip(id).cloned() else {
            self.status = "Selected clip no longer exists".into();
            return;
        };
        let cursor = self.selection.cursor();
        if cursor <= clip.timeline_start {
            self.status = "Place the cursor inside the selected clip".into();
            return;
        }
        let offset = cursor - clip.timeline_start;
        if offset >= clip.len() {
            self.status = "Cursor is past the selected clip".into();
            return;
        }
        let right = ClipId::new();
        if self.run(Box::new(SplitClip::new(id, offset, right)), "Split clip") {
            self.selected_clip = Some(right);
        }
    }

    /// Duplicate the selected clip immediately after it.
    pub fn duplicate(&mut self) {
        let Some(id) = self.selected_clip else {
            self.status = "Select a clip first".into();
            return;
        };
        let Some(clip) = self.clip(id).cloned() else {
            self.status = "Selected clip no longer exists".into();
            return;
        };
        let new_id = ClipId::new();
        let offset = self.project.clip_timeline_len(&clip);
        if self.run(
            Box::new(DuplicateClip::new(id, new_id, offset)),
            "Duplicate clip",
        ) {
            self.selected_clip = Some(new_id);
        }
    }

    /// Normalize the selected clip to -0.18 dBFS.
    pub fn normalize(&mut self) {
        let Some(id) = self.selected_clip else {
            self.status = "Select a clip first".into();
            return;
        };
        self.run(
            Box::new(NormalizeClip::new(id, NORMALIZE_TARGET)),
            "Normalize",
        );
    }

    /// Toggle reverse playback on the selected clip.
    pub fn toggle_reverse(&mut self) {
        let Some(id) = self.selected_clip else {
            self.status = "Select a clip first".into();
            return;
        };
        let reversed = self.clip(id).is_some_and(|clip| !clip.reversed);
        self.run(Box::new(SetClipReversed::new(id, reversed)), "Reverse clip");
    }

    /// Toggle phase inversion on the selected clip.
    pub fn toggle_invert(&mut self) {
        let Some(id) = self.selected_clip else {
            self.status = "Select a clip first".into();
            return;
        };
        let inverted = self.clip(id).is_some_and(|clip| !clip.inverted);
        self.run(Box::new(SetClipInverted::new(id, inverted)), "Invert clip");
    }

    /// Set selected clip gain from a `0.0..=2.0` control.
    pub fn set_gain(&mut self, gain: f32) {
        let Some(id) = self.selected_clip else {
            return;
        };
        self.run(
            Box::new(SetClipGain::new(id, gain.clamp(0.0, 2.0))),
            "Set gain",
        );
    }

    /// Set fades as fractions of the selected clip length.
    pub fn set_fades(&mut self, fade_in: f32, fade_out: f32) {
        let Some(id) = self.selected_clip else {
            return;
        };
        let Some(clip) = self.clip(id).cloned() else {
            return;
        };
        let len = clip.len();
        let fade_in = (fade_in.clamp(0.0, 1.0) as f64 * len as f64).round() as u64;
        let fade_out = (fade_out.clamp(0.0, 1.0) as f64 * len as f64).round() as u64;
        let fade_out = fade_out.min(len.saturating_sub(fade_in));
        self.run(
            Box::new(SetClipFades::new(id, fade_in, fade_out)),
            "Set fades",
        );
    }

    /// Set project loop range in seconds.
    #[allow(dead_code)]
    pub fn set_loop_range(&mut self, start_secs: f64, end_secs: f64) -> bool {
        let start = secs_to_frames(start_secs.max(0.0), self.project.sample_rate);
        let end = secs_to_frames(end_secs.max(0.0), self.project.sample_rate);
        if let Ok(range) = FrameRange::new(start, end) {
            self.run(Box::new(SetLoopRange::new(Some(range))), "Set loop range")
        } else {
            self.status = "Invalid loop range".into();
            false
        }
    }

    /// Clear the project loop range.
    #[allow(dead_code)]
    pub fn clear_loop_range(&mut self) -> bool {
        self.run(Box::new(SetLoopRange::new(None)), "Clear loop range")
    }

    /// Current project loop range.
    #[must_use]
    pub fn loop_range(&self) -> Option<FrameRange> {
        self.project.loop_range
    }

    /// Project markers.
    #[must_use]
    pub fn markers(&self) -> &[Marker] {
        &self.project.markers
    }

    /// Add a marker at `time_secs`.
    #[allow(dead_code)]
    pub fn add_marker(&mut self, time_secs: f64, label: impl Into<String>) -> bool {
        let frame = secs_to_frames(time_secs.max(0.0), self.project.sample_rate);
        let id = MarkerId::new();
        self.run(Box::new(AddMarker::new(id, frame, label)), "Add marker")
    }

    /// Remove a marker by id.
    #[allow(dead_code)]
    pub fn remove_marker(&mut self, id: MarkerId) -> bool {
        if self.project.markers.iter().any(|m| m.id == id) {
            self.run(Box::new(RemoveMarker::new(id)), "Remove marker")
        } else {
            false
        }
    }

    /// Undo the last command.
    pub fn undo(&mut self) {
        match self.stack.undo(&mut self.project) {
            Ok(true) => self.after_change("Undo"),
            Ok(false) => self.status = "Nothing to undo".into(),
            Err(error) => self.status = error.to_string(),
        }
    }

    /// Redo the next command.
    pub fn redo(&mut self) {
        match self.stack.redo(&mut self.project) {
            Ok(true) => self.after_change("Redo"),
            Ok(false) => self.status = "Nothing to redo".into(),
            Err(error) => self.status = error.to_string(),
        }
    }
}
