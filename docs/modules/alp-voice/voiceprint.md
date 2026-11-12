# `alp-voice` / `voiceprint`

## Purpose

Measures the *voiceprint* of a reference recording: the measurable description
of a speaker's timbre that the acoustic clone tier uses to re-colour
synthesized speech.

The descriptor is computed with real signal analysis, not a hand-tuned
approximation:

- **Fundamental frequency** — per-frame normalized autocorrelation over a
  16 kHz analysis signal. The per-frame estimates are combined into a median
  (pitch) and a relative spread (intonation range).
- **Long-term average spectrum (LTAS)** — the mean magnitude spectrum over all
  *voiced* frames, folded into 16 log-spaced bands between 100 Hz and 8 kHz.
  The band gains are the actual transfer function applied to the harmonic
  stack, so a clone reproduces the reference's timbre rather than a guess.
- **Brightness and tilt** — the spectral centroid relative to `f0`, and the
  slope of the average spectrum in dB per octave. These drive the excitation
  brightness and the amount of high-frequency energy.

The descriptor is stored inside the 256-dimensional
`VoiceProfile::cached_embedding` slot, so a cloned profile stays a plain,
portable profile JSON with no extra files.

## Public API

- `VoicePrint`: the descriptor — `f0_hz`, `f0_spread`, `band_gains`,
  `brightness`, `tilt`, `frames`.
  - `to_embedding() -> Vec<f32>`: encode into the 256-dimensional profile
    embedding. The layout is fixed, so a profile written by one build stays
    readable by the next.
  - `from_embedding(&[f32]) -> VoicePrint`: decode from a profile embedding.
    A missing or malformed embedding falls back to the neutral voiceprint
    rather than failing, so a profile created before this feature existed
    still speaks.
  - `gain_at(freq_hz) -> f32`: the interpolated band gain at a frequency, used
    to weight each harmonic of the synthesis stack.
- `analyze_voiceprint(&PcmBuffer) -> VoicePrint`: measure a reference
  recording.
- Constants: `BAND_COUNT = 16`, `EMBEDDING_DIM = 256`.

## Data flow

1. **Resample and downmix.** The buffer is resampled to 16 kHz
   (`alp_media::resample`) and downmixed to mono.
2. **Framing.** The signal is cut into 512-sample frames with a 256-sample
   hop (32 ms / 16 ms at 16 kHz).
3. **Pitch tracking.** Each frame is mean-removed and its fundamental is
   estimated by normalized autocorrelation:
   `r(lag) = sum(x[i]*x[i+lag]) / sqrt(sum(x[i]^2) * sum(x[i+lag]^2))`.
   Because a periodic signal correlates just as well at twice its period, the
   global maximum alone would report an octave too low; every local maximum
   is therefore collected and the *earliest* one reaching the global peak is
   taken. A parabolic fit around that peak refines the lag to sub-sample
   accuracy.
4. **Spectrum.** Voiced frames are transformed by a hand-written iterative
   radix-2 FFT (`Complex` is a small local type, so the crate needs no
   external math dependency) and their magnitudes are accumulated into a
   long-term average spectrum.
5. **Band gains.** The LTAS is folded into 16 log-spaced bands. Each band's
   level is normalized by the *geometric* mean of the populated bands, so the
   gains describe the shape of the spectrum rather than its loudness, and are
   clamped to `[0.05, 4.0]`.
6. **Aggregate statistics.** The median and the relative spread of the pitch
   estimates give `f0_hz` and `f0_spread`; the spectral centroid over `f0`
   gives `brightness`; the dB-per-octave slope between the lowest and highest
   populated bands gives `tilt`.
7. **Embedding.** `to_embedding` writes the descriptor into the fixed
   256-lane layout used by `VoiceProfile::cached_embedding`.

## Allowed dependencies

- `alp-media`: `PcmBuffer`, `resample`
- `serde`: `Serialize` / `Deserialize` for the descriptor itself
- Standard library only for the DSP (`std::f32::consts`, no `num-complex`)

## Threading / real-time notes

Pure computation over an owned buffer; no shared state and no I/O. It is
called from the UI's job worker and from CLI commands, never from the
real-time audio callback. The autocorrelation is O(frames x lag x frame),
which for a 10-second reference is a few million operations — fast enough to
run inline, but it belongs off the UI thread in interactive use.

## Error handling

The analyzer never fails: it returns the neutral voiceprint for audio that is
too short, silent, or contains no voiced frames (`frames == 0`). Callers that
require a usable voiceprint — `AcousticCloneBackend::create_profile` — check
`frames` and refuse with an actionable message.

## Testing

Unit tests in `voiceprint.rs` verify that:

- the pitch tracker recovers the fundamental of a synthetic vowel to within a
  few Hz;
- a higher reference voice measures higher than a lower one;
- the embedding round-trips the descriptor without drift;
- a missing embedding falls back to the neutral voiceprint;
- silence yields no voiceprint (`frames == 0`);
- band gains interpolate continuously and stay finite out of range;
- the hand-written FFT matches a naive DFT within a small tolerance.

## Known limits

- The pitch tracker assumes a single dominant fundamental. Very breathy or
  strongly creaky voices may report a shifted estimate.
- Only voiced frames contribute to the spectrum, so the descriptor describes
  the *voiced* spectrum; fricative energy is deliberately excluded.
- The analysis band tops out at 8 kHz, so very bright sibilance is not
  reproduced.
- The voiceprint describes timbre, not identity: it does not model accent,
  prosody patterns, or idiosyncratic delivery.
