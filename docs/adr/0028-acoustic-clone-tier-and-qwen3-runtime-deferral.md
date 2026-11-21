# ADR 0028: The Acoustic Clone Tier, and Deferring the Qwen3-TTS Neural Runtime

- **Status:** Accepted for the acoustic clone tier; Qwen3-TTS runtime deferral superseded by ADR 0029
- **Date:** 2026-11-11
- **Deciders:** Audio Labs Pro Engineering
- **Consulted:** `Serveurperso/Qwen3-TTS-GGUF` (Apache-2.0 GGUF export), `Qwen/Qwen3-TTS-12Hz-0.6B-Base` (Apache-2.0), the GGUF talker header, `candle-transformers` 0.11 model list, ADR 0008 (inference worker), ADR 0018 (voice profiles and cloning), ADR 0025 (two-tier daemon)

## Context and Problem Statement

The master checklist's P0 goal is "make the model tier real": the heavy clone
tier (Qwen3-TTS 0.6B/1.7B) must sit behind the existing `CloneBackend`,
consent and profile plumbing rather than being a catalogue entry with no
runtime behind it.

Two facts block a direct implementation:

1. **The architecture is bespoke.** The Apache-2.0 GGUF talker export declares
   `general.architecture = "qwen3-tts"` with 478 tensors across three
   sub-models: `spk_enc` (a 6-block Res2Net speaker encoder at 24 kHz),
   `talker` (28 Qwen3 blocks, 1024 hidden, 16 heads / 8 KV heads, mRoPE with
   `mrope_section = [24, 20, 20]` interleaved, `position_id_per_seconds = 13`)
   and `code_pred` (5 blocks, vocabulary 2048). A separate 12 Hz speech-token
   decoder GGUF is required to turn the predicted codes into audio.
   `candle-transformers` 0.11 ships `quantized_qwen3` and `qwen3` — the plain
   Qwen3 LLM — but **no `qwen3-tts` model**. No upstream Rust implementation
   of this architecture exists.
2. **The upstream repository has no GGUF and no ONNX.** `Qwen/Qwen3-TTS-12Hz-0.6B-Base`
   ships only `model.safetensors` (1.83 GB) and
   `speech_tokenizer/model.safetensors` (682 MB). The previous manifest in
   `crates/alp-voice/src/model.rs` pointed at `onnx/speech_decoder.onnx` and a
   `tokenizer.json` that do not exist, with a placeholder SHA-256
   (`a1b2c3d4…`). That is a fabricated download — it would have failed at
   install time and could not have produced audio even if it had succeeded.

The consent gate, the profile store, the `.alpvoice` bundle format and the
daemon routing for clones all already exist and are exercised by tests. What
does not exist is an engine that can actually *render* a cloned profile: the
`CloneBackend` trait had no implementation, so a cloned profile fell back to
the plain built-in synthesis path and its speaker conditioning was silently
discarded.

## Decision Drivers

1. **Honesty above coverage.** No simulated or synthetic stand-in audio may be
   presented as neural output. An engine that cannot run must say so.
2. **A real, working clone tier now.** The tier must be reachable and usable
   on a machine with no model download at all, and it must genuinely derive
   its output from the reference recording.
3. **Manifest integrity.** Every URL, size and digest in a model manifest
   must be verifiable against the upstream LFS pointers.
4. **Preserve the consent gate.** The human-only gate must hold at every
   layer that can render a cloned voice, not only at profile creation.
5. **Keep the heavy tier's shape.** When the neural runtime lands it must
   slot into the same places, without a protocol or storage redesign.

## Considered Options

- **Option 1: Implement the `qwen3-tts` architecture in Rust now**
  - Rejected as a single change: it requires the Res2Net speaker encoder, a
    Qwen3 talker with interleaved mRoPE and a 13 Hz position scheme, the
    code predictor, and the 12 Hz codec decoder — a separate GGUF. This is a
    multi-phase project, not a patch, and guessing at the conventions without
    a reference implementation risks producing subtly wrong audio.
- **Option 2: Keep the old manifest and add a placeholder "reference-conditioned
    acoustic mode"** (what `qwen3.rs` claimed to do)
  - Rejected: the manifest was fabricated, and the engine described a
    "conditioned synthesis" path that did not exist. A profile claiming to be
    a Qwen3 clone that is really a formant synthesizer is exactly the
    dishonesty this decision is meant to remove.
- **Option 3: A real acoustic clone tier, with the heavy engine validating
  honestly and deferring execution (Chosen)**
  - The acoustic tier measures the reference (pitch, LTAS band gains) and
    renders speech whose spectrum is that measurement. The output is real
    analysis-driven audio, not a stand-in for a neural model.
  - The Qwen3 engine validates the installation for real (files present, GGUF
    headers parsed, architecture and talker depth checked) and reports
    `VoiceError::Unavailable` naming the missing runtime when it cannot
    execute.

## Decision

1. **New module `crates/alp-voice/src/voiceprint.rs`.** Real analysis of the
   reference recording:
   - 16 kHz resample, mono downmix, 512-sample frames with a 256-sample hop.
   - Normalized autocorrelation pitch tracking
     (`r(lag) = sum(x[i]·x[i+lag]) / sqrt(sum(x[i]²)·sum(x[i+lag]²))`).
     Because a periodic signal correlates just as well at twice its period,
     every local maximum is collected and the *earliest* one reaching the
     global peak is taken; a parabolic fit refines the lag to sub-sample
     accuracy.
   - A hand-written radix-2 FFT (with a local `Complex` type, so the crate
     needs no external math dependency) produces the long-term average
     spectrum over voiced frames, folded into 16 log-spaced bands between
     100 Hz and 8 kHz. The band levels are normalized by their geometric mean
     so the gains describe spectral *shape*, not loudness.
   - The descriptor is encoded into the existing 256-dimensional
     `VoiceProfile::cached_embedding` slot with a fixed layout, so a cloned
     profile stays a plain portable profile JSON.
2. **New module `crates/alp-voice/src/acoustic.rs`.** `AcousticCloneBackend`
   implements `CloneBackend`:
   - `create_profile` measures the reference and stores the voiceprint.
     Empty and unvoiced references are refused with actionable messages. The
     consent record is always `None` — the engine never fabricates one.
   - `synthesize_with` renders a 48 kHz harmonic stack: up to 48 harmonics per
     voiced phoneme, each weighted by a three-formant Lorentzian envelope
     times the voiceprint's measured band gain times a `1/k` glottal rolloff.
3. **The heavy engine validates and defers.** `crates/alp-infer/src/qwen3.rs`
   checks the installation for real (using the new dependency-free GGUF
   header reader in `crates/alp-infer/src/gguf.rs`) and returns
   `VoiceError::Unavailable` naming the missing runtime. It never produces
   audio.
4. **Corrected manifest.** `crates/alp-voice/src/model.rs` now points at the
   Apache-2.0 community export (`Serveurperso/Qwen3-TTS-GGUF`) and the
   upstream text-side tokenizer files, with every size and SHA-256 read back
   from the upstream LFS pointers rather than guessed.
5. **A third daemon slot.** `alp-infer`'s `Worker` gains a `CloneEngine`
   slot beside the synthesis and transcription slots. Loading `qwen3-tts`
   populates it without disturbing the other two. A failed load keeps the
   acoustic tier active.
6. **A profile-conditioned request.** `InferRequest::SynthesizeWithProfile`
   carries the whole profile, because a cloned profile's speaker
   conditioning cannot be expressed by a voice id. `InferClient::synthesize_with_profile`
   sends it; the daemon enforces the consent gate when it receives one
   (a `Cloned` profile without consent is refused), and the in-process
   fallbacks in `alp-api` and `alp-ui` enforce the same rule so the guarantee
   does not depend on the daemon being reachable.
7. **The clone tier is reachable without the GUI.** `alp-cli tts --profile <id>`
   renders any stored profile, including a cloned one, with the same
   daemon-first routing and the acoustic fallback. *Creating* a cloned
   profile stays in the GUI only: the build plan requires the consent
   attestation to be human-only, and the CLI must not satisfy it.
8. **One source for the synthetic-voice tag.** `AudioMetadata::synthetic_speech(engine)`
   builds the provenance metadata, and every TTS export path (API, CLI, UI)
   uses it. The MCP `export_audio` tool exposes no metadata flags, so the tag
   still cannot be disabled through MCP.

## Consequences

**Positive**

- A cloned profile now renders with real conditioning derived from the
  reference, on a machine with nothing downloaded.
- The clone path is honest end to end: the acoustic tier is named as the
  acoustic tier, and the neural tier reports that it cannot execute.
- The fabricated manifest is gone; every digest is verifiable.
- The consent gate is enforced at render time in three independent layers
  (daemon, API fallback, UI fallback), not just at profile creation.
- The heavy engine's future implementation slots into a protocol and storage
  shape that already exists and is tested.

**Negative / follow-ups**

- The acoustic tier reproduces **timbre, not identity**. It does not model
  accent, phrasing or prosody, and the result is clearly synthetic next to a
  neural voice. The UI and CLI name it `acoustic-clone` so the distinction is
  never blurred.
- `Qwen3Engine::extract_embedding` currently returns the acoustic
  voiceprint. It is format-compatible with the embedding slot but is not a
  neural speaker embedding; the neural runtime will replace it.
- The `brightness` and `tilt` statistics are recorded in the voiceprint but
  do not yet drive the synthesis independently of the band gains.
- Implementing the neural runtime remains open work: it needs the
  `qwen3-tts` architecture and the 12 Hz codec decoder in Rust.

## References

- `crates/alp-voice/src/voiceprint.rs` — the measurement
- `crates/alp-voice/src/acoustic.rs` — the clone tier
- `crates/alp-voice/src/protocol.rs` — `SynthesizeWithProfile`
- `crates/alp-voice/src/client.rs` — `synthesize_with_profile`
- `crates/alp-infer/src/qwen3.rs` — validation and the deferred runtime
- `crates/alp-infer/src/gguf.rs` — the GGUF header reader
- `crates/alp-infer/src/main.rs` — the clone daemon slot
- `crates/alp-api/src/voice.rs`, `crates/alp-ui/src/app/studio.rs` — the
  rendering paths
- `crates/alp-voice/src/model.rs` — the corrected manifest
