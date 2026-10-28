# 0018 — Voice Profiles, Cloning Safety, Voice Design, and Package Bundles

- Status: accepted
- Date: 2026-10-27
- Phase: 8 (Voice profiles, cloning, and voice generation)

## Context

Phase 8 introduces reusable voice profiles, natural language voice design, reference audio quality auditing, side-by-side acoustic comparison, and portable `.alpvoice` bundling into Audio Labs Pro. Creators need a global voice library independent of specific projects, prebuilt profiles wrapping built-in voices, and objective checks when recording reference clips for voice cloning. Crucially, ethical and safety principles require an enforced, human-only consent gate for any voice cloning operation: automated agents, scripts, and headless APIs must be prohibited from synthesizing or bypassing speaker authorization.

## Decision

### Voice Profile System and `.alpvoice` Bundles

1. Implemented `ProfileStore` in `alp-voice::profile` storing profiles in `$XDG_DATA_HOME/alp/voices/<id>/`.
2. Created `VoiceProfile` containing parameters (`VoiceParams`: speed, pitch, energy, emotion), relative reference audio paths, cached speaker embeddings, and tags.
3. Automatically seeded four prebuilt system profiles on initial run ("Aura", "Atlas", "Echo", "Sol").
4. Implemented self-contained, zero-dependency ZIP archive writer and reader in `alp-voice::zip` (PKWARE method 0 STORED and IEEE 802.3 CRC-32) to import and export `.alpvoice` profile packages interoperable with standard system archive tools.

### Mandatory Human Consent Gate

1. Defined `ConsentRecord` requiring `speaker_name`, `attested_by`, `timestamp`, and an explicit authorization statement.
2. Enforced at the store level: `ProfileStore::save_profile` and `import_bundle` strictly reject saving any `VoiceKind::Cloned` profile lacking a valid `ConsentRecord`.
3. Designed the UI in `alp-ui` to require manual user interaction to review, sign, and confirm consent before a clone profile can be committed.

### Reference Audio Quality Gate

1. Implemented `analyze_reference_quality` in `alp-voice::reference`, evaluating:
   - Minimum duration ($\ge 5.0$ seconds).
   - Peak headroom and digital clipping spike detection ($\ge 0.999$).
   - Sample rate standards ($\ge 24,000$ Hz).
   - Background noise floor ceiling ($-35.0$ dBFS).
   - Silence ratio ceiling ($\le 60\%$).
2. Produces structured `ReferenceQualityReport` with actionable diagnostic advice.

### Voice Design and Generation Cache

1. Implemented `design_voice` in `alp-voice::design`, synthesizing voice profiles with tailored pitch, speed, energy, and emotion from natural language briefs.
2. Implemented `VoiceGenerationCache` in `alp-voice::cache` deduplicating speech synthesis via SHA-256 digests over `(profile_id, text, params)`.
3. Implemented `compare_audio` in `alp-voice::compare` measuring duration, RMS delta, zero-crossing similarity, and acoustic similarity score between reference and generated audio.
4. Added system hardware detection in `alp-voice::hardware` probing RAM, CPU cores, and GPU accelerators to recommend "Fast" vs "Best" model tiers and warn before heavy allocations.

### CLI and UI Integration

1. Implemented `alp-cli profile (list | info | check-reference | design | export | import | compare | delete)` and `alp-cli hardware`.
2. Enhanced `alp-ui` Voice Lab with profile dropdown, reference quality audit feedback, consent attestation dialog, voice design inputs, and hardware model recommendations.

## Consequences

- Voice profiles can be shared and transferred across workstations via portable `.alpvoice` files.
- Cloning cannot occur without recorded human attestation.
- Creators receive instant actionable feedback on reference recording quality.
- Redundant generation of identical script lines is eliminated by the cryptographic cache.
