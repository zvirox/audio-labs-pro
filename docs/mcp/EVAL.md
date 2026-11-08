# Audio Labs Pro — Model Context Protocol (MCP) Evaluation Suite & Benchmark

This document defines the evaluation framework, benchmark suites, and automated verification harness for the Audio Labs Pro Model Context Protocol (`alp-mcp`) implementation.

---

## 1. Overview & Evaluation Objectives

Audio Labs Pro integrates with autonomous AI agents, coding assistants (Claude Desktop, Cursor, Antigravity), and headless pipelines through the Model Context Protocol (revision `2026-07-28`).

Because an AI co-producer can perform non-destructive multi-track edits, render audio masters, and synthesize neural speech, the evaluation suite enforces strict guarantees:

1. **Protocol Compliance:** Exact adherence to JSON-RPC 2.0 streaming over standard input/output (`stdio`) and Unix domain sockets.
2. **Schema & Argument Determinism:** Every tool defines strict JSON Schema 2020-12 input definitions; invalid parameters are rejected with structured error objects before touching the audio engine.
3. **Permission Tier Integrity:** Tools are gated behind hierarchical tiers (`Observe` < `Edit` < `Export`). Sessions bound to lower tiers are prevented from executing higher-tier actions.
4. **Sandboxing & Jail Containment:** File operations must stay within the caller's authorized workspace directory; path traversal (`..`), absolute unauthorized paths, and symlink escapes are strictly blocked.
5. **Ethics & Consent Invariants:** In accordance with [`docs/ETHICS.md`](../ETHICS.md), automated agents are barred from creating or importing cloned voice profiles. Voice design is limited strictly to synthetic profiles (`VoiceKind::Designed`).
6. **State Preservation & Instant Rollback:** Every modification is undoable, and `rollback_session` guarantees instantaneous reversion to the initial checkpoint state captured at session start.
7. **Comprehensive Audit Trail:** All tool calls, parameter digests, revisions, and execution outcomes are recorded to `$WORKSPACE/audit.jsonl`.

---

## 2. Evaluation Axes (The 8 Benchmark Suites)

The evaluation harness (`scripts/mcp-eval.sh` / `scripts/mcp_eval.py`) executes 8 distinct benchmark suites:

```
┌────────────────────────────────────────────────────────────────────────┐
│                   Audio Labs Pro MCP Evaluation Suite                  │
├──────────────────────────┬─────────────────────────────────────────────┤
│ Suite 1: Handshake       │ initialize, server/discover, capabilities   │
│ Suite 2: Resources       │ alp:// URIs (project, effects, agent guide) │
│ Suite 3: Prompts         │ Multi-step workflow recipe templates        │
│ Suite 4: Tool Catalog    │ Schema validity and tool discovery          │
│ Suite 5: Permissions     │ Tier gating (Observe vs Edit vs Export)     │
│ Suite 6: Sandboxing      │ Path traversal rejection & jail security    │
│ Suite 7: Ethics Gate     │ Human-only consent invariant protection     │
│ Suite 8: Workflow & Undo │ Full timeline production, export & rollback │
└──────────────────────────┴─────────────────────────────────────────────┘
```

### Suite 1: Protocol Handshake & Discovery
- **Methods:** `initialize`, `server/discover`
- **Verification:**
  - `initialize` returns `serverInfo.name == "audio-labs-pro"`, protocol version, and declared capabilities (`tools`, `resources`, `prompts`).
  - `server/discover` reports protocol revision `2026-07-28` and endpoint metadata.
  - JSON-RPC framing complies with single-line delimited messages over stdio.

### Suite 2: Resource Resolution (`alp://`)
- **Methods:** `resources/list`, `resources/read`
- **Verification:**
  - `resources/list` advertises active URIs:
    - `alp://project/active` (live timeline state, tracks, duration, revision)
    - `alp://effects/catalog` (supported DSP effect algorithms and parameters)
    - `alp://docs/agent-guide` (agent operational rules)
  - `resources/read` successfully returns text or JSON content for each valid URI.
  - Requesting an invalid URI returns a structured JSON-RPC error.

### Suite 3: Prompt Templates & Recipes
- **Methods:** `prompts/list`, `prompts/get`
- **Verification:**
  - `prompts/list` advertises multi-step production templates (`clean_up_voice_recording`, `produce_podcast_episode`).
  - `prompts/get` accepts valid prompt names and returns pre-structured instructional messages ready for LLM consumption.

### Suite 4: Tool Catalog & Schema Conformance
- **Methods:** `tools/list`
- **Verification:**
  - Returns all registered tools appropriate for the active tier.
  - Every tool provides: `name`, `description`, and `input_schema` matching JSON Schema specifications.
  - Every input property defines expected data types, constraints, and descriptions.

### Suite 5: Hierarchical Permission Tier Enforcement
- **Scenarios:**
  - **`Observe` Tier Session:**
    - Authorized: `project_info`, `track_list`, `analyze_loudness`, `media_list`, `tts_voices`.
    - Prohibited: calling `track_add` or `project_save` returns an error response (`isError: true` or error code).
    - Prohibited: calling `export_audio` returns an error response.
  - **`Edit` Tier Session:**
    - Authorized: all `Observe` tools plus timeline editing (`track_add`, `clip_split`, `range_delete`, `tts_generate`, `media_import`, `clip_add`).
    - Prohibited: calling `export_audio` is refused.
  - **`Export` Tier Session:**
    - Authorized: full access including `export_audio`.

### Suite 6: Security Sandboxing & Directory Jail
- **Scenarios:**
  - Attempting to pass `path: "../../../etc/passwd"` to `media_import` or `transcribe` must fail with an unauthorized traversal error.
  - Attempting to pass an absolute path outside the workspace (e.g. `/tmp/outside.wav`) must fail validation.
  - Destination export paths outside the workspace sandbox are refused.
  - Failed security attempts must be logged in `audit.jsonl` with an error flag.

### Suite 7: Ethics Safety Gate & Voice Cloning Protection
- **Scenarios:**
  - Assert that `tools/list` contains **no** cloning creation or reference import tools.
  - Calling `voice_design` produces exclusively `VoiceKind::Designed` profiles.
  - Calling `voice_profile_get` on a cloned profile reports `consent_recorded: true|false` as a boolean without exposing raw embeddings.

### Suite 8: Full Agentic Production Workflow & Session Rollback
- **Multi-Step Production Execution:**
  1. `project_info` -> verify clean initial revision.
  2. `track_add` -> create "Narration" track.
  3. `track_add` -> create "Ambience" track.
  4. `tts_generate` -> synthesize test dialogue with synthetic watermark header.
  5. `media_import` -> import synthesized WAV into project `media/`.
  6. `clip_add` -> place imported audio onto "Narration" track.
  7. `analyze_loudness` -> verify LUFS loudness and peak metrics.
  8. `checkpoint_create` -> snapshot interim project state.
  9. `export_audio` -> render master file with delivery preset (`podcast`).
- **Rollback Fidelity:**
  10. Invoke `rollback_session`.
  11. `project_info` -> verify project has reverted to 0 tracks and initial revision.
- **Audit Log Verification:**
  - Confirm `$WORKSPACE/audit.jsonl` exists and records all operations with timestamps and argument digests.

---

## 3. Benchmark Scoring & Passing Criteria

| Evaluation Suite | Weight | Minimum Threshold | Target |
|---|---|---|---|
| Suite 1: Handshake & Discovery | 10% | 100% | 100% pass |
| Suite 2: Resources (`alp://`) | 10% | 100% | 100% pass |
| Suite 3: Prompts & Recipes | 10% | 100% | 100% pass |
| Suite 4: Tool Catalog & Schemas | 10% | 100% | 100% pass |
| Suite 5: Permission Tier Gating | 15% | 100% | 100% pass |
| Suite 6: Sandbox Jail Escape Checks | 15% | 100% | 100% pass (0 leaks) |
| Suite 7: Ethics Safety Invariants | 10% | 100% | 100% pass (0 bypasses) |
| Suite 8: Workflow & Rollback | 20% | 100% | 100% pass |

**Overall Requirement:** 100% pass across all 8 suites. Any security sandbox leak, unauthorized permission bypass, or failed session rollback constitutes an immediate test failure.

---

## 4. Running the Automated Evaluation Harness

To run the complete automated evaluation suite:

```bash
bash scripts/mcp-eval.sh
```

The script builds `alp-mcp`, provisions an isolated sandbox, runs `scripts/mcp_eval.py`, validates the JSONL audit trail, and prints the benchmark summary report.
