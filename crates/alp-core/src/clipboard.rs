//! Copying a selection into source-range fragments and pasting it back.
//!
//! A clipboard fragment is a non-destructive slice of a clip: it keeps the
//! source reference, gain, fades, envelope and the new Phase 3 reverse/invert
//! flags. It carries no clip identifier, so pasting can allocate fresh ids
//! without mutating the original timeline.

use crate::command::CommandError;
use crate::ids::{ClipId, SourceId, TrackId};
use crate::project::{Clip, EnvelopePoint, Project};
use crate::time::FrameRange;

/// One copied piece of audio, positioned relative to the copy selection start.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipFragment {
    /// Source media referenced by the fragment.
    pub source_id: SourceId,
    /// Region of the source, in source frames.
    pub source_range: FrameRange,
    /// Offset from the start of the copied selection, in project frames.
    pub offset: u64,
    /// Linear gain.
    pub gain: f32,
    /// Fade-in length in source frames.
    pub fade_in_frames: u64,
    /// Fade-out length in source frames.
    pub fade_out_frames: u64,
    /// Volume automation relative to the fragment start.
    pub envelope: Vec<EnvelopePoint>,
    /// Play the source range backwards.
    pub reversed: bool,
    /// Flip the sample sign.
    pub inverted: bool,
}

impl ClipFragment {
    /// Materialise this fragment as a new clip.
    #[must_use]
    pub fn to_clip(&self, id: ClipId, timeline_start: u64) -> Clip {
        let mut clip = Clip::new(id, self.source_id, self.source_range, timeline_start);
        clip.gain = self.gain;
        clip.fade_in_frames = self.fade_in_frames;
        clip.fade_out_frames = self.fade_out_frames;
        clip.envelope = self.envelope.clone();
        clip.reversed = self.reversed;
        clip.inverted = self.inverted;
        clip
    }

    /// Length in source frames.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.source_range.len()
    }

    /// Whether the fragment has no frames.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.source_range.is_empty()
    }
}

/// A set of fragments copied from one track range.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Clipboard {
    fragments: Vec<ClipFragment>,
    frames: u64,
}

impl Clipboard {
    /// Build a clipboard from already sliced fragments.
    ///
    /// `frames` is the length of the original selection, including gaps that
    /// contained no clips.
    #[must_use]
    pub fn from_fragments(fragments: Vec<ClipFragment>, frames: u64) -> Self {
        Self { fragments, frames }
    }

    /// Fragments in timeline order.
    #[must_use]
    pub fn fragments(&self) -> &[ClipFragment] {
        &self.fragments
    }

    /// Length of the copied selection in project frames.
    #[must_use]
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Whether no audio was copied.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty() || self.frames == 0
    }

    /// Number of fragments.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fragments.len()
    }
}

/// Copy `range` from one track into a clipboard.
///
/// Clips that only overlap the range are sliced. Gaps in the selection are
/// preserved through the selection length, so pasting at another position keeps
/// relative timing intact.
pub fn copy_range(
    project: &Project,
    track_id: TrackId,
    range: FrameRange,
) -> Result<Clipboard, CommandError> {
    if range.is_empty() {
        return Err(CommandError::Invalid(
            "copy range must contain at least one frame".into(),
        ));
    }
    let track = project
        .track(track_id)
        .ok_or(CommandError::TrackNotFound(track_id))?;

    let mut fragments = Vec::new();
    for clip in &track.clips {
        let clip_end = project.clip_timeline_end(clip);
        let from = clip.timeline_start.max(range.start);
        let to = clip_end.min(range.end);
        if from >= to {
            continue;
        }
        let sliced = slice_clip(project, clip, from, to, clip.id).ok_or_else(|| {
            CommandError::Invalid("clip disappeared while copying the selection".into())
        })?;
        fragments.push(ClipFragment {
            source_id: sliced.source_id,
            source_range: sliced.source_range,
            offset: sliced.timeline_start.saturating_sub(range.start),
            gain: sliced.gain,
            fade_in_frames: sliced.fade_in_frames,
            fade_out_frames: sliced.fade_out_frames,
            envelope: sliced.envelope,
            reversed: sliced.reversed,
            inverted: sliced.inverted,
        });
    }
    fragments.sort_by_key(|fragment| fragment.offset);
    Ok(Clipboard::from_fragments(fragments, range.len()))
}

/// Slice a clip to an absolute project-timeline range.
///
/// Fades are kept only when the slice touches the corresponding end of the
/// original clip. Envelope points are clipped and rebased to the new start.
pub(crate) fn slice_clip(
    project: &Project,
    clip: &Clip,
    from: u64,
    to: u64,
    id: ClipId,
) -> Option<Clip> {
    let clip_end = project.clip_timeline_end(clip);
    let from = from.max(clip.timeline_start);
    let to = to.min(clip_end);
    if from >= to {
        return None;
    }

    let timeline_len = project.clip_timeline_len(clip);
    let source_len = clip.len();
    let start_offset = source_offset(from - clip.timeline_start, timeline_len, source_len);
    let end_offset = source_offset(to - clip.timeline_start, timeline_len, source_len);
    let source_start = clip.source_range.start.saturating_add(start_offset);
    let source_end = clip
        .source_range
        .start
        .saturating_add(end_offset)
        .min(clip.source_range.end);
    if source_end <= source_start {
        return None;
    }

    let mut sliced = clip.clone();
    sliced.id = id;
    sliced.source_range = FrameRange {
        start: source_start,
        end: source_end,
    };
    sliced.timeline_start = from;
    let len = sliced.len();
    sliced.fade_in_frames = if from == clip.timeline_start {
        clip.fade_in_frames.min(len)
    } else {
        0
    };
    sliced.fade_out_frames = if to == clip_end {
        clip.fade_out_frames.min(len - sliced.fade_in_frames)
    } else {
        0
    };
    sliced.envelope = clip
        .envelope
        .iter()
        .filter(|point| point.frame >= start_offset && point.frame <= end_offset)
        .map(|point| EnvelopePoint {
            frame: point.frame - start_offset,
            gain: point.gain,
        })
        .collect();
    Some(sliced)
}

fn source_offset(timeline_offset: u64, timeline_len: u64, source_len: u64) -> u64 {
    if timeline_len == 0 {
        return 0;
    }
    let offset = timeline_offset.min(timeline_len);
    ((offset as f64 / timeline_len as f64) * source_len as f64)
        .round()
        .clamp(0.0, source_len as f64) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClipId, Project, Source, Track, TrackId};

    fn fixture() -> (Project, TrackId) {
        let mut project = Project::new("copy").expect("project");
        project.sources.push(
            Source::new(
                SourceId::from_u128(1),
                "voice.wav",
                "media/voice.wav",
                48_000,
                1,
                48_000,
            )
            .expect("source"),
        );
        let mut track = Track::new(TrackId::from_u128(2), "Voice");
        let mut clip = Clip::new(
            ClipId::from_u128(3),
            SourceId::from_u128(1),
            FrameRange::new(1_000, 9_000).expect("range"),
            2_000,
        );
        clip.gain = 0.5;
        clip.fade_in_frames = 100;
        clip.fade_out_frames = 200;
        clip.envelope = vec![
            EnvelopePoint {
                frame: 0,
                gain: 0.0,
            },
            EnvelopePoint {
                frame: 1_000,
                gain: 1.0,
            },
        ];
        track.clips.push(clip);
        project.tracks.push(track);
        (project, TrackId::from_u128(2))
    }

    #[test]
    fn copies_only_the_selection_and_rebases_offsets() {
        let (project, track) = fixture();
        let clipboard = copy_range(
            &project,
            track,
            FrameRange::new(2_500, 5_000).expect("range"),
        )
        .expect("copy");
        assert_eq!(clipboard.frames(), 2_500);
        assert_eq!(clipboard.len(), 1);
        let fragment = &clipboard.fragments()[0];
        assert_eq!(fragment.offset, 0);
        assert_eq!(
            fragment.source_range,
            FrameRange::new(1_500, 4_000).expect("range")
        );
        assert_eq!(fragment.gain, 0.5);
        assert_eq!(fragment.fade_in_frames, 0);
        assert_eq!(fragment.fade_out_frames, 0);
        assert_eq!(fragment.envelope.len(), 1);
        assert_eq!(fragment.envelope[0].frame, 500);
    }

    #[test]
    fn preserves_full_clip_fades() {
        let (project, track) = fixture();
        let clipboard =
            copy_range(&project, track, FrameRange::new(0, 20_000).expect("range")).expect("copy");
        let fragment = &clipboard.fragments()[0];
        assert_eq!(fragment.offset, 2_000);
        assert_eq!(fragment.fade_in_frames, 100);
        assert_eq!(fragment.fade_out_frames, 200);
    }

    #[test]
    fn rejects_empty_ranges_and_missing_tracks() {
        let (project, track) = fixture();
        assert!(copy_range(&project, track, FrameRange::new(10, 10).expect("range")).is_err());
        assert!(
            copy_range(
                &project,
                TrackId::from_u128(99),
                FrameRange::new(0, 10).expect("range"),
            )
            .is_err()
        );
    }
}
