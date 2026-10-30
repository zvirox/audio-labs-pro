# ADR 0026: Phase 10 Voice Toolset over MCP

- **Status:** Accepted
- **Date:** 2026-10-29
- **Deciders:** Audio Labs Pro Engineering
- **Consulted:** ALP_BUILD_PLAN.md Section 10.3 tool table, `docs/mcp/SECURITY.md` voice-cloning invariant, ADR 0025 (two-tier daemon contract), ADR 0018 (voice profiles and design)

## Context and Problem Statement

Phase 7 delivered a two-tier daemon (neural synthesis and transcription coexisting, ADR 0025), but nothing in `alp-mcp` consumes it: the promised Phase 10 voice tools (`tts_voices`, `voice_profile_get`, `tts_generate`, `voice_design`, `transcribe`) have no tool surface, and the only `alp-voice` consumers are the UI and CLI. The MCP resources list also advertises `alp://project/active` without serving it (`resources/read` falls through to `-32602`), a broken promise noticed while mapping the surface.

Everything the tools need already exists behind `alp-voice`: `ProfileStore` (default store seeds builtins `aura`/`atlas`/`echo`/`sol`), `design_voice`, `InferClient`, `BuiltinAcousticBackend`, `BuiltinSttBackend`, and `Transcript` export. The work is the dependency seam, the tier mapping, and honest engine naming.

## Decision Drivers

1. **Dependency direction:** `scripts/check-deps.sh` permits `alp-mcp` to depend only on `{alp-api, alp-core}`. Voice-tool logic must therefore live in `alp-api` (which already depends on `alp-voice`); `alp-mcp` adds thin JSON arms only.
2. **Security invariant (hard):** creating or *importing* cloned voice profiles is never reachable through MCP at any tier. `voice_design` only ever produces `VoiceKind::Designed` profiles; listing or reading cloned profiles is allowed (Observe), creating them is not.
3. **Honesty:** tool results must name the engine that actually ran - daemon-first with built-in fallback mirrors `synthesize_via_tier` (UI) and `transcribe_capture` (Phase 7). The built-in STT keeps its placeholder-words admission wherever it runs.
4. **Real surface over promise:** tools that cannot do their job yet are deferred, not stubbed.

## Considered Options

- **Option 1: Five tools now, voice plumbing in `alp-api`, thin arms in `alp-mcp` (Chosen)** - respects `check-deps.sh`, reuses the existing two-tier fallback pattern, ships the highest-value slice of the promised tool table.
- **Option 2: Ship `job_status`/`job_cancel` alongside** - rejected: `JobManager` exists but nothing submits jobs; the MCP stdio loop is synchronous, so no tool call is actually cancellable. Exposing status/cancel would advertise capabilities that cannot fire. Deferred until long jobs land.
- **Option 3: Expose `tts_script_generate`, `tts_regenerate`, `transcript_get`, `export_transcript` now** - rejected: they depend on script mode insertion, a persisted-transcript concept, and clip placement that Phase 10's `media_import` hasn't built yet. Deferred without stubs.
- **Option 4: Let the MCP crate call `alp-voice` directly** - rejected: breaks the documented dependency direction the gate enforces.

## Decision

1. **`alp-api` gains a voice module** (`crates/alp-api/src/voice.rs`, `impl ApiSession` methods; session delegates and audits like every other tool):
   - `tts_voices()` (Observe): profile list from `ProfileStore` plus daemon liveness/active model. Cloned profiles are listed with `kind: "cloned"` - visibility is not creation.
   - `voice_profile_get(id)` (Observe): full profile shape minus internals (`cached_embedding` is never exposed; consent is reported as a boolean, never raw).
   - `tts_generate(text, voice_id, speed)` (Edit): validates the voice id against the store, synthesizes daemon-first (`InferClient`, engine name `daemon:<active_model>` or `daemon:builtin-acoustic`), falls back in-process (`BuiltinAcousticBackend` at the project sample rate, engine name `builtin-acoustic`), and writes a 16-bit WAV into the project `media/` directory through `WorkspaceSandbox::validate_path`. No clip is inserted (`media_import` does not exist yet). Speed is clamped to `[0.25, 4.0]`.
   - `voice_design(name, prompt, tags)` (Edit): `design_voice` then `save_profile`. The tool can only ever produce a Designed profile - there is no code path that assigns `VoiceKind::Cloned` or a consent record.
   - `transcribe(path, format?, out?)` (Observe): sandboxed input decode, daemon-first transcription (engine named `daemon:<active_stt_model>`, or `daemon:builtin` when the daemon runs its segmenter, or `builtin` in-process with the placeholder-words admission), returns segments + plain text; when `out` is given, writes `text`/`srt`/`vtt` through the sandbox and reports the path.
   - The voice store opens lazily per call from a session voice-store dir that defaults to the XDG store and can be overridden for tests and embedding hosts (`with_voice_store_dir`); no store writes happen unless a store-touching tool runs.
2. **`alp-mcp` arms**: five tools advertised with JSON Schema in the existing hand-maintained list (`tts_voices`, `voice_profile_get`, `transcribe` at Observe; `tts_generate`, `voice_design` in the `>= Edit` block) and dispatched in the one call match. Engine/failure reporting uses the established `isError` result shape, not protocol errors, and unknown voice ids fail with a plain-language cause.
3. **Resources gap fix**: `alp://project/active` serves the project summary JSON; `alp://voices` (promised by the build plan's resource list) serves the same voice summary as `tts_voices`. Other advertised-but-unread URIs are a known limit, not silently widened.
4. **Honest engine mapping for synthesis**: the built-in acoustic engine maps any unknown voice id to a generic timbre rather than failing, so a store hit is the source of truth for "this voice exists"; engine naming in results is what tells the caller whose voice actually rendered. Designed profiles synthesize today with the generic timbre - documented, not hidden.
5. **No new dependencies**; license set unchanged (`ort` 2.0.0-rc.15 CPU-only remains the only external engine).

Deferred explicitly: `tts_script_generate`, `tts_regenerate`, `transcript_get`, `export_transcript`, `media_import`-backed clip insertion, and `job_status`/`job_cancel` (rationale in Option 2/3).

## Consequences

**Positive**

- The Phase 10 voice row of the tool table is real: list/get/generate/design/transcribe all work headless, daemon-capable, and builtin-fallback-honest.
- The resources surface stops lying: both active-project and voice state are readable.
- The security invariant gains tests (designed-only lifecycle, no cloned creation, no consent synthesis) instead of resting on convention.

**Negative / follow-ups**

- Designed profiles synthesize with a generic timbre until a backend consumes `VoiceProfile.default_params`; results carry the generic-engine naming but the voice metadata is still worth having now.
- `tts_generate` files land in `media/` but are not registered as project sources (no `media_import` yet); wiring that up is the next slice.
- The tool list and call match remain hand-synced; the tolerable cost of the hand-rolled registry stays until the tool count forces a table.

## References

- `crates/alp-api/src/voice.rs` - voice tool methods (this slice)
- `crates/alp-mcp/src/server.rs` - tool arms, schemas, resources
- `ALP_BUILD_PLAN.md` Section 10.3 - promised tool/resource set
- `docs/mcp/SECURITY.md` - voice-cloning invariant
- ADR 0025 - daemon-first engine pattern the tools reuse
