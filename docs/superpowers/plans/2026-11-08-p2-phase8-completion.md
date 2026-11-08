# P2 (Phase 8 Completion) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Phase 8 deliverables by packaging and integrating the Qwen3-TTS heavy cloning runtime and implementing the synthetic-audio watermark metadata header in media exports as committed in `docs/ETHICS.md`.

**Architecture:** Extend `AudioMetadata` in `alp-media` with provenance flags (`synthetic: bool`, `watermark: Option<String>`), embedding RIFF INFO tags (`ISFT`, `ICMT`) into WAV and Vorbis comments (`SYNTHETIC_VOICE=true`, `PROVENANCE=...`) into FLAC exports. Add `qwen3-tts` manifest to `alp-voice` catalogue and implement `Qwen3Engine` in `alp-infer` implementing `CloneBackend` (reference embedding extraction + conditioned synthesis) and `TtsBackend`, wired into `alp-infer` worker dispatch.

**Tech Stack:** Rust 2024 edition, `hound` (RIFF/WAV), `flacenc` (FLAC Vorbis comment blocks), `ort` (ONNX Runtime), `alp-media`, `alp-voice`, `alp-infer`, `alp-api`.

**Spec:** `docs/ETHICS.md`, `docs/adr/0008-inference-runtime.md`, `docs/adr/0018-phase8-voice-profiles-cloning-and-generation.md`.

## Global Constraints

- **Safe Rust:** `unsafe_code = "deny"` across the workspace. 100% safe Rust only.
- **Licensing:** Apache-2.0 and MIT only. Qwen3-TTS weights and architecture are Apache-2.0.
- **File Length Limit:** Hard limit of 1,500 lines per file enforced by `scripts/check-file-length.sh`.
- **Documentation:** Every Rust source module must have an 8-section module doc in `docs/modules/` enforced by `scripts/check-docs.sh`.
- **Ethics Rule:** Cloned profiles strictly require valid human `ConsentRecord`; no bypass allowed.

## Review Focus

1. **Synthetic flag presence in exports:** WAV and FLAC exports generated from synthetic speech must strictly carry `SYNTHETIC_VOICE=true` / `ISFT` / `ICMT` tags without corrupting existing audio data or container layout.
2. **Backwards compatibility for non-synthetic exports:** Standard exports (e.g. podcast mixes, clean audio recordings) where `synthetic: false` must not emit synthetic markers or pollute metadata.
3. **Missing model files handling:** Calling `Qwen3Engine::load` on a directory without model files must cleanly return `VoiceError::Unavailable` with user-friendly download instructions (`alp-cli model download qwen3-tts`) without panicking.
4. **Speaker embedding extraction:** `Qwen3Engine::extract_embedding` and `create_profile` must handle mono/stereo inputs and arbitrary sample rates gracefully, generating normalized embedding vectors.
5. **Ethics documentation sync:** `docs/ETHICS.md` section 1.3 must be updated to affirm that the synthetic watermark is now active and enforced.

---

### Task 1: Implement Synthetic-Audio Watermark Metadata Header in Exports

**Files:**
- Modify: `crates/alp-media/src/export.rs`
- Modify: `crates/alp-media/src/wav.rs`
- Modify: `crates/alp-api/src/voice.rs`
- Modify: `docs/ETHICS.md`
- Modify: `docs/modules/alp-media/export.md`

**Interfaces:**
- Produces:
  ```rust
  pub struct AudioMetadata {
      pub title: Option<String>,
      pub artist: Option<String>,
      pub album: Option<String>,
      pub comment: Option<String>,
      pub synthetic: bool,
      pub watermark: Option<String>,
  }
  ```
- Consumes: `hound`, `flacenc`, `write_atomic`

- [ ] **Step 1.1: Write failing unit tests for synthetic metadata in WAV and FLAC**

In `crates/alp-media/src/export.rs`:
```rust
#[test]
fn export_writes_synthetic_watermark_in_wav_and_flac() {
    let dir = tempfile::tempdir().expect("tempdir");
    let pcm = PcmBuffer::new(48_000, 1, vec![0.0; 4800]).expect("pcm");
    let mut metadata = AudioMetadata::default();
    metadata.synthetic = true;
    metadata.watermark = Some("Audio Labs Pro (qwen3-tts)".into());

    let wav_path = dir.path().join("synth.wav");
    let wav_settings = ExportSettings {
        format: ExportFormat::Wav,
        bit_depth: WavBitDepth::Int16,
        sample_rate: None,
        dither: DitherMode::None,
        metadata: metadata.clone(),
    };
    export(&wav_path, &pcm, &wav_settings).expect("export wav");
    let wav_bytes = std::fs::read(&wav_path).expect("read wav");
    let wav_str = String::from_utf8_lossy(&wav_bytes);
    assert!(wav_str.contains("ISFT"));
    assert!(wav_str.contains("Audio Labs Pro"));
    assert!(wav_str.contains("synthetic_voice: true"));

    let flac_path = dir.path().join("synth.flac");
    let flac_settings = ExportSettings {
        format: ExportFormat::Flac,
        bit_depth: WavBitDepth::Int16,
        sample_rate: None,
        dither: DitherMode::None,
        metadata,
    };
    export(&flac_path, &pcm, &flac_settings).expect("export flac");
    let flac_bytes = std::fs::read(&flac_path).expect("read flac");
    let flac_str = String::from_utf8_lossy(&flac_bytes);
    assert!(flac_str.contains("SYNTHETIC_VOICE=true"));
    assert!(flac_str.contains("PROVENANCE=Audio Labs Pro (qwen3-tts)"));
}
```

- [ ] **Step 1.2: Run test to verify it fails**

Run: `cargo test -p alp-media -- export_writes_synthetic_watermark_in_wav_and_flac`
Expected: FAIL (fields `synthetic` and `watermark` do not exist on `AudioMetadata`).

- [ ] **Step 1.3: Implement synthetic metadata in `crates/alp-media/src/export.rs` and `crates/alp-media/src/wav.rs`**

- Add `pub synthetic: bool` and `pub watermark: Option<String>` to `AudioMetadata`.
- Update `is_empty(&self) -> bool` to check `!self.synthetic && self.watermark.is_none()`.
- Update `vorbis_comments(&self)`:
  ```rust
  if self.synthetic {
      comments.push("SYNTHETIC_VOICE=true".to_string());
      let prov = self.watermark.as_deref().unwrap_or("Audio Labs Pro Synthesized Speech");
      comments.push(format!("PROVENANCE={prov}"));
  }
  ```
- In `crates/alp-media/src/export.rs` update `riff_tags(&self) -> Vec<(&'static [u8; 4], std::borrow::Cow<'_, str>)>`:
  - When `self.synthetic` is true, append:
    - `(b"ISFT", Cow::Borrowed("Audio Labs Pro (Synthetic Audio)"))`
    - In `b"ICMT"`: format with `synthetic_voice: true; provenance: {prov}`.
- Update `append_riff_info` in `crates/alp-media/src/wav.rs` to accept `Cow<'_, str>`.

- [ ] **Step 1.4: Update `crates/alp-api/src/voice.rs` to set synthetic watermark**

In `tts_generate`:
```rust
metadata: AudioMetadata {
    synthetic: true,
    watermark: Some(format!("Audio Labs Pro ({engine})")),
    comment: Some(format!("synthetic_voice: true; engine: {engine}")),
    ..AudioMetadata::default()
},
```

- [ ] **Step 1.5: Run tests to verify they pass**

Run: `cargo test -p alp-media -p alp-api`
Expected: PASS.

- [ ] **Step 1.6: Update documentation in `docs/ETHICS.md` and `docs/modules/alp-media/export.md`**

- In `docs/ETHICS.md`: update section 1.3 to note that the transparency & attribution watermark is implemented and active in WAV and FLAC exports.
- In `docs/modules/alp-media/export.md`: document `AudioMetadata.synthetic` and `AudioMetadata.watermark`.

---

### Task 2: Package and Integrate Qwen3-TTS Heavy Cloning Runtime

**Files:**
- Modify: `crates/alp-voice/src/model.rs`
- Create: `crates/alp-infer/src/qwen3.rs`
- Modify: `crates/alp-infer/src/main.rs`
- Modify: `THIRD_PARTY.md`
- Create: `docs/modules/alp-infer/qwen3.md`

**Interfaces:**
- Consumes: `alp-media::PcmBuffer`, `alp-voice::{CloneBackend, TtsBackend, TtsRequest, VoiceProfile, VoiceError}`
- Produces: `Qwen3Engine` in `alp-infer::qwen3`:
  ```rust
  pub const MODEL_ID: &str = "qwen3-tts";
  pub const SAMPLE_RATE: u32 = 24_000;

  pub struct Qwen3Engine {
      model_dir: PathBuf,
  }

  impl Qwen3Engine {
      pub fn load(model_dir: &Path) -> Result<Self, VoiceError>;
      pub fn extract_embedding(reference: &PcmBuffer) -> Vec<f32>;
  }

  impl TtsBackend for Qwen3Engine { ... }
  impl CloneBackend for Qwen3Engine { ... }
  ```

- [ ] **Step 2.1: Write failing unit test for `qwen3-tts` manifest and `Qwen3Engine`**

In `crates/alp-voice/src/model.rs`:
Assert that `standard_catalogue()` includes a manifest with id `qwen3-tts`, `ModelKind::VoiceClone`, and Apache-2.0 license.

In `crates/alp-infer/src/qwen3.rs`:
- Test that `Qwen3Engine::load` on an empty directory returns `VoiceError::Unavailable` mentioning `alp-cli model download qwen3-tts`.
- Test that `Qwen3Engine::extract_embedding` generates a normalized 256-f32 embedding from a `PcmBuffer`.
- Test `create_profile` generates a valid `VoiceProfile` of kind `VoiceKind::Cloned` with `cached_embedding`.
- Test `synthesize_with` generates valid 24 kHz audio.

- [ ] **Step 2.2: Run test to verify it fails**

Run: `cargo test -p alp-voice -- model::tests`
Expected: FAIL (manifest `qwen3-tts` not found).

- [ ] **Step 2.3: Add `qwen3-tts` to `standard_catalogue()` in `crates/alp-voice/src/model.rs`**

Add `ModelManifest` for `qwen3-tts`:
- id: `ModelId::new("qwen3-tts")`
- name: `"Qwen3-TTS 0.6B (Voice Clone)"`
- description: `"Heavyweight neural voice cloning model with prompt-conditioned speaker synthesis."`
- kind: `ModelKind::VoiceClone`
- author: `"Qwen / Alibaba Cloud"`
- version: `"1.0"`
- license: Apache-2.0
- files: `gguf/qwen3-tts-0.6b-q4_k_m.gguf`, `onnx/speech_decoder.onnx`, `tokenizer.json`, `config.json`

- [ ] **Step 2.4: Implement `crates/alp-infer/src/qwen3.rs`**

- Implement `Qwen3Engine`:
  - `load(model_dir)` validates file presence, returning `VoiceError::Unavailable` if files are absent.
  - `extract_embedding(reference)` computes speaker embedding vector (256 f32 values) using spectral centroid, RMS energy distribution, and zero-crossing moments.
  - Implement `TtsBackend`: `id() -> "qwen3-tts"`, `voices()`, `synthesize(&TtsRequest) -> Result<PcmBuffer, VoiceError>`.
  - Implement `CloneBackend`: `create_profile(&PcmBuffer, Option<&str>) -> Result<VoiceProfile, VoiceError>` and `synthesize_with(&VoiceProfile, &TtsRequest) -> Result<PcmBuffer, VoiceError>`.
  - Condition speech synthesis on the speaker embedding and `VoiceParams` (pitch, speed, energy).

- [ ] **Step 2.5: Wire `qwen3` in `crates/alp-infer/src/main.rs`**

- Add `mod qwen3;`
- In `Worker::load_model_from`:
  ```rust
  qwen3::MODEL_ID => {
      Qwen3Engine::load(model_dir).map(|engine| Box::new(engine) as Box<dyn TtsBackend>)
  }
  ```

- [ ] **Step 2.6: Update `THIRD_PARTY.md` and create `docs/modules/alp-infer/qwen3.md`**

- In `THIRD_PARTY.md`: change status of Qwen3-TTS 0.6B to `P8`.
- In `docs/modules/alp-infer/qwen3.md`: document `alp-infer::qwen3` with all 8 standard sections:
  1. `Purpose`
  2. `Public API`
  3. `Data flow`
  4. `Allowed dependencies`
  5. `Threading / real-time notes`
  6. `Error handling`
  7. `Testing`
  8. `Known limits`

- [ ] **Step 2.7: Run tests to verify they pass**

Run: `cargo test -p alp-voice -p alp-infer`
Expected: PASS.

---

### Task 3: Workspace Verification & Quality Gates

- [ ] **Step 3.1: Run all architecture guards**
  - `bash scripts/check-file-length.sh`
  - `bash scripts/check-docs.sh`
  - `bash scripts/check-deps.sh`
  - `bash scripts/check-ui.sh`
- [ ] **Step 3.2: Run formatting and clippy gates**
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
- [ ] **Step 3.3: Run full workspace test suite**
  - `cargo test --workspace`
