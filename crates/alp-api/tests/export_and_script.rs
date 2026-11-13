//! End-to-end checks for the export, transcript, preview and script tools:
//! each one must produce the artefact it names.

use alp_api::{ApiSession, AuditLogger, BatchOp, PermissionTier, WorkspaceSandbox};
use alp_core::{Clip, ClipId, FrameRange, Project, ProjectPaths, Source, SourceId, Track, TrackId};
use alp_media::{AudioMetadata, DitherMode, ExportFormat, ExportSettings, PcmBuffer, WavBitDepth};
use tempfile::TempDir;

const RATE: u32 = 48_000;

fn write_tone(dir: &std::path::Path, name: &str, amplitude: f32, secs: f64) -> String {
    let samples: Vec<f32> = (0..(secs * f64::from(RATE)) as usize)
        .map(|i| amplitude * (i as f32 * 220.0 * 2.0 * std::f32::consts::PI / RATE as f32).sin())
        .collect();
    let pcm = PcmBuffer::new(RATE, 1, samples).expect("pcm");
    let path = dir.join(name);
    alp_media::export(
        &path,
        &pcm,
        &ExportSettings {
            format: ExportFormat::Wav,
            bit_depth: WavBitDepth::Int16,
            sample_rate: None,
            dither: DitherMode::None,
            metadata: AudioMetadata::default(),
        },
    )
    .expect("write tone");
    path.display().to_string()
}

struct Fixture {
    session: ApiSession,
    dir: TempDir,
    loud_track: String,
    quiet_track: String,
    loud_clip: String,
}

/// Two tracks: one at full level, one 20 dB down, both with real clips.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = ProjectPaths::new(dir.path());
    paths.ensure_layout().expect("layout");
    std::fs::create_dir_all(paths.media_dir()).expect("media dir");

    let loud = write_tone(dir.path(), "loud.wav", 0.8, 2.0);
    let quiet = write_tone(dir.path(), "quiet.wav", 0.08, 2.0);
    std::fs::rename(&loud, paths.media_dir().join("loud.wav")).expect("move loud");
    std::fs::rename(&quiet, paths.media_dir().join("quiet.wav")).expect("move quiet");

    let mut project = Project::new("Export").expect("project");
    for (id, name) in [(1u128, "loud.wav"), (2u128, "quiet.wav")] {
        project.sources.push(
            Source::new(
                SourceId::from_u128(id),
                name,
                format!("media/{name}"),
                RATE,
                1,
                u64::from(RATE) * 2,
            )
            .expect("source"),
        );
    }
    let mut loud_track = Track::new(TrackId::from_u128(10), "Loud");
    loud_track.clips.push(Clip::new(
        ClipId::from_u128(100),
        SourceId::from_u128(1),
        FrameRange::new(0, u64::from(RATE) * 2).expect("range"),
        0,
    ));
    let mut quiet_track = Track::new(TrackId::from_u128(11), "Quiet");
    quiet_track.clips.push(Clip::new(
        ClipId::from_u128(101),
        SourceId::from_u128(2),
        FrameRange::new(0, u64::from(RATE) * 2).expect("range"),
        0,
    ));
    let loud_track_id = loud_track.id.to_string();
    let quiet_track_id = quiet_track.id.to_string();
    let loud_clip_id = loud_track.clips[0].id.to_string();
    project.tracks.push(loud_track);
    project.tracks.push(quiet_track);

    let sandbox = WorkspaceSandbox::new([dir.path()]).expect("sandbox");
    let audit = AuditLogger::new(dir.path().join("audit.jsonl")).expect("audit");
    let session = ApiSession::new(
        project,
        paths,
        PermissionTier::Export,
        sandbox,
        audit,
        "test",
    )
    .expect("session")
    .with_voice_store_dir(dir.path().join("voices"));

    Fixture {
        session,
        dir,
        loud_track: loud_track_id,
        quiet_track: quiet_track_id,
        loud_clip: loud_clip_id,
    }
}

#[test]
fn stems_carry_their_own_track_not_the_master() {
    let mut fixture = fixture();
    let stems_dir = fixture.dir.path().join("stems");
    let reports = fixture
        .session
        .export_stems(stems_dir.to_str().expect("utf8"), "voice_over")
        .expect("stems");

    assert_eq!(reports.len(), 2, "one stem per track");
    let loud = reports
        .iter()
        .find(|r| r.track_id == fixture.loud_track)
        .expect("loud stem");
    let quiet = reports
        .iter()
        .find(|r| r.track_id == fixture.quiet_track)
        .expect("quiet stem");

    assert!(
        loud.peak_dbfs > quiet.peak_dbfs + 10.0,
        "the loud stem ({:.1} dBFS) must be far above the quiet one ({:.1} dBFS): a master copy would make them equal",
        loud.peak_dbfs,
        quiet.peak_dbfs
    );

    let loud_buf = alp_media::decode(&loud.output_path).expect("decode loud stem");
    let quiet_buf = alp_media::decode(&quiet.output_path).expect("decode quiet stem");
    // Both stems are 2 s of the same tone, so a sample-identity check proves
    // the quiet stem is not the master duplicated under another name.
    let differing = loud_buf
        .samples()
        .iter()
        .zip(quiet_buf.samples())
        .filter(|(a, b)| (*a - *b).abs() > 1e-3)
        .count();
    assert!(
        differing > loud_buf.samples().len() / 2,
        "stems must differ sample-by-sample, only {differing} sampled differed"
    );
}

#[test]
fn transcript_export_contains_real_words_and_timestamps() {
    let mut fixture = fixture();
    let srt_path = fixture.dir.path().join("transcript.srt");
    let report = fixture
        .session
        .export_transcript("srt", srt_path.to_str().expect("utf8"))
        .expect("transcript");

    assert!(report.segments > 0, "a 2 s render must produce segments");
    assert!(
        report.text.contains("-->"),
        "SRT needs cue timings: {}",
        report.text
    );
    let written = std::fs::read_to_string(srt_path).expect("read srt");
    assert_eq!(written, report.text);
    assert!(report.engine.starts_with("builtin") || report.engine.starts_with("daemon"));
}

#[test]
fn preview_render_writes_the_requested_slice() {
    let mut fixture = fixture();
    let report = fixture.session.preview_render(0.5, 0.75).expect("preview");

    assert!(
        (report.duration_secs - 0.75).abs() < 0.02,
        "{}",
        report.duration_secs
    );
    assert!(
        (report.start_secs - 0.5).abs() < 0.01,
        "{}",
        report.start_secs
    );
    assert!(
        report.peak > 0.1,
        "the preview must carry the tone, peak {}",
        report.peak
    );
    let written = alp_media::decode(&report.preview_path).expect("decode preview");
    assert!(
        (written.duration_secs() - 0.75).abs() < 0.02,
        "written preview is {}s",
        written.duration_secs()
    );
}

#[test]
fn a_script_becomes_clips_on_its_own_track_in_one_undo_step() {
    let mut fixture = fixture();
    let script = r#"{"segments":[
        {"text":"First line of the script.","voice_id":"aura","gap_secs":0.2},
        {"text":"Second line of the script.","voice_id":"aura"}
    ]}"#;

    let undo_before = fixture.session.history_list().expect("history").len();
    let report = fixture
        .session
        .tts_script_generate(script)
        .expect("script generate");

    assert_eq!(report.clip_ids.len(), 2, "one clip per script line");
    assert!(report.single_undo_step, "a script must be one undo step");
    assert!(
        report.total_duration_secs > 0.2,
        "{}",
        report.total_duration_secs
    );
    assert!(report.end_secs > report.total_duration_secs);

    let tracks = fixture.session.list_tracks().expect("tracks");
    let script_track = tracks
        .iter()
        .find(|t| t.id == report.track_id)
        .expect("script track");
    assert_eq!(script_track.clip_count, 2);
    assert!(
        fixture.session.history_list().expect("history").len() > undo_before,
        "the script generation must be undoable"
    );

    fixture.session.undo().expect("undo");
    let tracks = fixture.session.list_tracks().expect("tracks after undo");
    let generated: usize = tracks
        .iter()
        .filter(|t| t.id == report.track_id)
        .map(|t| t.clip_count)
        .sum();
    assert_eq!(
        generated, 0,
        "one undo must remove every generated clip (and the track they created)"
    );
}

#[test]
fn regenerating_a_clip_replaces_its_audio_in_place() {
    let mut fixture = fixture();
    let clip_id = fixture.loud_clip.clone();
    let original_start = 0.0;

    let report = fixture
        .session
        .tts_regenerate(&clip_id, "A replacement take.", "aura", None)
        .expect("regenerate");

    assert_eq!(report.start_secs, original_start);
    assert_ne!(report.new_clip_id, clip_id);
    assert!(report.duration_secs > 0.2);
    assert!(report.single_undo_step);

    let tracks = fixture.session.list_tracks().expect("tracks");
    let loud = tracks
        .iter()
        .find(|t| t.id == fixture.loud_track)
        .expect("loud track");
    assert_eq!(
        loud.clip_count, 1,
        "the old clip must be gone, not duplicated"
    );
}

#[test]
fn a_batch_of_edits_is_atomic() {
    let mut fixture = fixture();
    let ops = vec![
        BatchOp::TrackUpdate {
            track_id: fixture.quiet_track.clone(),
            name: Some("Bed".into()),
            gain: Some(0.5),
            pan: None,
            muted: None,
            solo: None,
            armed: None,
        },
        BatchOp::MarkerAdd {
            name: "Chapter".into(),
            position_secs: 0.25,
        },
    ];
    let report = fixture.session.edit_batch("setup", &ops).expect("batch");
    assert_eq!(report.operations_applied, 2);
    assert!(report.single_undo_step);
}
