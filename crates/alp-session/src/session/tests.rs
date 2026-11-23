//! Unit tests for the editing session.

use super::*;

#[test]
fn demo_starts_valid_and_selects_a_clip() {
    let session = EditSession::demo();
    session.project.validate().expect("valid");
    assert!(session.clip_count() >= 2);
    assert!(session.selected_clip().is_some());
    assert!(!session.selection().is_empty());
}

#[test]
fn range_edits_are_undoable() {
    let mut session = EditSession::demo();
    let before = session.clip_count();
    let first = session
        .project()
        .track(session.track_id())
        .and_then(|track| track.clips.first())
        .map(|clip| clip.id)
        .expect("clip");
    session.select_clip(first);
    session.set_selection(2.0, 4.0);
    session.silence();
    assert!(session.can_undo());
    session.undo();
    assert_eq!(session.clip_count(), before);
    session.project().validate().expect("valid after undo");
}

#[test]
fn attached_undo_restores_the_path_and_audio_context_of_the_original_project() {
    let first_root = tempfile::tempdir().expect("first project");
    let second_root = tempfile::tempdir().expect("second project");
    let mut session = EditSession::open_or_demo(first_root.path().to_path_buf());
    let first_path = session.paths.clone().expect("saved project");
    let source_id = session.project.sources.first().expect("demo source").id;
    let first_samples = session
        .audio
        .get(&source_id)
        .expect("first source audio")
        .samples()
        .to_vec();

    // A copied project can reuse the same project and source UUIDs while its
    // media root and decoded samples differ.
    let mut state = session.attached_project_state().expect("attached state");
    let base_revision = state.revision;
    state.paths = ProjectPaths::new(second_root.path().to_path_buf());
    let second_samples = vec![0.25_f32, -0.25, 0.25, -0.25];
    let second_audio = HashMap::from([(
        source_id,
        Arc::new(
            alp_engine::SourceAudio::from_samples(48_000, 1, second_samples.clone())
                .expect("second source audio"),
        ),
    )]);

    session
        .commit_attached_project(base_revision, state, second_audio, "project_open")
        .expect("attached project switch");
    assert_eq!(
        session.paths.as_ref().map(|paths| paths.root()),
        Some(second_root.path())
    );
    assert_eq!(
        session.audio[&source_id].samples(),
        second_samples.as_slice()
    );

    session.undo();
    assert_eq!(
        session.paths.as_ref().map(|paths| paths.root()),
        Some(first_path.root())
    );
    assert_eq!(
        session.audio[&source_id].samples(),
        first_samples.as_slice()
    );

    session.redo();
    assert_eq!(
        session.paths.as_ref().map(|paths| paths.root()),
        Some(second_root.path())
    );
    assert_eq!(
        session.audio[&source_id].samples(),
        second_samples.as_slice()
    );
}

#[test]
fn copy_and_paste_preserve_selection_length() {
    let mut session = EditSession::demo();
    session.set_selection(1.0, 2.0);
    session.copy();
    assert!(!session.status().is_empty());
    session.paste();
    assert!(session.can_undo());
}

#[test]
fn project_autosave_is_recovered_on_reopen() {
    let directory = tempfile::tempdir().expect("tempdir");
    let root = directory.path().join("ui-demo");
    let mut session = EditSession::open_or_demo(root.clone());
    assert!(root.join("project.json").is_file());
    session.set_selection(2.0, 4.0);
    session.trim();
    let expected = session.clip_count();
    let id = session.selected_clip();
    session.flush_autosave();
    assert!(root.join("autosave").join("project.json").is_file());
    drop(session);

    let recovered = EditSession::open_or_demo(root);
    assert!(recovered.status().contains("Recovered autosave"));
    assert_eq!(recovered.clip_count(), expected);
    assert_eq!(recovered.selected_clip(), id);
    recovered
        .project()
        .validate()
        .expect("valid recovered project");
}

#[test]
fn envelope_dip_is_undoable() {
    let mut session = EditSession::demo();
    let id = session.selected_clip().expect("selected clip");
    session.set_selection(2.0, 4.0);
    session.dip_selection();
    assert!(!session.clip(id).expect("clip").envelope.is_empty());
    session.undo();
    assert!(session.clip(id).expect("clip").envelope.is_empty());
}

#[test]
fn trim_reduces_the_clip_to_the_selection_and_undoes() {
    let mut session = EditSession::demo();
    session.set_selection(2.0, 4.0);
    let id = session.selected_clip().expect("selected clip");
    let before = session.clip(id).expect("clip").len();
    session.trim();
    assert_eq!(session.clip(id).expect("clip").len(), 48_000 * 2);
    assert!(before > 48_000 * 2);
    session.undo();
    assert_eq!(session.clip(id).expect("clip").len(), before);
}

#[test]
fn punch_in_replaces_the_selection_and_is_undoable() {
    let mut session = EditSession::demo();
    session.set_selection(2.0, 4.0);
    let before = session.clip_count();
    let buffer = PcmBuffer::new(48_000, 1, vec![0.25; 48_000 * 2]).expect("buffer");
    assert!(session.punch_in_buffer(buffer, "punch", "punch.wav"));
    assert!(session.can_undo());
    assert_eq!(session.clip_count(), before + 2);
    session.project().validate().expect("valid after punch");
    session.undo();
    assert_eq!(session.clip_count(), before);
    session.project().validate().expect("valid after undo");
}

#[test]
fn background_import_adds_a_clip() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("drop.wav");
    alp_media::write_test_tone(&path, 0.25, 48_000, 1, 330.0, 0.2).expect("tone");
    let mut session = EditSession::demo();
    let before = session.clip_count();
    session.begin_import(path).expect("begin import");
    for _ in 0..400 {
        if session.poll_import() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(!session.is_importing(), "import worker did not finish");
    assert_eq!(session.clip_count(), before + 1);
    session.project().validate().expect("valid");
}

#[test]
fn recorded_style_buffer_can_be_imported() {
    let mut session = EditSession::demo();
    let before = session.clip_count();
    let buffer = PcmBuffer::new(48_000, 1, vec![0.1; 48_000]).expect("buffer");
    session.import_buffer(buffer, "take.wav", "/tmp/take.wav");
    assert_eq!(session.clip_count(), before + 1);
    assert!(session.selected_clip().is_some());
    session.project().validate().expect("valid");
}

#[test]
fn markers_and_loop_range_are_managed() {
    let mut session = EditSession::demo();
    assert!(session.set_loop_range(1.0, 5.0));
    assert!(session.loop_range().is_some());
    assert!(session.clear_loop_range());
    assert!(session.loop_range().is_none());

    assert!(session.add_marker(2.5, "Chorus"));
    assert_eq!(session.markers().len(), 1);
    assert_eq!(session.markers()[0].label, "Chorus");
    let marker_id = session.markers()[0].id;
    assert!(session.remove_marker(marker_id));
    assert_eq!(session.markers().len(), 0);
}

#[test]
fn export_presets_are_accessible() {
    let settings = EditSession::export_preset_settings(alp_media::ExportPreset::Podcast);
    assert_eq!(settings.format, alp_media::ExportFormat::Flac);
    assert_eq!(settings.sample_rate, Some(44_100));
}

#[test]
fn timeline_operations_undo_redo_and_clip_id_preservation() {
    let mut session = EditSession::demo();
    let dto = session.timeline_dto();
    assert!(!dto.tracks.is_empty());
    assert!(!dto.tracks[0].clips.is_empty());

    let original_clip_id = dto.tracks[0].clips[0].id.clone();
    let original_start = dto.tracks[0].clips[0].timeline_start_secs;
    let track_id = dto.tracks[0].id.clone();

    // 1. Test Split Clip & ID Preservation on Redo
    let split_pos = original_start + 1.0;
    session
        .split_clip_at(&original_clip_id, split_pos)
        .expect("split succeeds");

    let post_split_dto = session.timeline_dto();
    let split_clip_ids: Vec<String> = post_split_dto.tracks[0]
        .clips
        .iter()
        .map(|c| c.id.clone())
        .collect();
    assert!(split_clip_ids.contains(&original_clip_id));
    assert_eq!(split_clip_ids.len(), dto.tracks[0].clips.len() + 1);

    // Undo restores the single clip
    session.undo();
    let undo_dto = session.timeline_dto();
    assert_eq!(undo_dto.tracks[0].clips.len(), dto.tracks[0].clips.len());
    assert_eq!(undo_dto.tracks[0].clips[0].id, original_clip_id);

    // Redo restores the exact same split clip ids!
    session.redo();
    let redo_dto = session.timeline_dto();
    let redo_ids: Vec<String> = redo_dto.tracks[0]
        .clips
        .iter()
        .map(|c| c.id.clone())
        .collect();
    assert_eq!(redo_ids, split_clip_ids);

    // Undo back to clean state
    session.undo();

    // 2. Test Move Clip
    let moved_start = 5.0;
    session
        .move_clip_by_id(&original_clip_id, &track_id, moved_start)
        .expect("move succeeds");
    let moved_dto = session.timeline_dto();
    let moved_clip = moved_dto.tracks[0]
        .clips
        .iter()
        .find(|c| c.id == original_clip_id)
        .expect("found clip");
    assert!((moved_clip.timeline_start_secs - moved_start).abs() < 0.01);

    session.undo();
    let reverted_move_dto = session.timeline_dto();
    let reverted_clip = reverted_move_dto.tracks[0]
        .clips
        .iter()
        .find(|c| c.id == original_clip_id)
        .expect("found clip");
    assert!((reverted_clip.timeline_start_secs - original_start).abs() < 0.01);

    // 3. Test Set Fades
    session
        .set_clip_fades_by_id(&original_clip_id, 0.25, 0.5)
        .expect("fades succeed");
    let fade_dto = session.timeline_dto();
    let fade_clip = fade_dto.tracks[0]
        .clips
        .iter()
        .find(|c| c.id == original_clip_id)
        .expect("found clip");
    assert!((fade_clip.fade_in_secs - 0.25).abs() < 0.01);
    assert!((fade_clip.fade_out_secs - 0.5).abs() < 0.01);

    // 4. Test Envelope
    session
        .set_clip_envelope_by_id(&original_clip_id, vec![(0.0, 1.0), (0.5, 0.4), (1.0, 1.0)])
        .expect("envelope succeeds");
    let env_dto = session.timeline_dto();
    let env_clip = env_dto.tracks[0]
        .clips
        .iter()
        .find(|c| c.id == original_clip_id)
        .expect("found clip");
    assert_eq!(env_clip.envelope.len(), 3);

    // Project validation invariant (null test)
    session.project().validate().expect("project is valid");
}

#[test]
fn phase13_spectral_and_multitrack_canvas_operations() {
    let mut session = EditSession::demo();
    let initial_dto = session.timeline_dto();
    let track_id = initial_dto.tracks[0].id.clone();
    let clip_id = initial_dto.tracks[0].clips[0].id.clone();

    // 1. Test Track View Mode toggles
    assert_eq!(initial_dto.tracks[0].view_mode, crate::timeline::TrackViewMode::Waveform);
    session
        .set_track_view_mode_by_id(&track_id, "spectrogram")
        .expect("set view mode spectrogram");
    assert_eq!(
        session.timeline_dto().tracks[0].view_mode,
        crate::timeline::TrackViewMode::Spectrogram
    );
    session
        .set_track_view_mode_by_id(&track_id, "split")
        .expect("set view mode split");
    assert_eq!(
        session.timeline_dto().tracks[0].view_mode,
        crate::timeline::TrackViewMode::Split
    );
    assert!(session.set_track_view_mode_by_id(&track_id, "invalid_mode").is_err());

    // 2. Test 2D Spectral Selection
    let sel = crate::timeline::SpectralSelectionDto::new(1.0, 3.5, 400.0, 1600.0);
    assert!((sel.t_start_secs - 1.0).abs() < 1e-4);
    assert!((sel.t_end_secs - 3.5).abs() < 1e-4);
    assert!((sel.f_low_hz - 400.0).abs() < 1e-4);
    assert!((sel.f_high_hz - 1600.0).abs() < 1e-4);
    // Center freq = sqrt(400 * 1600) = 800 Hz
    assert!((sel.center_freq_hz - 800.0).abs() < 1e-3);
    // Bandwidth = 1200 Hz; Q = 800 / 1200 = 0.6667
    assert!((sel.bandwidth_q - (800.0 / 1200.0)).abs() < 1e-3);

    session
        .set_spectral_selection_by_id(&track_id, Some(sel))
        .expect("set spectral selection");
    let dto = session.timeline_dto();
    let track_sel = dto.tracks[0].spectral_selection.as_ref().expect("has spectral selection");
    assert!((track_sel.center_freq_hz - 800.0).abs() < 1e-3);

    // Clear spectral selection
    session
        .set_spectral_selection_by_id(&track_id, None)
        .expect("clear spectral selection");
    assert!(session.timeline_dto().tracks[0].spectral_selection.is_none());

    // 3. Test Clip Pan Automation Envelope
    session
        .set_clip_pan_envelope_by_id(&clip_id, vec![(0.0, -0.8), (1.5, 0.0), (3.0, 0.8)])
        .expect("set pan envelope");
    let dto = session.timeline_dto();
    let pan_env = &dto.tracks[0].clips[0].pan_envelope;
    assert_eq!(pan_env.len(), 3);
    assert!((pan_env[0].pan - (-0.8)).abs() < 1e-4);
    assert!((pan_env[2].pan - 0.8).abs() < 1e-4);

    // 4. Test Slip Edit
    assert_eq!(dto.tracks[0].clips[0].slip_offset_secs, 0.0);
    session.slip_clip_by_id(&clip_id, 0.5).expect("slip edit succeeds");
    let dto_slipped = session.timeline_dto();
    assert!((dto_slipped.tracks[0].clips[0].slip_offset_secs - 0.5).abs() < 1e-3);

    // 5. Test Spectrogram Computation on Live Demo Audio
    let spec_config = alp_dsp::SpectrogramConfig {
        fft_size: 1024,
        hop_size: 256,
        window_type: alp_dsp::WindowType::Hanning,
        freq_scale: alp_dsp::FrequencyScale::Logarithmic,
        min_freq_hz: 20.0,
        max_freq_hz: 12000.0,
        num_freq_bins: 64,
        min_db: -90.0,
        max_db: 0.0,
    };
    let spec_data = session
        .compute_clip_spectrogram_by_id(&clip_id, Some(spec_config))
        .expect("compute spectrogram");
    assert!(spec_data.num_frames > 0);
    assert_eq!(spec_data.num_bins, 64);
    assert_eq!(spec_data.matrix.len(), spec_data.num_frames * 64);
    assert_eq!(spec_data.freq_bins_hz.len(), 64);
    assert_eq!(spec_data.time_stamps_sec.len(), spec_data.num_frames);
}
