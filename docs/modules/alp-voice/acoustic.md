# `alp-voice` / `acoustic`

## Purpose

The **acoustic clone tier**: speech synthesis re-coloured by a measured
voiceprint.

This is the lightweight end of the cloning spectrum. It does not reproduce a
person's identity — it reproduces their *timbre*: pitch, intonation range and
the shape of their spectral envelope. The voiceprint comes from real signal
analysis of the reference recording
([`voiceprint`](./voiceprint.md)), and the synthesized harmonic stack is
shaped by the measured per-band gains, so the output genuinely tracks the
reference instead of imitating it by ear.

The engine is dependency-free and runs at 48 kHz. It is the tier that keeps
the application useful on a machine with no neural model installed, and it is
the tier the heavy neural clone backend (`qwen3-tts`) will replace.

## Public API

- `AcousticCloneBackend`: `new` (48 kHz), `with_sample_rate`, `Default`.
- `TtsBackend` implementation:
  - `id() -> "acoustic-clone"`.
  - `voices()`: one `Cloned` voice descriptor.
  - `synthesize(request)`: renders with the neutral voiceprint.
- `CloneBackend` implementation:
  - `create_profile(reference, transcript)`: measures the reference's
    voiceprint and returns a `VoiceProfile` of kind `Cloned` whose
    `cached_embedding` holds the voiceprint encoding. The consent record is
    always `None` — the engine never fabricates one.
  - `synthesize_with(profile, request)`: renders the request with the
    profile's stored voiceprint and generation parameters.
- Constants: `MODEL_ID = "acoustic-clone"`, `SAMPLE_RATE = 48_000`.

## Data flow

1. **Profile creation.** The reference is decoded, then measured by
   `analyze_voiceprint`. A reference with no voiced frames is refused with
   guidance ("record a clear spoken phrase"); an empty reference is refused
   as well. The resulting profile stores the voiceprint in
   `cached_embedding` and carries **no** consent record — the human-only
   consent gate lives in `ProfileStore` and the UI dialog.
2. **Rendering.** The voiceprint is decoded from the profile embedding. The
   fundamental is `f0_hz` scaled by the profile's pitch parameter; a small
   jitter derived from `f0_spread` keeps the result from sounding like a pure
   tone.
3. **Per-phoneme envelope.** Text is normalized with `TextNormalizer` and
   split into words and characters. Each alphabetic character maps to a
   formant target (`a`, `e`, `i`, `o`, `u`, and a generic voiced fallback;
   `s`, `f`, `t`, `k`, `p` are unvoiced).
4. **Harmonic stack.** Up to 48 harmonics are generated per voiced phoneme.
   Each harmonic's weight is the product of:
   - a three-formant Lorentzian resonance envelope (F1/F2/F3 with bandwidths
     of 80/110/160 Hz and weights 1.0/0.55/0.3),
   - the voiceprint's measured band gain at that harmonic's frequency
     (`VoicePrint::gain_at`), and
   - a `1/k` glottal rolloff.
5. **Amplitude shaping.** Each phoneme has a 15 % attack and 15 % release
   envelope; unvoiced phonemes use shaped noise instead of the harmonic
   stack. Word boundaries insert a 60 ms gap.

## Allowed dependencies

- `alp-media`: `PcmBuffer`
- `crate::profile`: `VoiceProfile`, `VoiceParams`
- `crate::text::TextNormalizer`
- `crate::voiceprint`: `VoicePrint`, `analyze_voiceprint`
- No external crates: the synthesis model is implemented directly.

## Threading / real-time notes

Pure computation with no shared state; safe to call from any worker thread.
It is never called from the real-time audio callback — the GUI runs it on its
job queue and the CLI runs it inline. Rendering is O(text length x phonemes x
harmonics), so long texts should be chunked (ADR 0024).

## Error handling

- An empty reference or one with no voiced speech is refused with an
  actionable `VoiceError::Backend` message.
- A malformed or missing embedding decodes to the neutral voiceprint rather
  than failing, so older profiles still speak.
- Text normalization failures and empty text produce a short silent buffer
  rather than an error.

## Testing

Unit tests in `acoustic.rs` verify that:

- a profile is created from a reference recording, carries an embedding, and
  carries **no** consent record;
- an empty reference and a silent reference are both refused with guidance;
- the clone reproduces the reference's pitch: re-analyzing the rendered
  waveform with the same tracker used on references lands near the
  reference's fundamental, and below the neutral voice's;
- synthesis produces audible, non-silent 48 kHz mono audio;
- the voiceprint stored in the profile matches a direct measurement of the
  source recording.

## Known limits

- This is a **timbre** clone, not an identity clone: it does not reproduce a
  speaker's accent, phrasing, or prosody.
- The synthesis model is a harmonic stack with a static formant map, so the
  result is intelligible but clearly synthetic compared to a neural voice.
- Only English vowels have dedicated formant targets; other languages fall
  back to a generic voiced vowel.
- The voiceprint's contribution is pitch and spectral shape; the "brightness"
  and "tilt" statistics are recorded in the profile but do not yet drive the
  synthesis independently.
