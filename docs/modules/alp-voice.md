# `alp-voice`

## Purpose

Defines speech synthesis (TTS), speech-to-text (STT), and voice profile traits; owns the model catalogue, download manager, SHA-256 verification, Unix socket IPC protocol, text normalization, pronunciation dictionary, and multi-line script mode.

## Public API

- `VoiceError`, `VoiceInfo`, `VoiceKind`, `VoiceProfile`, `TtsRequest`, `TranscriptSegment`
- `VoiceParams`, `ConsentRecord`, `ProfileStore`: profile management, legal consent safety, and `.alpvoice` bundling
- `ReferenceQualityReport`, `analyze_reference_quality`: reference audio acoustics and quality validation
- `VoiceDesignRequest`, `design_voice`: natural language voice prompt synthesis
- `VoiceGenerationCache`: cryptographic SHA-256 generation deduplication
- `VoiceComparison`, `compare_audio`: objective side-by-side acoustic comparison
- `HardwareProfile`, `ModelTier`: system hardware detection and model recommendations
- `TtsBackend`, `CloneBackend`, `SttBackend`
- `VadConfig`, `VoiceActivityDetector`, `segment_speech`
- `Transcript`, `to_plain_text`, `to_srt`, `to_vtt`, `parse_srt`, `cut_range_for_segment`
- `DictationSession`, `DictationEvent`, `BuiltinSttBackend`
- `ModelManager`, `ModelManagerConfig`: catalogue management, verification, license acceptance
- `ModelManifest`, `ModelFile`, `ModelId`, `ModelStatus`, `standard_catalogue`
- `TextNormalizer`: text expansion for numbers, currencies, dates, and abbreviations
- `PronunciationDictionary`, `MarkupParser`, `SpeechToken`: pronunciation overrides and pause tags
- `Script`, `ScriptLine`, `LineStatus`: script mode data structures, line regeneration, and stitching
- `InferClient`, `BuiltinAcousticBackend`: IPC client and in-process acoustic synthesizer
- `InferRequest`, `InferResponse`, `default_socket_path`: IPC protocol

## Data flow

1. The UI or CLI constructs a `TtsRequest` or `Script`.
2. Text is normalized via `TextNormalizer` and custom pronunciations applied via `PronunciationDictionary`.
3. Speech is synthesized through `InferClient` connecting to `alp-infer` over a Unix socket, or locally via `BuiltinAcousticBackend`.
4. Audio returns across the boundary as a `PcmBuffer`.

## Allowed dependencies

- `alp-core`, `alp-media`
- `dirs`, `serde`, `serde_json`, `thiserror`, `uuid`
- `ureq` (rustls, gzip), `fs4` (sync) for model downloads - ADR 0022
- Standard library

## Threading / real-time notes

All synthesis and model management operations run on background worker threads or CLI execution paths. They are never invoked on the real-time audio callback.

## Error handling

`VoiceError` distinguishes unknown voices, unavailable backends, and backend failures. IPC errors are represented as `CoreError`.

## Testing

Comprehensive unit tests across `client`, `lexicon`, `manager`, `protocol`, `script`, `sha256`, and `text` verify synthesis output, hash correctness, text normalization, and IPC serialization.

## Known limits

- Heavy model neural synthesis runs via `alp-infer` when model weights are downloaded; built-in acoustic mode is available without downloads.
