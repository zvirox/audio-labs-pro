# `alp-infer` / `style_tts2`

## Purpose

Shared mechanics for every `style_text_to_speech_2`-family neural engine
(ADR 0023): vocabulary parsing, IPA-to-token mapping with 0 padding, style-row
selection, ONNX session bootstrap with tensor-signature validation, the full
chunked synthesis flow (ADR 0024), and waveform concatenation. `kokoro.rs` and
`kitten.rs` keep only model facts on top of this layer.

## Public API

- `parse_vocab(path)`: character vocabulary from `tokenizer.json`
  (`model.vocab`; accepts Kokoro's export and Kitten's HF `tokenizers` file)
- `token_ids(vocab, ipa)`: ids 0-padded at both ends (`$` is id 0); unknown
  characters are reported, not fatal
- `style_row(bin, token_count)`: one 256-lane f32 vector, clamped to the last
  row (Kokoro's length buckets; Kitten's single row always resolves to row 0)
- `voice_path(dir, id, table)` / `list_voices(dir, table)`: stem, alias,
  tail/contains, then canonical (letters+digits) resolution so manifest
  display names like `Expr Voice 2 (F)` match `expr-voice-2-f.bin`
- `NeuralSession`: `load(path, env_name, required_inputs)` with signature
  validation, `has_input`, `synthesize_tokens(ids, style, speed)`
- `synthesize_text(session, phonemizer, vocab, request, sample_rate,
  log_prefix, style_for)`: normalize, chunk (ADR 0024), phonemize, run each
  chunk, concatenate with 0.15 s gaps
- `concatenate_chunks(rate, waveforms)`: gap semantics pinned by tests
  (empty chunks never produce gaps)
- `phonemize(phonemizer, text)`: G2P front-end call shared by both engines

## Data flow

1. Each engine's `TtsBackend::synthesize` calls `synthesize_text` with its
   own style resolver and sample rate.
2. Normalized text is split by `TextNormalizer::sentence_chunks`
   (320-character budget) into text chunks; each chunk is phonemized,
   tokenized (skipped characters accumulated across chunks), styled, and run
   through `NeuralSession::synthesize_tokens`.
3. Non-empty chunk waveforms are concatenated with 0.15 s silences into one
   `PcmBuffer`; chunk count > 1 is logged.

## Allowed dependencies

- `alp-media` (`PcmBuffer`), `alp-voice` (`TextNormalizer`, `TtsRequest`,
  `VoiceError`, `VoiceInfo`, `VoiceKind`)
- `ort` (pinned `2.0.0-rc.15`), `piper-plus-g2p` (MIT; bundled CMUdict per
  ADR 0010), `serde_json` (vocabulary parsing), standard library

## Threading / real-time notes

Runs only inside the `alp-infer` worker, never in an audio callback. The ONNX
session requires `&mut` for `run`, so `NeuralSession` holds it behind a
mutex; the daemon serves connections sequentially, so the lock is
uncontended, and the session stays idle between requests. Chunking is serial
per request.

## Error handling

Missing vocabulary or style files surface as `VoiceError::Backend` /
`UnknownVoice` from the resolvers; wrong tensor signatures are load-time
errors naming the found inputs. Text with no speakable phonemes after all
chunks is an error listing what was skipped; inference failures propagate as
`VoiceError::Backend` with the ORT message.

## Testing

Unit tests cover both tokenizer layouts, 0-padding and skipped-character
reporting, row selection incl. the single-row (Kitten) case, the canonical
voice pass, gap/concatenation semantics, and (via the engines) real-model
smoke plus chunked long-text reconstruction tests that skip when models are
absent.

## Known limits

- The chunk budget (320 characters) and gap (0.15 s) are fixed constants;
  they are not user-tunable.
- Cancel-between-chunks is future work; a long request runs all chunks once
  started.
- The canonical voice pass only matches ASCII letters/digits; catalogue
  names relying on other characters need table aliases.
