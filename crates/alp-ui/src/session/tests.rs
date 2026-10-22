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
