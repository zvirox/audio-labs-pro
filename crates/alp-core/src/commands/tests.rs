use super::*;
use crate::{
    Clip, CommandStack, CrossfadeClips, FrameRange, MarkerId, Project, ProjectId, RegionId, Source,
    SourceId, Track, TrackId,
};

fn fixture() -> (Project, TrackId, SourceId) {
    let mut project =
        Project::new_with_id(ProjectId::from_u128(1), "test", 48_000).expect("project");
    let source = Source::new(
        SourceId::from_u128(2),
        "voice.wav",
        "media/voice.wav",
        48_000,
        1,
        48_000,
    )
    .expect("source");
    project.sources.push(source);
    let track = Track::new(TrackId::from_u128(3), "Track 1");
    project.tracks.push(track);
    (project, TrackId::from_u128(3), SourceId::from_u128(2))
}

fn fixture_clip(id: u128) -> Clip {
    Clip::new(
        ClipId::from_u128(id),
        SourceId::from_u128(2),
        FrameRange::new(0, 4_800).expect("range"),
        0,
    )
}

#[test]
fn add_and_remove_track_round_trip() {
    let (mut project, _, _) = fixture();
    let initial = serde_json::to_string(&project).expect("json");
    let mut stack = CommandStack::new();
    stack
        .execute(
            &mut project,
            Box::new(AddTrack::new(Track::new(TrackId::from_u128(4), "New"), 1)),
        )
        .expect("add");
    assert_eq!(project.tracks.len(), 2);
    stack.undo(&mut project).expect("undo");
    assert_eq!(serde_json::to_string(&project).expect("json"), initial);
    stack.redo(&mut project).expect("redo");
    assert_eq!(project.tracks.len(), 2);
}

#[test]
fn split_and_revert_restores_clip() {
    let (mut project, track_id, _) = fixture();
    project.tracks[0].clips.push(fixture_clip(10));
    let initial = serde_json::to_string(&project).expect("json");
    let mut stack = CommandStack::new();
    stack
        .execute(
            &mut project,
            Box::new(SplitClip::new(
                ClipId::from_u128(10),
                2_400,
                ClipId::from_u128(11),
            )),
        )
        .expect("split");
    assert_eq!(project.clip_count(), 2);
    project.validate().expect("valid after split");
    stack.undo(&mut project).expect("undo");
    assert_eq!(serde_json::to_string(&project).expect("json"), initial);
    let _ = track_id;
}

#[test]
fn move_clip_between_tracks_and_back() {
    let (mut project, _, _) = fixture();
    project
        .tracks
        .push(Track::new(TrackId::from_u128(4), "Track 2"));
    project.tracks[0].clips.push(fixture_clip(10));
    let initial = serde_json::to_string(&project).expect("json");
    let mut stack = CommandStack::new();
    stack
        .execute(
            &mut project,
            Box::new(MoveClip::new(
                ClipId::from_u128(10),
                TrackId::from_u128(4),
                0,
                9_600,
            )),
        )
        .expect("move");
    assert_eq!(project.tracks[0].clips.len(), 0);
    assert_eq!(project.tracks[1].clips[0].timeline_start, 9_600);
    stack.undo(&mut project).expect("undo");
    assert_eq!(serde_json::to_string(&project).expect("json"), initial);
}

#[test]
fn new_command_clears_redo() {
    let (mut project, track_id, _) = fixture();
    let mut stack = CommandStack::new();
    stack
        .execute(&mut project, Box::new(RenameTrack::new(track_id, "A")))
        .expect("rename");
    stack.undo(&mut project).expect("undo");
    assert!(stack.can_redo());
    stack
        .execute(&mut project, Box::new(RenameTrack::new(track_id, "B")))
        .expect("rename");
    assert!(!stack.can_redo());
}

#[test]
fn stack_limit_drops_oldest_history() {
    let (mut project, track_id, _) = fixture();
    let mut stack = CommandStack::with_limit(2);
    for name in ["A", "B", "C"] {
        stack
            .execute(&mut project, Box::new(RenameTrack::new(track_id, name)))
            .expect("rename");
    }
    assert_eq!(stack.undo_len(), 2);
    assert_eq!(stack.undo_label(), Some("Rename track"));
}

#[test]
fn marker_add_remove_rename_and_loop_range_round_trip() {
    let mut project = Project::new("test").expect("project");
    let marker_id = MarkerId::from_u128(10);
    let region_id = RegionId::from_u128(11);
    let mut stack = CommandStack::new();

    stack
        .execute(
            &mut project,
            Box::new(AddMarker::new(marker_id, 1_000, " verse ")),
        )
        .expect("add marker");
    assert_eq!(project.markers[0].label, " verse ");
    assert_eq!(project.markers.len(), 1);

    stack.undo(&mut project).expect("undo");
    assert!(project.markers.is_empty());
    stack.redo(&mut project).expect("redo");
    assert_eq!(project.markers.len(), 1);

    stack
        .execute(
            &mut project,
            Box::new(RenameMarker::new(marker_id, "intro")),
        )
        .expect("rename marker");
    assert_eq!(project.markers[0].label, "intro");
    stack.undo(&mut project).expect("undo");
    assert_eq!(project.markers[0].label, " verse ");

    stack
        .execute(&mut project, Box::new(RemoveMarker::new(marker_id)))
        .expect("remove marker");
    assert!(project.markers.is_empty());
    stack.undo(&mut project).expect("undo");
    assert_eq!(project.markers.len(), 1);

    let range = FrameRange::new(256, 2_048).expect("range");
    stack
        .execute(
            &mut project,
            Box::new(AddRegion::new(region_id, range, "refrain")),
        )
        .expect("add region");
    assert_eq!(project.regions.len(), 1);
    assert_eq!(project.regions[0].range, range);

    stack
        .execute(&mut project, Box::new(SetLoopRange::from_range(range)))
        .expect("set loop");
    assert_eq!(project.loop_range, Some(range));
    stack.undo(&mut project).expect("undo loop");
    assert_eq!(project.loop_range, None);
    stack.redo(&mut project).expect("redo loop");
    assert_eq!(project.loop_range, Some(range));

    stack
        .execute(&mut project, Box::new(SetLoopRange::new(None)))
        .expect("clear loop");
    assert_eq!(project.loop_range, None);
}

#[test]
fn track_color_and_arm_round_trip() {
    let (mut project, track_id, _) = fixture();
    let initial = serde_json::to_string(&project).expect("json");
    let mut stack = CommandStack::new();

    stack
        .execute(
            &mut project,
            Box::new(SetTrackColor::new(track_id, [12, 34, 56])),
        )
        .expect("set color");
    assert_eq!(project.tracks[0].color, [12, 34, 56]);

    stack
        .execute(&mut project, Box::new(SetTrackArm::new(track_id, true)))
        .expect("arm");
    assert!(project.tracks[0].armed);

    stack.undo(&mut project).expect("undo arm");
    assert!(!project.tracks[0].armed);
    assert_eq!(project.tracks[0].color, [12, 34, 56]);

    stack.undo(&mut project).expect("undo color");
    assert_eq!(serde_json::to_string(&project).expect("json"), initial);
}

#[test]
fn crossfade_adjacent_clips_is_undoable() {
    let (mut project, _track_id, _) = fixture();
    project.tracks[0].clips.push(Clip::new(
        ClipId::from_u128(10),
        SourceId::from_u128(2),
        FrameRange::new(0, 4_800).expect("range"),
        0,
    ));
    project.tracks[0].clips.push(Clip::new(
        ClipId::from_u128(11),
        SourceId::from_u128(2),
        FrameRange::new(0, 4_800).expect("range"),
        4_800,
    ));
    let initial = serde_json::to_string(&project).expect("json");
    let mut stack = CommandStack::new();
    stack
        .execute(
            &mut project,
            Box::new(CrossfadeClips::new(
                ClipId::from_u128(10),
                ClipId::from_u128(11),
                1_000,
            )),
        )
        .expect("crossfade");
    assert_eq!(project.tracks[0].clips[0].fade_out_frames, 1_000);
    assert_eq!(project.tracks[0].clips[1].fade_in_frames, 1_000);
    stack.undo(&mut project).expect("undo");
    assert_eq!(serde_json::to_string(&project).expect("json"), initial);
}

#[test]
fn track_effects_round_trip() {
    let (mut project, track_id, _) = fixture();
    let initial = serde_json::to_string(&project).expect("json");
    let mut stack = CommandStack::new();

    let effect = EffectRef::new("eq-1", "eq");

    stack
        .execute(
            &mut project,
            Box::new(SetTrackEffects::new(track_id, vec![effect.clone()])),
        )
        .expect("set track effects");
    assert_eq!(project.tracks[0].effects.len(), 1);
    assert_eq!(project.tracks[0].effects[0].id, "eq-1");

    stack.undo(&mut project).expect("undo effects");
    assert!(project.tracks[0].effects.is_empty());
    assert_eq!(serde_json::to_string(&project).expect("json"), initial);
}
