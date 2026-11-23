//! Timeline, track, and clip editing operations for `EditSession`.

use std::collections::BTreeMap;

use alp_core::{
    AddMarker, AddTrack, ClipId, CrossfadeClips, DeleteRange, DuplicateClip, EnvelopePoint,
    FrameRange, Marker, MarkerId, MoveClip, NormalizeClip, PasteClips, RemoveClip, RemoveMarker,
    Selection, SetClipEnvelope, SetClipFades, SetClipGain, SetClipInverted, SetClipReversed,
    SetLoopRange, SetMasterGain, SetTrackArm, SetTrackGain, SetTrackMuted, SetTrackPan, SetTrackSolo,
    SilenceRange, SplitClip, Track, TrackId, TrimClip, copy_range, frames_to_secs, secs_to_frames,
};
use alp_dsp::{SpectrogramConfig, SpectrogramData, SpectrogramEngine};

use crate::timeline::{SpectralSelectionDto, TimelinePanPointDto, TrackViewMode};

use super::{EditSession, NORMALIZE_TARGET, clear_attached_context, short_id};

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

    /// Append a track through the same undo stack used by human edits.
    pub fn add_track(&mut self, name: &str) -> Result<alp_api::TrackSummary, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 128 {
            return Err("track name must contain 1-128 characters".into());
        }
        let id = TrackId::new();
        let index = self.project.tracks.len();
        let track = Track::new(id, name);
        if !self.run(Box::new(AddTrack::new(track, index)), "AI add track") {
            return Err(self.status.clone());
        }
        let track = &self.project.tracks[index];
        Ok(alp_api::TrackSummary {
            id: track.id.to_string(),
            name: track.name.clone(),
            index,
            gain: track.gain,
            pan: track.pan,
            muted: track.muted,
            solo: track.solo,
            armed: track.armed,
            clip_count: track.clips.len(),
        })
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

    /// Split an addressed clip for an attached automation client.
    pub fn split_clip_at(&mut self, clip_id: &str, split_secs: f64) -> Result<(), String> {
        let id: ClipId = clip_id
            .parse()
            .map_err(|error| format!("invalid clip id: {error}"))?;
        let (_, clip) = self
            .project
            .clip(id)
            .ok_or_else(|| format!("clip '{clip_id}' not found"))?;
        let rate = self.project.sample_rate;
        let split_frame = secs_to_frames(split_secs, rate);
        if !split_secs.is_finite()
            || split_frame <= clip.timeline_start
            || split_frame >= clip.timeline_end()
        {
            return Err(format!(
                "split position must be inside [{:.6}s, {:.6}s]",
                frames_to_secs(clip.timeline_start, rate),
                frames_to_secs(clip.timeline_end(), rate)
            ));
        }
        let offset = split_frame - clip.timeline_start;
        let right = ClipId::new();
        if !self.run(Box::new(SplitClip::new(id, offset, right)), "AI split clip") {
            return Err(self.status.clone());
        }
        self.selected_clip = Some(right);
        Ok(())
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
        clear_attached_context(&self.attached_context);
        match self.stack.undo(&mut self.project) {
            Ok(true) => self.after_change("Undo"),
            Ok(false) => self.status = "Nothing to undo".into(),
            Err(error) => self.status = error.to_string(),
        }
    }

    /// Redo the next command.
    pub fn redo(&mut self) {
        clear_attached_context(&self.attached_context);
        match self.stack.redo(&mut self.project) {
            Ok(true) => self.after_change("Redo"),
            Ok(false) => self.status = "Nothing to redo".into(),
            Err(error) => self.status = error.to_string(),
        }
    }

    /// Move a clip to a target track and start position in seconds.
    pub fn move_clip_by_id(
        &mut self,
        clip_id_str: &str,
        target_track_id_str: &str,
        new_start_secs: f64,
    ) -> Result<(), String> {
        let clip_id: ClipId = clip_id_str
            .parse()
            .map_err(|e| format!("invalid clip id: {e}"))?;
        let target_track_id: TrackId = target_track_id_str
            .parse()
            .map_err(|e| format!("invalid track id: {e}"))?;

        let rate = self.project.sample_rate;
        let timeline_start = secs_to_frames(new_start_secs.max(0.0), rate);

        let target_track = self
            .project
            .track(target_track_id)
            .ok_or_else(|| format!("target track '{target_track_id_str}' not found"))?;

        let target_index = target_track
            .clips
            .iter()
            .filter(|c| c.id != clip_id)
            .position(|c| c.timeline_start > timeline_start)
            .unwrap_or_else(|| {
                target_track
                    .clips
                    .iter()
                    .filter(|c| c.id != clip_id)
                    .count()
            });

        if self.run(
            Box::new(MoveClip::new(
                clip_id,
                target_track_id,
                target_index,
                timeline_start,
            )),
            "Move clip",
        ) {
            self.selected_clip = Some(clip_id);
            Ok(())
        } else {
            Err(self.status.clone())
        }
    }

    /// Trim clip boundaries non-destructively within source constraints.
    pub fn trim_clip_by_id(
        &mut self,
        clip_id_str: &str,
        new_start_secs: f64,
        new_duration_secs: f64,
    ) -> Result<(), String> {
        let clip_id: ClipId = clip_id_str
            .parse()
            .map_err(|e| format!("invalid clip id: {e}"))?;

        let (_, clip) = self
            .project
            .clip(clip_id)
            .ok_or_else(|| format!("clip '{clip_id_str}' not found"))?;

        let rate = self.project.sample_rate;
        let source_id = clip.source_id;
        let source = self
            .project
            .source(source_id)
            .ok_or_else(|| format!("source '{source_id}' not found"))?;

        let new_start_frame = secs_to_frames(new_start_secs.max(0.0), rate);
        let new_len_frames = secs_to_frames(new_duration_secs.max(0.01), rate).max(1);

        let start_delta = new_start_frame as i64 - clip.timeline_start as i64;
        let new_source_start =
            (clip.source_range.start as i64 + start_delta).clamp(0, source.frames as i64) as u64;
        let new_source_end = (new_source_start + new_len_frames).min(source.frames);

        if new_source_end <= new_source_start {
            return Err("trim results in empty or invalid range".into());
        }

        let new_range = FrameRange::new(new_source_start, new_source_end)
            .map_err(|e| format!("invalid frame range: {e}"))?;

        if self.run(
            Box::new(TrimClip::new(clip_id, new_range, new_start_frame)),
            "Trim clip",
        ) {
            self.selected_clip = Some(clip_id);
            Ok(())
        } else {
            Err(self.status.clone())
        }
    }

    /// Set linear fade-in and fade-out durations in seconds.
    pub fn set_clip_fades_by_id(
        &mut self,
        clip_id_str: &str,
        fade_in_secs: f64,
        fade_out_secs: f64,
    ) -> Result<(), String> {
        let clip_id: ClipId = clip_id_str
            .parse()
            .map_err(|e| format!("invalid clip id: {e}"))?;

        let (_, clip) = self
            .project
            .clip(clip_id)
            .ok_or_else(|| format!("clip '{clip_id_str}' not found"))?;

        let rate = self.project.sample_rate;
        let fade_in = secs_to_frames(fade_in_secs.max(0.0), rate);
        let fade_out = secs_to_frames(fade_out_secs.max(0.0), rate);

        let max_fades = clip.len();
        let total = fade_in.saturating_add(fade_out);
        let (fi, fo) = if total > max_fades && total > 0 {
            let scale = max_fades as f64 / total as f64;
            ((fade_in as f64 * scale) as u64, (fade_out as f64 * scale) as u64)
        } else {
            (fade_in, fade_out)
        };

        if self.run(Box::new(SetClipFades::new(clip_id, fi, fo)), "Set clip fades") {
            Ok(())
        } else {
            Err(self.status.clone())
        }
    }

    /// Set volume envelope control points for a clip.
    pub fn set_clip_envelope_by_id(
        &mut self,
        clip_id_str: &str,
        points: Vec<(f64, f32)>,
    ) -> Result<(), String> {
        let clip_id: ClipId = clip_id_str
            .parse()
            .map_err(|e| format!("invalid clip id: {e}"))?;

        let (_, clip) = self
            .project
            .clip(clip_id)
            .ok_or_else(|| format!("clip '{clip_id_str}' not found"))?;

        let rate = self.project.sample_rate;
        let mut envelope = Vec::new();
        for (offset_secs, gain) in points {
            let frame = secs_to_frames(offset_secs.max(0.0), rate).min(clip.len());
            envelope.push(EnvelopePoint {
                frame,
                gain: gain.max(0.0),
            });
        }
        envelope.sort_by_key(|p| p.frame);
        envelope.dedup_by_key(|p| p.frame);

        if self.run(
            Box::new(SetClipEnvelope::new(clip_id, envelope)),
            "Set clip envelope",
        ) {
            Ok(())
        } else {
            Err(self.status.clone())
        }
    }

    /// Retrieve full multitrack timeline serialization DTO.
    #[must_use]
    pub fn timeline_dto(&self) -> crate::timeline::TimelineDto {
        let rate = self.project.sample_rate;
        let tracks = self
            .project
            .tracks
            .iter()
            .map(|track| {
                let clips = track
                    .clips
                    .iter()
                    .map(|clip| {
                        let source_name = self
                            .project
                            .source(clip.source_id)
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| {
                                let id_str = clip.id.to_string();
                                format!("clip-{}", &id_str[..id_str.len().min(8)])
                            });

                        let pan_envelope = self
                            .clip_pan_envelopes
                            .get(&clip.id)
                            .cloned()
                            .unwrap_or_default();
                        let slip_offset_secs =
                            self.clip_slip_offsets.get(&clip.id).copied().unwrap_or(0.0);

                        crate::timeline::TimelineClipDto {
                            id: clip.id.to_string(),
                            source_id: clip.source_id.to_string(),
                            name: source_name,
                            timeline_start_secs: frames_to_secs(clip.timeline_start, rate),
                            duration_secs: frames_to_secs(clip.len(), rate),
                            source_start_secs: frames_to_secs(clip.source_range.start, rate),
                            gain: clip.gain,
                            fade_in_secs: frames_to_secs(clip.fade_in_frames, rate),
                            fade_out_secs: frames_to_secs(clip.fade_out_frames, rate),
                            slip_offset_secs,
                            envelope: clip
                                .envelope
                                .iter()
                                .map(|p| crate::timeline::TimelineEnvelopePointDto {
                                    offset_secs: frames_to_secs(p.frame, rate),
                                    gain: p.gain,
                                })
                                .collect(),
                            pan_envelope,
                        }
                    })
                    .collect();

                let view_mode = self
                    .track_view_modes
                    .get(&track.id)
                    .copied()
                    .unwrap_or_default();
                let spectral_selection = self.track_spectral_selections.get(&track.id).cloned();

                crate::timeline::TimelineTrackDto {
                    id: track.id.to_string(),
                    name: track.name.clone(),
                    gain: track.gain,
                    pan: track.pan,
                    muted: track.muted,
                    solo: track.solo,
                    armed: track.armed,
                    view_mode,
                    spectral_selection,
                    clips,
                }
            })
            .collect();

        let markers = self
            .project
            .markers
            .iter()
            .map(|m| crate::timeline::TimelineMarkerDto {
                id: m.id.to_string(),
                time_secs: frames_to_secs(m.frame, rate),
                label: m.label.clone(),
            })
            .collect();

        let loop_range = self.project.loop_range.map(|r| crate::timeline::TimelineLoopRangeDto {
            start_secs: frames_to_secs(r.start, rate),
            end_secs: frames_to_secs(r.end, rate),
        });

        let selection = if self.selection.is_empty() {
            None
        } else {
            Some(crate::timeline::TimelineSelectionDto {
                track_id: Some(self.selection.track_id().to_string()),
                start_secs: frames_to_secs(self.selection.range().start, rate),
                end_secs: frames_to_secs(self.selection.range().end, rate),
            })
        };

        crate::timeline::TimelineDto {
            sample_rate: rate,
            duration_secs: self.duration_secs(),
            tracks,
            markers,
            loop_range,
            selection,
            selected_clip_id: self.selected_clip.map(|id| id.to_string()),
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            undo_steps: self.undo_steps(),
            redo_steps: self.redo_steps(),
            revision: self.revision(),
        }
    }

    /// Set visualization mode for a track by ID.
    pub fn set_track_view_mode_by_id(&mut self, track_id_str: &str, view_mode_str: &str) -> Result<(), String> {
        let track_id: TrackId = track_id_str
            .parse()
            .map_err(|e| format!("invalid track id: {e}"))?;

        if !self.project.tracks.iter().any(|t| t.id == track_id) {
            return Err(format!("track '{track_id_str}' not found"));
        }

        let mode = match view_mode_str.to_lowercase().as_str() {
            "waveform" => TrackViewMode::Waveform,
            "spectrogram" => TrackViewMode::Spectrogram,
            "split" => TrackViewMode::Split,
            other => return Err(format!("unknown view mode '{other}' (expected 'waveform', 'spectrogram', or 'split')")),
        };

        self.track_view_modes.insert(track_id, mode);
        self.bump_revision();
        Ok(())
    }

    /// Set 2D time-frequency spectral selection for a track by ID.
    pub fn set_spectral_selection_by_id(
        &mut self,
        track_id_str: &str,
        selection: Option<SpectralSelectionDto>,
    ) -> Result<(), String> {
        let track_id: TrackId = track_id_str
            .parse()
            .map_err(|e| format!("invalid track id: {e}"))?;

        if !self.project.tracks.iter().any(|t| t.id == track_id) {
            return Err(format!("track '{track_id_str}' not found"));
        }

        if let Some(sel) = selection {
            self.track_spectral_selections.insert(track_id, sel);
        } else {
            self.track_spectral_selections.remove(&track_id);
        }
        self.bump_revision();
        Ok(())
    }

    /// Set pan automation envelope for a clip by ID.
    pub fn set_clip_pan_envelope_by_id(
        &mut self,
        clip_id_str: &str,
        points: Vec<(f64, f32)>,
    ) -> Result<(), String> {
        let clip_id: ClipId = clip_id_str
            .parse()
            .map_err(|e| format!("invalid clip id: {e}"))?;

        let (_, clip) = self
            .project
            .clip(clip_id)
            .ok_or_else(|| format!("clip '{clip_id_str}' not found"))?;

        let max_duration = frames_to_secs(clip.len(), self.project.sample_rate);
        let mut pan_points = Vec::new();
        for (offset_secs, pan) in points {
            let offset = offset_secs.clamp(0.0, max_duration);
            let clamped_pan = pan.clamp(-1.0, 1.0);
            pan_points.push(TimelinePanPointDto {
                offset_secs: offset,
                pan: clamped_pan,
            });
        }
        pan_points.sort_by(|a, b| a.offset_secs.total_cmp(&b.offset_secs));
        pan_points.dedup_by(|a, b| (a.offset_secs - b.offset_secs).abs() < 1e-5);

        self.clip_pan_envelopes.insert(clip_id, pan_points);
        self.bump_revision();
        Ok(())
    }

    /// Slip edit a clip: adjusts source audio offset by delta_secs while keeping timeline position fixed.
    pub fn slip_clip_by_id(&mut self, clip_id_str: &str, delta_secs: f64) -> Result<(), String> {
        let clip_id: ClipId = clip_id_str
            .parse()
            .map_err(|e| format!("invalid clip id: {e}"))?;

        let (_, clip) = self
            .project
            .clip(clip_id)
            .ok_or_else(|| format!("clip '{clip_id_str}' not found"))?;

        let source = self
            .project
            .source(clip.source_id)
            .ok_or_else(|| format!("source '{}' not found", clip.source_id))?;

        let max_source_secs = frames_to_secs(source.frames, self.project.sample_rate);
        let clip_len_secs = frames_to_secs(clip.len(), self.project.sample_rate);
        let max_slip = (max_source_secs - clip_len_secs).max(0.0);

        let current_offset = self.clip_slip_offsets.get(&clip_id).copied().unwrap_or(0.0);
        let new_offset = (current_offset + delta_secs).clamp(0.0, max_slip);

        self.clip_slip_offsets.insert(clip_id, new_offset);
        self.bump_revision();
        Ok(())
    }

    /// Compute high-resolution Short-Time Fourier Transform (STFT) spectrogram for a clip.
    pub fn compute_clip_spectrogram_by_id(
        &self,
        clip_id_str: &str,
        config: Option<SpectrogramConfig>,
    ) -> Result<SpectrogramData, String> {
        let clip_id: ClipId = clip_id_str
            .parse()
            .map_err(|e| format!("invalid clip id: {e}"))?;

        let (_, clip) = self
            .project
            .clip(clip_id)
            .ok_or_else(|| format!("clip '{clip_id_str}' not found"))?;

        let source_audio = self.audio.get(&clip.source_id).ok_or_else(|| {
            format!("audio source {} not found for clip {}", clip.source_id, clip_id_str)
        })?;

        let sample_rate = source_audio.sample_rate();
        let channels = source_audio.channels() as usize;
        let all_samples = source_audio.samples();

        let slip_offset = self.clip_slip_offsets.get(&clip_id).copied().unwrap_or(0.0);
        let slip_frames = (slip_offset * sample_rate as f64).round() as i64;
        let start_frame = ((clip.source_range.start as i64 + slip_frames).max(0)) as usize;
        let duration_frames = clip.len() as usize;
        let end_frame = (start_frame + duration_frames).min(source_audio.frames() as usize);

        if start_frame >= end_frame {
            return Err("clip has empty or out-of-bounds source range".into());
        }

        let actual_frames = end_frame - start_frame;
        let mut mono_samples = Vec::with_capacity(actual_frames);

        if channels <= 1 {
            mono_samples.extend_from_slice(&all_samples[start_frame..end_frame]);
        } else {
            for f in start_frame..end_frame {
                let mut sum = 0.0f32;
                for ch in 0..channels {
                    sum += all_samples[f * channels + ch];
                }
                mono_samples.push(sum / channels as f32);
            }
        }

        let cfg = config.unwrap_or_default();
        let engine = SpectrogramEngine::new(cfg).map_err(|e| e.to_string())?;
        engine.compute(&mono_samples, sample_rate).map_err(|e| e.to_string())
    }
}
