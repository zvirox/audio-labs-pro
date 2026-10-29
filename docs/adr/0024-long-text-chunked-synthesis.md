# ADR 0024: Long-Text Synthesis via Sentence-Chunked Reconstruction

- **Status:** Accepted
- **Date:** 2026-10-29
- **Deciders:** Audio Labs Pro Engineering
- **Consulted:** Kokoro v1.0 style-row layout (510 length buckets), `TextNormalizer` sentence splitting, measured speech/text ratios

## Context and Problem Statement

The neural engines sent the whole request through one inference: a long paragraph phonemized into one giant token sequence, whose style vector was simply clamped to the last of Kokoro's 510 length-bucketed rows ("the token-count style-row clamp"). Past a few hundred tokens this degrades prosody, inflates per-request latency monotonically, and risks runtime blowups, while Kitten Nano, with its single style row, has no length adaptation at all. The status report listed "reconstructed chunked synthesis" as the open gap.

## Decision Drivers

1. **No content loss:** every word of the request must reach the waveform; truncation is unacceptable.
2. **Prosody:** each chunk's style vector should match *that chunk's* length bucket, not a distant overshoot.
3. **Determinism:** splitting rules must be pure text logic (testable without a model).
4. **Shared path:** both `style_text_to_speech_2` engines must get the behavior for free, since they share ADR 0023's layer.

## Considered Options

- **Option 1: Sentence-boundary chunking with silent-gap reconstruction (Chosen)**
  - `TextNormalizer::sentence_chunks(text, budget)` splits on sentences first (`split_sentences`, abbreviation-aware), then clause punctuation, then word boundaries; greedy packing merges short sentences so a budget of 320 characters rarely splits single sentences.
  - Each chunk is phonemized, tokenized, and given its own length-appropriate style vector; waveforms are concatenated with a 0.15 s silence between chunks.
  - A budget of 320 characters lands English IPA (about 1.5 chars/phoneme) around 200-240 tokens, inside the comfortable bucket range and typical latency.
- **Option 2: Chunk the IPA after one global phonemization**
  - Splits must respect phonemizer punctuation context; doing G2P per chunk keeps prosody marks local and avoids mid-phoneme cuts.
- **Option 3: Streaming crossfade instead of gaps**
  - Rejected: crossfading two independently generated StyleTTS2 waveforms produces audible artifacts at boundaries; a natural sentence pause is what the gap models anyway.
- **Option 4: Raise the clamp limit and accept degradation**
  - Rejected: that is the status quo; it is the bug, not the fix.

## Decision

1. `TextNormalizer::sentence_chunks` becomes the single chunk planner in `alp-voice`, with unit-tested rules for sentence, clause, word, and degenerate (single word larger than the budget) splits; empty and whitespace-only inputs yield no chunks.
2. `style_tts2::synthesize_text` (ADR 0023's shared flow) runs every chunk through phonemize, token-ids, style-row selection, and inference, and concatenates non-empty chunk waveforms with `CHUNK_GAP_SECS = 0.15` silence; empty chunks never produce gaps. Skipped-character reports and chunk counts are logged per engine prefix.
3. Both `KokoroEngine` and `KittenEngine` delegate their `TtsBackend::synthesize` to this flow; a short request takes exactly the old single-chunk path (one waveform, no gap).
4. Per-chunk style selection is the model's own: Kokoro picks the length bucket for the chunk; Kitten yields its single row.
5. The conscious degenerate case: a single word longer than the budget is emitted as-is rather than broken mid-word.

## Consequences

- **Positive:** audiobook-scale paragraphs render fully; per-chunk prosody matches chunk length; chunked runs are observable in daemon logs (`reconstructed N chunks`).
- **Positive:** duration scales linearly with text instead of quadratically-ish with sequence attention; measured 20-22 s of speech from a ~360-character paragraph through both engines.
- **Neutral:** 0.15 s of silence appears between chunks; for sentence boundaries this reads as natural phrasing, for clause/word fallback splits it is a pause where a speaker would breathe anyway.
- **Negative:** one request on long text performs several inferences serially; cancellation between chunks is not implemented (the daemon's per-request cancel leaves room for it later).

## Verification

- `sentence_chunks` unit tests pin sentence/clause/word splitting, budget rules, packing, and empty-input behavior.
- Engine-level tests with the downloaded models (skipped when absent) assert that a > 320-character paragraph yields > 12 s of 24 kHz mono audio, which is impossible under the old clamp-and-pray path without visibly truncated content.
- The concatenation helper's gap semantics (including the empty-chunk rule) are pinned by unit tests.
