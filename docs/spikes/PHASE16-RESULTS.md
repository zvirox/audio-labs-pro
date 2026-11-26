# Phase 16 Results — Neural Stem Separation & Video Dubbing Pipeline

Date: 2026-11-26

## Deliverables

- `alp-media::time_stretch`: pitch-preserving WSOLA duration change.
- `alp-voice::rate_fitting`: `FitWindow`, `FitPlan`, `plan_fit`, `fit_take`; TTS speed first, WSOLA fallback.
- `alp-media::video_sync`: `VideoRemuxer` with `ffprobe` probing, audio extraction, and `-c:v copy` remux via explicit ffmpeg argument vectors.
- `alp-infer::demucs`: model-gated four-stem ONNX runner driven by a `demucs.json` manifest, wired into the daemon protocol (`SeparateStems`/`Stems`) and `InferClient::separate_stems`.
- `alp-session::dubbing`: IPC bridge for stem separation, rate fitting, and remux; Tauri commands `dubbing_separate_stems`, `dubbing_fit_rate`, `dubbing_remux`.
- New `DubbingWorkspace` React workspace replacing the previous placeholder, with locked Linear UI components only.

## Dependencies

No new Rust crates. FFmpeg/ffprobe are external runtime tools (LGPL/GPL build-dependent), recorded in `THIRD_PARTY.md`; the app does not redistribute them. No Demucs weights are bundled.

## Verification

`cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check`, all four repository guards, `check:linear`, frontend build/test passed. The Demucs tests verify manifest validation and honest errors; separation itself is model-gated and was **not** executed because no compatible weights are installed.

## Known limits

- Stem separation requires a compatible ONNX export plus `ALP_DEMUCS_DIR` / `LoadModel demucs-4stem`.
- The runner expects NCT `[1, C, N]` tensors with manifest-declared names.
- Remux requires system FFmpeg and always stream-copies video; subtitles are side files.
- WSOLA artefacts increase with extreme stretch ratios; review fitted takes in the UI.

## Acceptance harness

`bash scripts/phase16-demo.sh`
