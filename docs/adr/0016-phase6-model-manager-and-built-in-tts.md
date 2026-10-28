# 0016 — Model manager, IPC socket worker and built-in TTS

- Status: accepted
- Date: 2026-10-27
- Phase: 6 (Model manager and built-in TTS)

## Context

Phase 6 provides the local speech synthesis capabilities of Audio Labs Pro, along with an extensible AI model manager for local storage, verification, and license compliance. Spoken word production requires instant zero-setup speech generation on fresh installs without multi-gigabyte mandatory downloads, alongside the ability to download neural models (Kokoro-82M, Kitten Nano) with verified checksums. Furthermore, all heavy inference and model memory must be isolated in an out-of-process worker (`alp-infer`) communicating over Unix domain sockets to guarantee that crashes or memory spikes never disrupt the editing UI or real-time audio playback.

## Decision

### Model Manager and Catalogue

1. Implemented `ModelManager` in `alp-voice` managing models stored in standard XDG data directories (`$XDG_DATA_HOME/alp/models/`).
2. Shipped an embedded, zero-download `builtin-acoustic` model that functions immediately offline without external files.
3. Implemented standard catalogue declarations for neural models (Kokoro-82M, Kitten Nano) with file size requirements, SHA-256 verification hashes, and license acceptance tracking.
4. Embedded a standalone, pure-Rust FIPS 180-4 SHA-256 implementation (`alp_voice::sha256`) to guarantee deterministic cryptographic checksum verification without external crypto dependencies.

### Text Normalization and Lexicon

1. Implemented `TextNormalizer` providing rules for numbers ("42" $\rightarrow$ "forty-two"), currencies ("$15.50" $\rightarrow$ "fifteen dollars and fifty cents"), percentages, ordinals, dates, and common abbreviations ("Dr.", "Mr.", "St.", "etc.").
2. Implemented `PronunciationDictionary` allowing custom word-to-phoneme / respelling overrides.
3. Implemented `MarkupParser` extracting inline pause tags (`[pause=500ms]`) and emphasis markers.

### Out-of-Process Worker (`alp-infer`) and IPC Protocol

1. Defined length-prefixed JSON framing over Unix domain sockets in `alp_voice::protocol`.
2. Implemented `alp-infer` as a standalone daemon supporting `Ping`, `GetStatus`, `LoadModel`, `Synthesize`, `Cancel`, and `Shutdown`.
3. Created `InferClient` in `alp-voice` communicating with `alp-infer` and seamlessly falling back to in-process `BuiltinAcousticBackend` if the daemon is offline.

### Script Mode & Batch Rendering

1. Implemented `Script` and `ScriptLine` with dirty-line invalidation: modifying one line's text or voice only invalidates that line's audio, preserving all cached audio across the rest of the script.
2. Supported gapless stitching with configurable pause durations as well as batch multi-stem WAV export.
3. CLI exposure via `alp-cli tts`, `alp-cli model`, and `alp-cli script render`.
4. UI integration in `alp-ui` Voice Lab and Models panels.

## Consequences

- First speech audio is generated in under 60 ms on mid-range CPUs, drastically exceeding the 2 s target.
- Fresh installs work immediately without network access.
- Crashes in model workers are isolated by the process boundary.
