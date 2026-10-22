//! Property tests for the undo/redo contract.
//!
//! A random sequence of valid edits is applied to a project. Undoing everything
//! must reproduce the initial JSON byte-for-byte; redoing everything must
//! reproduce the edited JSON byte-for-byte.

use alp_core::{
    AddClip, AddTrack, Clip, ClipId, CommandStack, FrameRange, MoveClip, Project, ProjectId,
    RemoveClip, RemoveTrack, RenameTrack, SetClipFades, SetClipGain, SetTrackGain, SetTrackMuted,
    SetTrackPan, SetTrackSolo, Source, SourceId, SplitClip, Track, TrackId, TrimClip,
};
use proptest::prelude::*;

const SOURCE_ID_U128: u128 = 900;

fn fixture() -> Project {
    let mut project =
        Project::new_with_id(ProjectId::from_u128(1), "property", 48_000).expect("project");
    project.sources.push(
        Source::new(
            SourceId::from_u128(SOURCE_ID_U128),
            "fixture.wav",
            "media/fixture.wav",
            48_000,
            1,
            480_000,
        )
        .expect("source"),
    );
    project
}

fn json(project: &Project) -> String {
    serde_json::to_string(project).expect("serialize")
}

fn commit(
    project: &mut Project,
    stack: &mut CommandStack,
    command: Box<dyn alp_core::Command>,
) -> Result<(), String> {
    stack
        .execute(project, command)
        .map_err(|error| error.to_string())
}

fn next_id(next: &mut u128) -> u128 {
    *next += 1;
    *next
}

fn apply_op(
    project: &mut Project,
    stack: &mut CommandStack,
    next: &mut u128,
    op: u8,
) -> Result<(), String> {
    match op % 16 {
        0..=2 => {
            let id = TrackId::from_u128(next_id(next));
            let name = format!("Track {id}");
            commit(
                project,
                stack,
                Box::new(AddTrack::new(Track::new(id, name), project.tracks.len())),
            )
        }
        3 => {
            if project.tracks.is_empty() {
                return Ok(());
            }
            let track = &project.tracks[usize::from(op) % project.tracks.len()];
            let id = track.id;
            commit(
                project,
                stack,
                Box::new(RenameTrack::new(id, format!("Renamed {op}"))),
            )
        }
        4 => {
            if project.tracks.is_empty() {
                return Ok(());
            }
            let id = project.tracks[usize::from(op) % project.tracks.len()].id;
            let gain = f32::from(op) / 64.0;
            commit(project, stack, Box::new(SetTrackGain::new(id, gain)))
        }
        5 => {
            if project.tracks.is_empty() {
                return Ok(());
            }
            let id = project.tracks[usize::from(op) % project.tracks.len()].id;
            let pan = (f32::from(op) / 127.5) - 1.0;
            commit(project, stack, Box::new(SetTrackPan::new(id, pan)))
        }
        6 => {
            if project.tracks.is_empty() {
                return Ok(());
            }
            let id = project.tracks[usize::from(op) % project.tracks.len()].id;
            commit(
                project,
                stack,
                Box::new(SetTrackMuted::new(id, op.is_multiple_of(2))),
            )
        }
        7 => {
            if project.tracks.is_empty() {
                return Ok(());
            }
            let id = project.tracks[usize::from(op) % project.tracks.len()].id;
            commit(project, stack, Box::new(SetTrackSolo::new(id, op % 2 == 1)))
        }
        8..=9 => {
            if project.tracks.is_empty() {
                return Ok(());
            }
            let track_index = usize::from(op) % project.tracks.len();
            let track_id = project.tracks[track_index].id;
            let clip_id = ClipId::from_u128(next_id(next));
            let start = u64::from(op) * 997 % 400_000;
            let len = 1 + u64::from(op) * 131 % 80_000;
            let range = FrameRange::new(start, start + len).expect("range");
            let timeline_start = u64::from(op) * 7_919 % 1_000_000;
            let clip = Clip::new(
                clip_id,
                SourceId::from_u128(SOURCE_ID_U128),
                range,
                timeline_start,
            );
            let index = project.tracks[track_index].clips.len();
            commit(
                project,
                stack,
                Box::new(AddClip::new(track_id, clip, index)),
            )
        }
        10 => {
            let Some(clip) = project
                .tracks
                .iter()
                .flat_map(|track| track.clips.iter())
                .next()
            else {
                return Ok(());
            };
            let id = clip.id;
            commit(
                project,
                stack,
                Box::new(SetClipGain::new(id, f32::from(op) / 64.0)),
            )
        }
        11 => {
            let Some(clip) = project
                .tracks
                .iter()
                .flat_map(|track| track.clips.iter())
                .next()
            else {
                return Ok(());
            };
            let id = clip.id;
            let len = clip.len();
            let fade_in = u64::from(op) % (len / 3 + 1);
            let fade_out = u64::from(op) % (len / 3 + 1);
            let fade_out = fade_out.min(len - fade_in);
            commit(
                project,
                stack,
                Box::new(SetClipFades::new(id, fade_in, fade_out)),
            )
        }
        12 => {
            let Some(clip) = project
                .tracks
                .iter()
                .flat_map(|track| track.clips.iter())
                .next()
            else {
                return Ok(());
            };
            let id = clip.id;
            let len = clip.len();
            let start = clip.source_range.start;
            let new_len = 1 + u64::from(op) * 37 % len;
            let range = FrameRange::new(start, start + new_len).expect("range");
            commit(
                project,
                stack,
                Box::new(TrimClip::new(id, range, clip.timeline_start)),
            )
        }
        13 => {
            let Some(clip) = project
                .tracks
                .iter()
                .flat_map(|track| track.clips.iter())
                .next()
            else {
                return Ok(());
            };
            let (clip_id, len) = (clip.id, clip.len());
            if len < 2 {
                return Ok(());
            }
            let right_id = ClipId::from_u128(next_id(next));
            commit(
                project,
                stack,
                Box::new(SplitClip::new(clip_id, len / 2, right_id)),
            )
        }
        14 => {
            let Some(clip) = project
                .tracks
                .iter()
                .flat_map(|track| track.clips.iter())
                .next()
            else {
                return Ok(());
            };
            let clip_id = clip.id;
            let timeline_start = clip.timeline_start + 1_000;
            let target_track = if project.tracks.len() > 1 {
                project.tracks[1].id
            } else {
                project.tracks[0].id
            };
            commit(
                project,
                stack,
                Box::new(MoveClip::new(clip_id, target_track, 0, timeline_start)),
            )
        }
        _ => {
            if let Some(clip) = project
                .tracks
                .iter()
                .flat_map(|track| track.clips.iter())
                .next()
            {
                let id = clip.id;
                return commit(project, stack, Box::new(RemoveClip::new(id)));
            }
            if !project.tracks.is_empty() {
                let id = project.tracks[0].id;
                return commit(project, stack, Box::new(RemoveTrack::new(id)));
            }
            Ok(())
        }
    }
}

fn run(ops: &[u8]) -> Result<(), String> {
    let mut project = fixture();
    let initial = json(&project);
    let mut stack = CommandStack::with_limit(1_000);
    let mut next = 100;

    for op in ops {
        apply_op(&mut project, &mut stack, &mut next, *op)?;
        project.validate().map_err(|error| error.to_string())?;
    }

    let edited = json(&project);
    let undo_count = stack.undo_len();
    for _ in 0..undo_count {
        if !stack
            .undo(&mut project)
            .map_err(|error| error.to_string())?
        {
            return Err("undo stack ended early".into());
        }
        project.validate().map_err(|error| error.to_string())?;
    }
    if json(&project) != initial {
        return Err("undo did not restore the initial project".into());
    }
    if stack.can_undo() {
        return Err("undo stack not empty after full undo".into());
    }

    let redo_count = stack.redo_len();
    for _ in 0..redo_count {
        if !stack
            .redo(&mut project)
            .map_err(|error| error.to_string())?
        {
            return Err("redo stack ended early".into());
        }
        project.validate().map_err(|error| error.to_string())?;
    }
    if json(&project) != edited {
        return Err("redo did not restore the edited project".into());
    }
    Ok(())
}

proptest! {
    #[test]
    fn undo_redo_round_trip(ops in prop::collection::vec(any::<u8>(), 0..160)) {
        run(&ops).expect("random edit sequence must round-trip");
    }
}

#[test]
fn empty_sequence_is_a_noop() {
    run(&[]).expect("noop");
}
