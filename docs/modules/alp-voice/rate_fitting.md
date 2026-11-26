# `alp-voice` / `rate_fitting`

## Purpose

Speech-rate planning for dubbing. Chooses a TTS speed multiplier first and falls back to pitch-preserving WSOLA stretching when the required speed is outside the engine range.

## Public API

- `DEFAULT_MIN_SPEED = 0.5`, `DEFAULT_MAX_SPEED = 2.0`.
- `FitWindow { start_secs, end_secs }` with `duration_secs()`.
- `FitPlan { current_duration_secs, target_duration_secs, recommended_speed, stretch_ratio, uses_wsola, within_tts_range }`.
- `plan_fit(current, target, min_speed, max_speed) -> FitPlan`.
- `fit_take(&PcmBuffer, target_duration_secs) -> Result<(PcmBuffer, FitPlan), MediaError>`.

## Data flow

1. Compute the required speed as `current / target`.
2. If it lies in the engine range, recommend it and leave audio unchanged.
3. Otherwise clamp the speed and compute the residual WSOLA ratio `target / (current / speed)`.
4. `fit_take` applies that residual stretch via `alp_media::time_stretch` to an already-rendered take.

## Allowed dependencies

`alp-media` (`PcmBuffer`, `time_stretch`) and `serde` for the DTO. No new dependency edge.

## Threading / real-time notes

Pure offline planning plus an offline stretch. Safe on a job worker; never call from the audio callback.

## Error handling

Non-positive durations produce a neutral plan (`speed 1.0`, no stretch). `fit_take` propagates `MediaError` for invalid stretch ratios.

## Testing

Tests cover window duration, an in-range speed plan, an out-of-range plan requiring WSOLA, and duration convergence after fitting a one-second tone to 0.75 s.

## Known limits

- `fit_take` does not re-synthesise speech; `recommended_speed` is informational for a later TTS regeneration.
- WSOLA artefacts grow with extreme residual ratios; anything beyond the speed range should be reviewed in the Dubbing Studio.
