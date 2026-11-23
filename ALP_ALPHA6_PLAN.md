# Audio Labs Pro — Alpha 6 Implementation Plan (REAVSA-driven)

> **Scope note:** `REAVSA.md` (in the repo root) is the ground-truth analysis of the two
> installed targets: **target-A** = the installed voice-cloning app, **target-B** = the
> installed system audio-editing app. "ALP" = Audio Labs Pro at the current public release
> `v0.0.1-alpha.5` (tag preserved; alpha.6 will be tagged when Phase 1 lands).
>
> **Branch:** `alpha6` (from `main` @ `b73b297`). Alpha.6 code changes are not pushed to any
> public release branch: `v0.0.1-alpha.5` stays the current public baseline. Tag `v0.0.1-alpha.6`
> is created (locally + pushed by a release maintainer) once Phase 1 passes its verification gates.

---

## 1. Goal

Close the ALP-side gaps that target-A and target-B expose, starting with the highest-value ones
(voice-profile management + batch + audiobook parity), then the target-B editor-parity items,
then MCP infra. Every item below is **implementable in ALP without cloning-model changes** and
every item keeps the human-only invariants of alpha.5 intact (cloned-profile creation/import
stays unreachable via MCP; consent gates stay human-only).

## 2. Phase 1 — voice-platform parity (alpha.6, NOW)

### 2.1 MCP tools (58 → 63)

| Tool | Target-A parity | ALP surface | Tier |
|---|---|---|---|
| `voice_profile_update` | `PUT /profiles/{id}` (`voices update`) | `alp-api::voice::voice_profile_update` | Edit |
| `voice_profile_lock` | `POST /profiles/{id}/lock` (`voices lock`) | `alp-api::voice::voice_profile_lock` | Edit |
| `voice_profile_unlock` | `POST /profiles/{id}/unlock` (`voices unlock`) | `alp-api::voice::voice_profile_unlock` | Edit |
| `tts_batch` | `POST /batch/enqueue` (batch text lines) | `alp-api::tts_batch` (new: `alp-api::voice::tts_batch` + `BatchTtsReport`) | Edit |
| `audiobook_encode` | `POST /audiobook/synthesize` (+ render; `audiobook render`) | `alp-api::audiobook::audiobook_encode` (new module) | Export |

### 2.2 ALP-side implementation map

- `crates/alp-voice/src/profile.rs`: add `locked: bool` to `VoiceProfile` (with
  `#[serde(default)]` for stored-profile compatibility) + set `false` in the constructors
  (`VoiceProfile::builtin`, the designed/clone constructors, and any other literal sites).
- `crates/alp-api/src/voice.rs`:
  - `VoiceProfileUpdate { name, tags, default_params }` (all optional; empty = no change).
  - `ApiSession::voice_profile_update(&mut self, voice_id, update)` — loads via
    `ProfileStore::get_profile`, applies the non-empty fields, `save_profile`, returns
    `VoiceProfileDetail`. Profile kind/backend/consent are untouched (update is not a path to
    create/clone).
  - `ApiSession::voice_profile_lock` / `voice_profile_unlock` — flip `locked`; lock is a
    protective operation: the MCP agent can lock/unlock (non-destructive), the *human*
    GUI enforces protection semantics as before (a locked profile stays visible and immutable
    for agent mutation; the alpha.5 GUI-side guarantee "human-only" is preserved by not
    adding any agent path that removes consent records or flips `kind`).
  - `BatchTtsRequest { voice_id, text, speed? }`, `BatchTtsReport { results: Vec<TtsGenerationReport> }`,
    `ApiSession::tts_batch(&[BatchTtsRequest])` — the **first job submitter**: one `JobManager`
    job per request (`jobs.submit`), executed sequentially via `tts_generate_inner`, with
    `jobs.is_cancelled` checks between requests and `jobs.complete`/`jobs.fail` per job. On
    error the call returns `Err` naming the request; earlier jobs stay queryable via
    `job_status`.
  - `pub(crate) fn tts_generate_inner` visibility (used by `audiobook_encode`).
- `crates/alp-api/src/audiobook.rs` (new module):
  - `AudiobookSegment { text, voice_id, speed?, gap_secs? }`,
    `AudiobookScript { segments }` (serde; same JSON contract as `tts_script_generate`),
  - `AudiobookEncodeReport { track_id, master_path, segment_count, total_duration_secs, job_id }`,
  - `ApiSession::audiobook_encode(script_json, output_path?)` — parses the script (same
    contract), validates every segment (non-empty text, known voice), submits one job,
    `add_track("Audiobook")`, and per segment: `tts_generate_inner` → `media_import` →
    `clip_add` at the cursor with the per-segment gap; `update_progress`; then
    `export_audio(ExportPreset::Audiobook, master_path)` (default master path under
    `paths.renders_dir()`; `output_path` honored inside the sandbox). Job cancelled between
    segments breaks the loop; partial timeline stays (documented).
- `crates/alp-api/src/lib.rs`: `pub mod audiobook;` + re-exports
  (`AudiobookEncodeReport`, `BatchTtsReport`, `BatchTtsRequest`, `VoiceProfileUpdate`).
- `crates/alp-mcp/src/tool_defs.rs`: the 5 `McpTool` entries (schemas incl. `expected_revision`
  decoration for free via `decorate_schema`; `PAGED_TOOLS` unchanged).
- `crates/alp-mcp/src/dispatch.rs`: 5 dispatch arms (mirroring the alpha.5 style: argument
  parsing → session call → success/failure summary + structured payload).
- `crates/alp-mcp/src/main.rs`: update `assert_eq!(tools.len(), 58)` → `63`; add annotations
  spot-checks for the new tools.
- Docs (same-commit rule): `docs/modules/alp-api/audiobook.md` (new; required sections per
  `scripts/check-docs.sh`), `docs/modules/alp-api/voice.md` (+ sections),
  `docs/mcp/TOOLS.md` (count + new entries), `ALP_STATUS_REPORT.md` (alpha.6 section).

### 2.3 Verification gates

- `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` (workspace) — green.
- New tests:
  - `voice.rs`: `voice_profile_update_mutates_the_stored_profile`,
    `voice_profile_lock_unlock_round_trip`, `tts_batch_runs_each_request_as_a_job`
    (asserts job ids, engine names, WAVs written, `job_status` Completed),
    `tts_batch_rejects_an_empty_request_list`.
  - `audiobook.rs`: `audiobook_encode_places_clips_and_exports_master` (script of 2 segments
    → track "Audiobook" with clips, master file exists, `total_duration_secs` = sum of
    segment durations + gaps), `audiobook_encode_rejects_an_unknown_voice`,
    `audiobook_encode_rejects_an_empty_script`.
- `scripts/check-file-length.sh`, `check-deps.sh`, `check-docs.sh`, `check-ui.sh` green.
- Manual smoke: `alp-mcp` stdio — list tools (63), call `voice_profile_lock` on a builtin
  voice, call `tts_batch` with 2 requests, call `audiobook_encode` with a 2-segment script.
- `scripts/phase10-demo.sh`-style demo: add `scripts/alpha6-phase1-demo.sh` (mirrors
  phase-demo.sh conventions) + a one-line note in `ALP_BUILD_PLAN.md`.

## 3. Phase 2 — target-B editor parity (alpha.6+ or alpha.7)

MCP + `alp-dsp`/`alp-core`/`alp-mcp` work:

1. **Spectral editing** (target-B: Spectral Edit Multi Tool, Spectral Edit Parametric EQ,
   Spectral Edit Shelves, Spectral Delete + vendor spectral-edit scripts):
   - `alp-core`: a spectral-edit clip/segment model + a time→frequency mask representation.
   - `alp-dsp`: spectral-mask application (attenuate/delete frequency bands over a region).
   - MCP tools: `spectral_edit` (region + bands, delete/attenuate), `spectral_select`? — start
     with `spectral_edit` only, add selection later if needed.
2. **Time-stretch / pitch** (target-B: Paulstretch, Sliding Stretch, Change Speed and Pitch):
   - License review first: Signalsmith Stretch (MIT) vs Rubber Band GPL (ADR/TODO entry).
   - Phase-vocoder time stretch using existing `alp-dsp` primitives (or new `alp-dsp::time_stretch`).
   - MCP: `time_stretch`, `pitch_shift` on clip regions.
3. **Delay / reverb / echo + other B-side effects** (target-B §3.2.3):
   `echo`, `reverb`, `delay` (target-A: no echo/reverb/delay in ALP α5 — gap);
   `repeat`, `reverse`, `invert`, `vocal_reduction_isolation`; legacy compressor/limiter.
4. **Macros** (target-B §3.2.5): `macro_save`, `macro_apply` (durable named command sequences;
   reuse `edit_batch` + undo groups).
5. **Glossary** (target-A: `/glossary/*`): `glossary_list`, `glossary_add`, `glossary_update`,
   `glossary_delete` (ALP: `alp-voice::lexicon` covers pronunciation rules; glossary is the
   project-scoped term store).
6. **Marketplace / gallery MCP parity** (target-A): `marketplace_publish`,
   `marketplace_import` (ALP GUI covers these; agent surface missing).
7. **Project bundles** (target-A): `project_bundle_export`, `project_bundle_import`.
8. **Watermark** (target-A): `watermark_detect`, `watermark_status`, `watermark_settings`
   (reads embedded metadata; ALP already stamps `synthetic_voice` metadata on exports — reuse).
9. **Targets-B gaps, misc**: Opus/AAC/MP3 export (license-deferred — non-goal for alpha.6),
   minimap, more formats/chains.

## 4. Phase 3 — MCP infra (alpha.7)

- `docs/mcp/EVAL.md` + `scripts/mcp_eval.py`/`mcp-eval.sh` harness (scripted-agent replay) —
  build-plan §10.8.
- `alp-api/src/limits.rs`: expose caps/timeouts as config; per-tier rate limits documented.
- Authenticated remote MCP: Host/Origin validation (ADR 0020 follow-up).
- Multi-client MCP sessions; Review-mode accept/reject flow.
- More workflow recipes in `docs/mcp/AGENT_GUIDE.md` (audiobook_encode, tts_batch recipes).

## 5. Non-goals (kept from alpha.5; documented in REAVSA)

- Clone creation/import via MCP — human-only consent invariant stays.
- Video dubbing — ALP has no video-track substrate (REAVSA §4 note).
- Cloud accounts/sharing, MIDI, MP3/Opus/AAC export.

## 6. Release bookkeeping

- `v0.0.1-alpha.6`: tag created after Phase 1 passes gates; current public release stays
  `v0.0.1-alpha.5`.
- `ALP_STATUS_REPORT.md`: add an "Alpha 6 (current branch)" section listing implemented tools
  + verification + remaining Phase 2/3 items.
- `CHANGELOG.md`: add an `Unreleased (alpha.6)` section (see CHANGELOG.md conventions).
- `THIRD_PARTY.md`: unchanged (no new crates/models).
