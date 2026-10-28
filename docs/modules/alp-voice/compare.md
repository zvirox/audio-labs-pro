# `alp-voice` / `compare`

## Purpose

Provides objective side-by-side acoustic comparison between reference voice recordings and generated speech takes (measuring duration, RMS level delta, zero-crossing rate, and composite similarity score).

## Public API

- `VoiceComparison`: `ref_duration_secs`, `gen_duration_secs`, `duration_diff_secs`, `ref_rms_dbfs`, `gen_rms_dbfs`, `level_diff_db`, `ref_zcr`, `gen_zcr`, `similarity_score`
- `compare_audio(reference: &PcmBuffer, generated: &PcmBuffer) -> VoiceComparison`

## Data flow

1. Accepts reference and generated `PcmBuffer` instances.
2. Extracts duration, RMS level, and zero-crossing rates.
3. Computes normalized acoustic distance metrics and similarity score (0.0 to 1.0).

## Allowed dependencies

- `alp-media::PcmBuffer`
- `serde`, Standard library

## Threading / real-time notes

Execution runs on background threads or CLI execution loops in $O(N)$ time.

## Error handling

Handles empty or silent buffers safely by returning default floors (-120 dBFS) without panicking.

## Testing

Unit tests in `compare.rs` test identical audio signals yielding expected duration matches, zero level difference, and similarity scores > 0.95.

## Known limits

- Time-domain and zero-crossing analysis; mel-frequency cepstral coefficient (MFCC) distance can be added.
