//! Stress testing, fuzzing, and corrupted-project recovery tests for Phase 9.

use alp_core::{
    AddTrack, Clip, ClipId, CommandStack, FrameRange, Project, ProjectPaths, Source, SourceId,
    Track, TrackId, from_json, to_json,
};
use tempfile::tempdir;

#[test]
fn fuzzed_and_malformed_json_inputs_are_safely_rejected() {
    let cases = [
        "",
        "   ",
        "<!DOCTYPE html><html><body>Error</body></html>",
        "{\"schema_version\": 1}",
        "{\"schema_version\": 999999, \"project\": {}}",
        "{\"schema_version\": -1, \"project\": {}}",
        "{\"project\": {\"id\": \"prj_1\", \"name\": \"No Version\"}}",
        "{\"schema_version\": 1, \"project\": null}",
        "{\"schema_version\": 1, \"project\": {\"id\": \"\", \"name\": \"\", \"sample_rate\": 0, \"tempo\": 120.0, \"time_signature\": [4, 4], \"tracks\": []}}",
        "{\"schema_version\": 1, \"project\": {\"id\": \"p1\", \"name\": \"Bad Rate\", \"sample_rate\": 99999999, \"tempo\": 120.0, \"time_signature\": [4, 4], \"tracks\": []}}",
        "{\"schema_version\": 1, \"project\": {\"id\": \"p1\", \"name\": \"Dup Tracks\", \"sample_rate\": 48000, \"tempo\": 120.0, \"time_signature\": [4, 4], \"tracks\": [{\"id\": \"t1\", \"name\": \"T1\", \"gain\": 1.0, \"pan\": 0.0, \"mute\": false, \"solo\": false, \"clips\": []}, {\"id\": \"t1\", \"name\": \"T2\", \"gain\": 1.0, \"pan\": 0.0, \"mute\": false, \"solo\": false, \"clips\": []}]}}",
    ];

    for (idx, malformed) in cases.iter().enumerate() {
        let result = from_json(malformed);
        assert!(
            result.is_err(),
            "fuzzed case #{idx} should have been rejected but parsed successfully: {malformed}"
        );
    }
}

#[test]
fn relative_media_path_traversal_is_blocked() {
    let dir = tempdir().expect("tempdir");
    let proj_dir = ProjectPaths::new(dir.path());

    let escape_attempts = [
        "../secret.wav",
        "../../etc/shadow",
        "media/../../sensitive.dat",
        "/absolute/path/file.wav",
    ];

    for attempt in escape_attempts {
        let res = proj_dir.resolve_relative(attempt);
        assert!(
            res.is_err(),
            "path traversal attempt should fail: {attempt}"
        );
    }

    assert!(proj_dir.resolve_relative("media/take_1.wav").is_ok());
    assert!(proj_dir.resolve_relative("ambient.flac").is_ok());
}

#[test]
fn corrupted_autosave_recovery_safeguard() {
    let dir = tempdir().expect("tempdir");
    let proj_dir = ProjectPaths::new(dir.path());
    proj_dir.ensure_layout().expect("layout");

    // Write a valid project.json
    let valid_project = Project::new("Main Project").expect("valid project");
    proj_dir.save(&valid_project).expect("save");

    // Write a corrupted autosave copy
    let autosave_file = proj_dir.autosave_file();
    std::fs::create_dir_all(autosave_file.parent().unwrap()).unwrap();
    std::fs::write(&autosave_file, b"corrupted garbage {[[}").unwrap();

    // Recovery must fail and refuse to replace project.json
    let recovered = proj_dir.recover_autosave();
    assert!(
        recovered.is_err(),
        "recovering corrupted autosave must return Err"
    );

    // Ensure the original project.json is still intact
    let loaded = proj_dir.load().expect("load project");
    assert_eq!(loaded.name, "Main Project");

    // Ensure the corrupt autosave is kept for debugging
    assert!(autosave_file.is_file());
}

#[test]
fn valid_autosave_recovers_interrupted_work() {
    let dir = tempdir().expect("tempdir");
    let proj_dir = ProjectPaths::new(dir.path());
    proj_dir.ensure_layout().expect("layout");

    let initial = Project::new("Saved Copy").expect("valid project");
    proj_dir.save(&initial).expect("save");

    let mut autosaved = initial.clone();
    autosaved.name = "Recovered Session Work".into();
    proj_dir.autosave(&autosaved).expect("autosave");

    assert!(proj_dir.has_autosave());
    let recovered = proj_dir.recover_autosave().expect("recovery");
    assert!(recovered);

    let current = proj_dir.load().expect("load");
    assert_eq!(current.name, "Recovered Session Work");
}

#[test]
fn large_project_stress_and_performance_benchmark() {
    let mut project = Project::new("Stress Project").expect("project");

    let source_id = SourceId::new();
    let source = Source::new(
        source_id,
        "source.wav",
        "media/source.wav",
        48_000,
        1,
        48_000 * 600,
    )
    .expect("source");
    project.sources.push(source);

    // Create 50 tracks, each with 10 clips = 500 clips
    let track_count = 50;
    let clips_per_track = 10;

    for t in 0..track_count {
        let mut track = Track::new(TrackId::new(), format!("Track {t:02}"));
        for c in 0..clips_per_track {
            let start = (c as u64) * 48_000 * 5;
            let clip = Clip::new(
                ClipId::new(),
                source_id,
                FrameRange::new(0, 48_000 * 4).expect("range"),
                start,
            );
            track.clips.push(clip);
        }
        project.tracks.push(track);
    }

    assert_eq!(project.tracks.len(), 50);

    // Benchmark serialization time
    let start_time = std::time::Instant::now();
    let json = to_json(&project).expect("serialize");
    let serialize_elapsed = start_time.elapsed();
    assert!(
        serialize_elapsed.as_millis() < 100,
        "serialization exceeded 100ms: {:?}",
        serialize_elapsed
    );

    // Benchmark deserialization time
    let parse_start = std::time::Instant::now();
    let parsed = from_json(&json).expect("deserialize");
    let parse_elapsed = parse_start.elapsed();
    assert!(
        parse_elapsed.as_millis() < 100,
        "deserialization exceeded 100ms: {:?}",
        parse_elapsed
    );

    assert_eq!(parsed.tracks.len(), 50);
    let total_clips: usize = parsed.tracks.iter().map(|tr| tr.clips.len()).sum();
    assert_eq!(total_clips, 500);

    // Stress test command stack with 500 operations
    let mut stack = CommandStack::with_limit(100);
    for i in 0..500 {
        let cmd = AddTrack::new(
            Track::new(TrackId::new(), format!("UndoTrack {i}")),
            project.tracks.len(),
        );
        stack.execute(&mut project, Box::new(cmd)).expect("execute");
    }

    // Capacity limit of 100 must be enforced
    assert!(stack.can_undo());
    let mut undone = 0;
    while stack.can_undo() {
        assert!(stack.undo(&mut project).expect("undo"));
        undone += 1;
    }
    assert_eq!(undone, 100, "stack limit trimmed older history correctly");
}
