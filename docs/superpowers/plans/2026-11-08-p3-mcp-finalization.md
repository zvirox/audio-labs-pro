# P3 (Phase 10 MCP Finalization) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finalize Phase 10 Model Context Protocol (MCP) deliverables by authoring `docs/mcp/EVAL.md` and implementing an automated, end-to-end scripted evaluation harness (`scripts/mcp-eval.sh` / `scripts/mcp_eval.py`) validating protocol conformance, tool schemas, permission tier gating, sandboxing boundaries, and rollback fidelity.

**Architecture:** Create an automated evaluation suite that executes `alp-mcp` over stdio JSON-RPC 2.0 streaming, systematically testing 8 evaluation axes: protocol handshake, resource inspection, prompt templates, permission tier boundaries (`Observe` vs `Edit` vs `Export`), sandbox jail escape prevention, voice cloning ethics invariants, multi-turn workflow execution (`tts_generate` -> `media_import` -> `clip_add` -> `analyze_loudness` -> `export_audio`), and session rollback. Document the evaluation methodology and scoring criteria in `docs/mcp/EVAL.md`.

**Tech Stack:** Python 3, Bash, Rust `alp-mcp`, JSON-RPC 2.0, MCP specification revision `2026-07-28`.

**Spec:** `docs/adr/0020-phase10-mcp-architecture-and-protocols.md`, `docs/mcp/TOOLS.md`, `docs/mcp/SECURITY.md`, `docs/mcp/AGENT_GUIDE.md`.

## Global Constraints

- **Safe Rust:** `unsafe_code = "deny"` across the workspace.
- **Protocol Spec:** Conforms strictly to MCP revision `2026-07-28`.
- **Zero Dependencies:** Evaluation harness uses Python 3 standard library (`subprocess`, `json`, `sys`, `pathlib`, `tempfile`) without requiring external pip packages.
- **Architecture Guards:** All 4 guard scripts (`check-file-length.sh`, `check-docs.sh`, `check-deps.sh`, `check-ui.sh`) must pass cleanly.

## Review Focus

1. **Permission Tier Enforcement:** Verify that invoking an `Edit` tool under `--tier observe` or an `Export` tool under `--tier edit` cleanly returns an MCP error response (`isError: true` or JSON-RPC error).
2. **Sandbox Traversal Security:** Verify that path arguments attempting directory traversal (`../` or external absolute paths) fail safely and are logged in the audit trail.
3. **Ethics Safety Gate:** Verify that no voice cloning creation/import tools are exposed through `tools/list` and that human consent records remain unbypassable.
4. **Session Rollback Fidelity:** Verify that `rollback_session` cleanly undoes all edits made during the session, restoring the timeline to the initial checkpoint state.
5. **Deterministic Audit Log:** Verify that `audit.jsonl` logs every invocation with caller client ID, arguments digest, and revision.

---

### Task 1: Author `docs/mcp/EVAL.md`

**Files:**
- Create: `docs/mcp/EVAL.md`
- Modify: `docs/README.md` (add EVAL.md to documentation index)

**Interfaces:**
- Produces: Complete MCP evaluation specification and benchmark guide covering the 8 evaluation axes, scoring rubrics, and automated test instructions.

- [ ] **Step 1.1: Draft `docs/mcp/EVAL.md`**
  Cover:
  - Evaluation Objectives and Scope (Protocol revision `2026-07-28`, AI co-producer reliability).
  - 8 Comprehensive Evaluation Axes:
    1. Protocol Handshake & Capability Discovery (`initialize`, `server/discover`).
    2. Tool Catalog & Input Schema Conformance (JSON Schema 2020-12 validation).
    3. Resource URI Resolution (`alp://project/active`, `alp://effects/catalog`, `alp://docs/agent-guide`).
    4. Prompt Template Interpolation (`clean_up_voice_recording`, `produce_podcast_episode`).
    5. Hierarchical Permission Tiers (`Observe` < `Edit` < `Export`).
    6. Directory Sandboxing & Jail Escape Rejection.
    7. Ethics Safety Gate (Strict prohibition on automated voice clone creation).
    8. Multi-Turn Timeline Assembly, Export, and Session Rollback.
  - Scoring Methodology, Failure Thresholds, and Benchmark Report Format.
  - Instructions for executing the automated evaluation harness.

- [ ] **Step 1.2: Update `docs/README.md`**
  Add `[mcp/EVAL.md](mcp/EVAL.md)` to the documentation index table.

---

### Task 2: Implement Automated Scripted Evaluation Harness

**Files:**
- Create: `scripts/mcp_eval.py`
- Create: `scripts/mcp-eval.sh`

**Interfaces:**
- Produces: Executable harness running all 8 evaluation suites against `alp-mcp`, emitting structured test summaries and exiting with code 0 on complete pass.

- [ ] **Step 2.1: Implement `scripts/mcp_eval.py`**
  - Connects to `alp-mcp` via `subprocess.Popen` over stdio with line-delimited JSON-RPC 2.0.
  - Implements helper `rpc(method, params, req_id)`.
  - Executes Suite 1: Protocol Handshake (`initialize`, `server/discover`).
  - Executes Suite 2: Resources & Prompts (`resources/list`, `resources/read`, `prompts/list`, `prompts/get`).
  - Executes Suite 3: Tool Catalog & Schema Conformance (`tools/list`).
  - Executes Suite 4: Permission Tier Enforcement (Verify `Observe` session rejects `track_add` and `export_audio`).
  - Executes Suite 5: Security Sandboxing & Path Traversal (Verify rejection of `../escape.wav` and `/etc/shadow`).
  - Executes Suite 6: Voice Cloning Ethics Gate (Verify absence of cloning tools; verify `voice_design` yields `VoiceKind::Designed`).
  - Executes Suite 7: Full Agentic Production Workflow (`project_info` -> `track_add` -> `tts_generate` -> `media_import` -> `clip_add` -> `analyze_loudness` -> `checkpoint_create` -> `export_audio`).
  - Executes Suite 8: Session Rollback & Audit Log Verification (`rollback_session` restores project; verify `audit.jsonl` entries).
  - Prints structured test report and pass/fail summary.

- [ ] **Step 2.2: Implement `scripts/mcp-eval.sh` wrapper**
  - Runs workspace check guards (`check-file-length.sh`, `check-deps.sh`, `check-docs.sh`, `check-ui.sh`).
  - Builds `alp-mcp` binary (`cargo build --bin alp-mcp`).
  - Sets up isolated temporary workspace.
  - Invokes `python3 scripts/mcp_eval.py`.
  - Reports final exit code.

- [ ] **Step 2.3: Execute harness and verify all evaluation suites pass**
  Run: `bash scripts/mcp-eval.sh`
  Expected: All 8 evaluation suites PASS, 0 failures, exit 0.

---

### Task 3: Workspace Verification & Quality Gates

- [ ] **Step 3.1: Run all architecture guards**
  - `bash scripts/check-file-length.sh`
  - `bash scripts/check-docs.sh`
  - `bash scripts/check-deps.sh`
  - `bash scripts/check-ui.sh`
- [ ] **Step 3.2: Run formatting and clippy gates**
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
- [ ] **Step 3.3: Run full workspace test suite**
  - `cargo test --workspace`
