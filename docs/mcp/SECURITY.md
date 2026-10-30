# Audio Labs Pro — MCP Threat Model & Security Architecture

Security architecture governing AI model and automation interactions.

---

## 1. Security Invariants

1. **Human-Only Consent Gate:** The MCP API exposes no voice-cloning tools — cloned profiles cannot be created or imported through it at all. The `voice_design` tool creates `Designed` profiles only (no reference audio, no consent record, no `Cloned` kind), and `tts_voices`/`voice_profile_get` may list and read cloned profiles but never touch their creation. The underlying library enforces structural consent checks (presence, well-formed fields, kind/consent consistency) on every save and import, but does not yet cryptographically verify a record's authenticity.
2. **Workspace Sandboxing:** All file reads, imports, and exports are restricted to canonicalized workspace directories (`WorkspaceSandbox`). Path traversal attempts (`..` or symlink escapes) trigger immediate `SandboxEscape` errors.
3. **No Arbitrary Command Execution:** `alp-mcp` exposes only domain-specific audio editing commands. It contains no shell execution, generic file writers, or network download tools.
4. **Append-Only Audit Log:** All tool calls are recorded in `$WORKSPACE/audit.jsonl` with client ID, tool name, a SHA-256 digest of the staged call arguments (via `stage_arguments`), and before/after project revisions.
5. **Session Rollback:** The initial project state is snapshotted upon client connection, allowing instantaneous complete rollback via `rollback_session`.
