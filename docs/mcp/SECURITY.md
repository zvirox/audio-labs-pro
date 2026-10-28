# Audio Labs Pro — MCP Threat Model & Security Architecture

Security architecture governing AI model and automation interactions.

---

## 1. Security Invariants

1. **Human-Only Consent Gate:** Cloned voice profile generation is strictly prohibited through the MCP API. Any attempt to invoke cloning without interactive human consent is rejected at the library level.
2. **Workspace Sandboxing:** All file reads, imports, and exports are restricted to canonicalized workspace directories (`WorkspaceSandbox`). Path traversal attempts (`..` or symlink escapes) trigger immediate `SandboxEscape` errors.
3. **No Arbitrary Command Execution:** `alp-mcp` exposes only domain-specific audio editing commands. It contains no shell execution, generic file writers, or network download tools.
4. **Append-Only Audit Log:** All tool calls are recorded in `$WORKSPACE/audit.jsonl` with client ID, tool name, argument digest, and before/after project revisions.
5. **Session Rollback:** The initial project state is snapshotted upon client connection, allowing instantaneous complete rollback via `rollback_session`.
