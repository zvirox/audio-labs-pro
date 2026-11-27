# `alp-infer` / `demucs`

## Purpose

Four-stem audio separation using either an explicit `demucs.json` ONNX manifest or the catalog-downloaded MIT-licensed HTDemucs ONNX graph (`StemSplitio/htdemucs-onnx`).

## Public API

- `MODEL_ID = "demucs-4stem"`, `MANIFEST_FILE = "demucs.json"`, `STEM_NAMES = ["vocals", "drums", "bass", "other"]`.
- `DemucsManifest { graph, input_name, output_names, packed_output, segment_frames, overlap, sample_rate, stereo }`.
- `DemucsEngine::{load(model_dir), from_environment(), manifest(), separate(&PcmBuffer)}`.

## Data flow

1. Load `demucs.json`, or auto-synthesize `default_htdemucs_manifest()` when `htdemucs_fp16weights.onnx` / `htdemucs.onnx` is present from the `ModelManager` download.
2. Validate the ONNX graph's input and single packed `[1, 4, 2, N]` or 4 discrete `[1, C, N]` outputs using the shared runtime environment.
3. Resample input to the manifest rate (e.g. 44.1 kHz) and execute either discrete inference or overlap-add chunked inference (`separate_packed`) across `segment_frames`.
4. Map stems to canonical `STEM_NAMES` order (`["vocals", "drums", "bass", "other"]`) and resample back to the caller's sample rate.

## Allowed dependencies

`alp-media` (PCM + resample), `alp-voice` (`VoiceError`), `ort` (`2.0.0-rc.10`) with the shared `style_tts2` runtime environment, and `serde` for the manifest. No new internal dependency edge.

## Threading / real-time notes

The ONNX session sits behind a mutex because the daemon serves clients sequentially. Loading and separation are CPU-bound and must never run on the audio callback.

## Error handling

Missing manifests without recognized HTDemucs weights, absent graphs, tensor-name mismatches, and invalid output shapes return `VoiceError::Backend` with the offending path/name. `from_environment` returns `None` when `ALP_DEMUCS_DIR` is unset.

## Testing

Unit tests assert missing manifest and zero sample-rate rejection, default HTDemucs manifest synthesis for catalog downloads, and overlap-add stereo stem extraction. Model weights are downloaded on demand via `demucs-4stem` rather than bundled in the binary.

## Known limits

- Model weights are not bundled inside the repository (165.6 MB download via `ModelManager` / `demucs-4stem` manifest).
- Packed overlap-add uses fixed segment length (`343_980` frames for HTDemucs) and overlap ratio from the manifest.
- The daemon protocol exposes `SeparateStems` / `Stems`; callers must load the model first.
