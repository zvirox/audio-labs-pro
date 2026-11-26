# `alp-infer` / `demucs`

## Purpose

Model-gated four-stem separation using a user-supplied Demucs-style ONNX export. The exact graph and tensor names come from a `demucs.json` manifest, so the runner does not assume a particular public export.

## Public API

- `MODEL_ID = "demucs-4stem"`, `MANIFEST_FILE = "demucs.json"`, `STEM_NAMES = ["vocals", "drums", "bass", "other"]`.
- `DemucsManifest { graph, input_name, output_names, sample_rate, stereo }`.
- `DemucsEngine::{load(model_dir), from_environment(), manifest(), separate(&PcmBuffer)}`.

## Data flow

1. Load and validate `demucs.json`, then open the ONNX graph with the shared runtime environment.
2. Refuse graphs missing the declared input or any of the four declared outputs.
3. Resample input to the manifest rate, transpose to `[1, C, N]`, run inference.
4. Read the four outputs, resample back to the caller's rate, and return them in `STEM_NAMES` order.

## Allowed dependencies

`alp-media` (PCM + resample), `alp-voice` (`VoiceError`), `ort` with the shared `style_tts2` runtime environment, and `serde` for the manifest. No new internal dependency edge.

## Threading / real-time notes

The ONNX session sits behind a mutex because the daemon serves clients sequentially. Loading and separation are CPU-bound and must never run on the audio callback.

## Error handling

Missing manifests, absent graphs, tensor-name mismatches, and invalid output shapes return `VoiceError::Backend` with the offending path/name. `from_environment` returns `None` when `ALP_DEMUCS_DIR` is unset.

## Testing

Unit tests assert that a missing manifest and a zero sample-rate manifest are reported as backend errors. There is no bundled model, so no separation test runs in CI; a compatible model must be supplied via `ALP_DEMUCS_DIR` and loaded with `LoadModel demucs-4stem`.

## Known limits

- No model weights are bundled; separation is unavailable until a compatible export is provided.
- Input layout is fixed to `[1, C, N]` (NCT) and outputs must use the same layout.
- Output channel counts are clamped to the requested mono/stereo shape.
- The daemon protocol exposes `SeparateStems` / `Stems`; callers must load the model first.
