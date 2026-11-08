# P1 (Phase 5 & Phase 7 Gap Closure) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

---

## 1. Overview & Objectives

This plan closes the two primary architectural gaps identified in the project status report:
1. **Phase 5 DSP Gap:** Implement neural noise suppression (**RNNoise** via `nnnoiseless` 0.5.2) and classical **Spectral Subtraction** with noise-profile capture in `alp-dsp`. Integrate the denoiser into `VoiceCleanup` and the `Effect` trait hierarchy.
2. **Phase 7 STT Gap:** Add **Word-Level Timestamp Alignment** to `TranscriptSegment` and the Moonshine speech-to-text engine in `alp-infer`, enabling token/word frame boundaries and unlocking precision filler-word detection and ripple cutting in `alp-api::smart_edit_ops`.

---

## 2. File Structure & Responsibilities

| File | Change | Responsibility |
|---|---|---|
| [`crates/alp-dsp/Cargo.toml`](file:///home/zvirox/Projects/audio-labs-pro/crates/alp-dsp/Cargo.toml) | Modify | Add `nnnoiseless = "0.5.2"` and `realfft = "3.5"` dependencies. |
| [`crates/alp-dsp/src/denoise.rs`](file:///home/zvirox/Projects/audio-labs-pro/crates/alp-dsp/src/denoise.rs) | Create | `RnnoiseDenoiser` (buffered 480-sample frames), `SpectralDenoiser` (noise-profile capture + spectral subtraction), `NoiseReduction` (`Effect` wrapper). |
| [`crates/alp-dsp/src/chain.rs`](file:///home/zvirox/Projects/audio-labs-pro/crates/alp-dsp/src/chain.rs) | Modify | Incorporate `NoiseReduction` into the `VoiceCleanup` preset chain. |
| [`crates/alp-dsp/src/lib.rs`](file:///home/zvirox/Projects/audio-labs-pro/crates/alp-dsp/src/lib.rs) | Modify | Re-export `RnnoiseDenoiser`, `SpectralDenoiser`, `NoiseReduction`, and `DenoiseMode`. |
| [`docs/modules/alp-dsp/denoise.md`](file:///home/zvirox/Projects/audio-labs-pro/docs/modules/alp-dsp/denoise.md) | Create | Module specification for `denoise.rs` satisfying `check-docs.sh`. |
| [`crates/alp-voice/src/lib.rs`](file:///home/zvirox/Projects/audio-labs-pro/crates/alp-voice/src/lib.rs) | Modify | Add `WordTimestamp` struct; add `words: Vec<WordTimestamp>` to `TranscriptSegment`. |
| [`crates/alp-voice/src/transcript.rs`](file:///home/zvirox/Projects/audio-labs-pro/crates/alp-voice/src/transcript.rs) | Modify | Add word-level query methods and frame-range helpers. |
| [`crates/alp-infer/src/moonshine.rs`](file:///home/zvirox/Projects/audio-labs-pro/crates/alp-infer/src/moonshine.rs) | Modify | Decode BPE tokens into words with speech-aligned frame boundaries (`WordTimestamp`). |
| [`crates/alp-api/src/smart_edit_ops.rs`](file:///home/zvirox/Projects/audio-labs-pro/crates/alp-api/src/smart_edit_ops.rs) | Modify | Implement `edit_remove_fillers` using `WordTimestamp` ranges. |
| [`docs/modules/alp-voice/transcript.md`](file:///home/zvirox/Projects/audio-labs-pro/docs/modules/alp-voice/transcript.md) | Modify | Document `WordTimestamp` additions. |
| [`docs/modules/alp-infer/moonshine.md`](file:///home/zvirox/Projects/audio-labs-pro/docs/modules/alp-infer/moonshine.md) | Modify | Document word-level timestamp emission. |

---

## 3. Tasks

### Task 1: Implement RNNoise & Spectral Denoiser in `alp-dsp`

- [x] **Step 1.1: Add dependencies to `crates/alp-dsp/Cargo.toml`**
  Add `nnnoiseless = "0.5.2"` and `realfft = "3.5.0"`.
- [x] **Step 1.2: Implement `crates/alp-dsp/src/denoise.rs`**
  - Implement `RnnoiseDenoiser`:
    - Maintains a per-channel `nnnoiseless::DenoiseState`.
    - Implements FIFO buffering so caller can pass arbitrary slice sizes while RNNoise processes native 480-sample blocks (10 ms at 48 kHz).
  - Implement `SpectralDenoiser`:
    - Maintains FFT plan (`realfft::RealFftPlanner`) with windowing (Hann).
    - `capture_noise_profile(&mut self, samples: &[f32])`: computes average noise magnitude spectrum.
    - `process_spectrum`: subtracts noise magnitude with over-subtraction factor $\alpha$ (e.g. 1.5) and spectral floor $\beta$ (e.g. 0.05).
  - Implement `NoiseReduction` wrapping both modes with `Effect` trait.
- [x] **Step 1.3: Add unit tests in `crates/alp-dsp/src/denoise.rs`**
  - Test `RnnoiseDenoiser` processes synthetic noise and reduces RMS power without panic.
  - Test `SpectralDenoiser` noise profile capture and attenuation.
  - Test chunk-size invariance (processing in 64, 128, 256, 512, 1024 sample buffers gives consistent output).
- [x] **Step 1.4: Integrate into `VoiceCleanup` in `crates/alp-dsp/src/chain.rs`**
  Insert `NoiseReduction` into the `VoiceCleanup` chain between high-pass filtering and de-humming.
- [x] **Step 1.5: Re-export from `crates/alp-dsp/src/lib.rs` and create `docs/modules/alp-dsp/denoise.md`**
  Ensure all 8 required sections are present.

---

### Task 2: Implement Word-Level Timestamp Alignment in `alp-voice` & `alp-infer`

- [x] **Step 2.1: Define `WordTimestamp` in `crates/alp-voice/src/lib.rs`**
  ```rust
  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub struct WordTimestamp {
      pub word: String,
      pub start_frame: u64,
      pub end_frame: u64,
      pub probability: f32,
  }
  ```
  Add `#[serde(default, skip_serializing_if = "Vec::is_empty")] pub words: Vec<WordTimestamp>` to `TranscriptSegment`.
- [x] **Step 2.2: Add word query methods in `crates/alp-voice/src/transcript.rs`**
  Add helper methods to find words by text or time range, and test serialization compatibility.
- [x] **Step 2.3: Implement word timestamp alignment in `crates/alp-infer/src/moonshine.rs`**
  - Map BPE tokens into words (recognizing `SPACE_MARK` `\u{2581}` boundaries and byte fallback).
  - Distribute token/word timestamps proportionally across detected speech energy intervals, producing monotonic non-overlapping frame bounds.
  - Populate `TranscriptSegment.words` in `MoonshineEngine::transcribe`.
- [x] **Step 2.4: Wire `WordTimestamp` into `edit_remove_fillers` in `crates/alp-api/src/smart_edit_ops.rs`**
  Connect transcript word timestamps to detect common filler words ("um", "uh", "like") and report cut boundaries.
- [x] **Step 2.5: Update module documentation**
  Update `docs/modules/alp-voice/transcript.md` and `docs/modules/alp-infer/moonshine.md`.

---

### Task 3: Workspace Verification & Quality Gates

- [x] **Step 3.1: Run all architecture guards**
  - `bash scripts/check-file-length.sh`
  - `bash scripts/check-docs.sh`
  - `bash scripts/check-deps.sh`
  - `bash scripts/check-ui.sh`
- [x] **Step 3.2: Run formatting and clippy gates**
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
- [x] **Step 3.3: Run full workspace test suite**
  - `cargo test --workspace`
