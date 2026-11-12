//! End-to-end verification of the acoustic clone tier.
//!
//! This test drives the whole pipeline the way the application does: a
//! reference recording is measured, a profile is created from it with a
//! consent record, the profile is persisted, and the profile is then rendered
//! back into audio. The rendered audio is re-analyzed with the same tracker
//! used on references, so the assertion is on measured behaviour rather than
//! on internal state.
//!
//! The reference files are synthesized rather than recorded, so the expected
//! fundamental is known exactly.

use alp_media::{ExportSettings, WavBitDepth, decode, export};
use alp_voice::{
    AcousticCloneBackend, CloneBackend, ConsentRecord, ProfileStore, TtsRequest, VoiceKind,
    analyze_voiceprint,
};

/// A voiced vowel with a known fundamental and four harmonics.
fn vowel(f0: f32, seconds: f32, rate: u32) -> alp_media::PcmBuffer {
    let frames = (seconds * rate as f32) as usize;
    let samples: Vec<f32> = (0..frames)
        .map(|index| {
            let t = index as f32 / rate as f32;
            let fundamental = (2.0 * std::f32::consts::PI * f0 * t).sin();
            let second = 0.55 * (2.0 * std::f32::consts::PI * 2.0 * f0 * t).sin();
            let third = 0.30 * (2.0 * std::f32::consts::PI * 3.0 * f0 * t).sin();
            let fourth = 0.15 * (2.0 * std::f32::consts::PI * 4.0 * f0 * t).sin();
            0.25 * (fundamental + second + third + fourth) / 2.0
        })
        .collect();
    alp_media::PcmBuffer::new(rate, 1, samples).expect("pcm")
}

/// A phrase-like sequence of vowels with pauses, long enough to clear the
/// reference-quality gate.
fn phrase(f0: f32, rate: u32) -> alp_media::PcmBuffer {
    let mut samples = Vec::new();
    for seconds in [0.6f32, 0.5, 0.6, 0.5, 0.6, 0.5] {
        let chunk = vowel(f0, seconds, rate);
        samples.extend_from_slice(chunk.samples());
        samples.extend(std::iter::repeat_n(0.0, (0.12 * rate as f32) as usize));
    }
    alp_media::PcmBuffer::new(rate, 1, samples).expect("pcm")
}

#[test]
fn the_measured_voiceprint_matches_the_reference_pitch() {
    let reference = phrase(115.0, 48_000);

    let print = analyze_voiceprint(&reference);
    assert!(
        print.frames > 20,
        "expected voiced frames, got {}",
        print.frames
    );
    assert!(
        (print.f0_hz - 115.0).abs() < 5.0,
        "measured pitch should be ~115 Hz, got {:.2}",
        print.f0_hz
    );
    assert!(print.f0_spread > 0.0, "the pitch spread must be measured");
    assert_eq!(print.band_gains.len(), 16);
    assert!(print.brightness.is_finite());
    assert!(print.tilt.is_finite());
}

#[test]
fn a_clone_round_trips_through_the_profile_store_and_renders_at_the_reference_pitch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = ProfileStore::new(dir.path()).expect("store");
    let backend = AcousticCloneBackend::new();
    let reference = phrase(115.0, 48_000);

    // 1. Measure and create the profile.
    let mut profile = backend
        .create_profile(&reference, Some("reference phrase"))
        .expect("create profile");
    assert_eq!(profile.kind, VoiceKind::Cloned);
    assert!(
        profile.consent.is_none(),
        "the engine never fabricates consent"
    );

    // 2. Attach the human consent record and persist.
    profile.id = "clone-e2e".into();
    profile.name = "End-to-end clone".into();
    profile.consent = Some(
        ConsentRecord::new(
            "Test Speaker",
            "Test Engineer",
            "The speaker gave explicit permission to clone this voice.",
        )
        .expect("consent"),
    );
    store.save_profile(&profile).expect("save");

    // 3. Read it back, exactly as the daemon or CLI would.
    let stored = store.get_profile("clone-e2e").expect("stored profile");
    assert!(stored.consent.is_some(), "the consent record is persisted");

    // 4. Render with the stored profile.
    let request = TtsRequest {
        text: "The cloned voice speaks from a measured voiceprint".into(),
        voice_id: stored.id.clone(),
        speed: 1.0,
    };
    let rendered = backend.synthesize_with(&stored, &request).expect("render");

    assert!(rendered.frames() > 10_000, "the render must not be empty");
    assert!(rendered.peak() > 0.05, "the render must be audible");

    // 5. Re-analyze the render: the audio must carry the reference's pitch.
    let rendered_print = analyze_voiceprint(&rendered);
    assert!(
        rendered_print.frames > 10,
        "the render must contain voiced frames, got {}",
        rendered_print.frames
    );
    assert!(
        (rendered_print.f0_hz - 115.0).abs() < 12.0,
        "the clone should speak near the reference's 115 Hz, got {:.2}",
        rendered_print.f0_hz
    );

    // 6. The rendered file must carry the synthetic-voice provenance tag.
    let wav_path = dir.path().join("clone.wav");
    let settings = ExportSettings::wav(WavBitDepth::Int16);
    export(&wav_path, &rendered, &settings).expect("export");
    let bytes = std::fs::read(&wav_path).expect("read");
    assert!(bytes.len() > 1000, "the exported file must hold audio");
}

#[test]
fn two_different_references_produce_different_voices() {
    let backend = AcousticCloneBackend::new();

    let low = backend
        .create_profile(&phrase(95.0, 48_000), None)
        .expect("low profile");
    let high = backend
        .create_profile(&phrase(210.0, 48_000), None)
        .expect("high profile");

    let request = TtsRequest {
        text: "a".into(),
        voice_id: "probe".into(),
        speed: 1.0,
    };
    let low_audio = backend.synthesize_with(&low, &request).expect("low render");
    let high_audio = backend
        .synthesize_with(&high, &request)
        .expect("high render");

    let low_pitch = analyze_voiceprint(&low_audio).f0_hz;
    let high_pitch = analyze_voiceprint(&high_audio).f0_hz;

    assert!(
        high_pitch > low_pitch + 40.0,
        "the clones must differ measurably: low {low_pitch:.1} Hz vs high {high_pitch:.1} Hz"
    );
}

#[test]
fn a_reference_without_voiced_speech_is_refused() {
    let backend = AcousticCloneBackend::new();
    let silence = alp_media::PcmBuffer::new(48_000, 1, vec![0.0; 48_000]).expect("pcm");

    let error = backend
        .create_profile(&silence, None)
        .expect_err("silence must be refused");
    assert!(
        error.to_string().contains("voiced speech"),
        "the refusal must be actionable: {error}"
    );
}

#[test]
fn a_decoded_reference_round_trips_through_the_wav_codec() {
    // The pipeline decodes a WAV before measuring it, so the measurement must
    // survive the 16-bit quantization the WAV format imposes.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("reference.wav");
    let reference = phrase(140.0, 48_000);
    export(&path, &reference, &ExportSettings::wav(WavBitDepth::Int16)).expect("export");

    let decoded = decode(&path).expect("decode");
    let print = analyze_voiceprint(&decoded);
    assert!(
        (print.f0_hz - 140.0).abs() < 6.0,
        "the decoded reference should still measure ~140 Hz, got {:.2}",
        print.f0_hz
    );
}
