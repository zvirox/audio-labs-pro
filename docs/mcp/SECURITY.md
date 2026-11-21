# Audio Labs Pro — MCP Threat Model & Security Architecture

Security architecture governing AI model and automation interactions.

---

## 1. Security Invariants

1. **Human-Only Consent Gate:** The MCP API exposes no voice-cloning tools — cloned profiles cannot be created or imported through it at all. The `voice_design` tool creates `Designed` profiles only (no reference audio, no consent record, no `Cloned` kind), and `tts_voices`/`voice_profile_get` may list and read cloned profiles but never touch their creation. The underlying library enforces structural consent checks (presence, well-formed fields, kind/consent consistency) on every save and import, but does not yet cryptographically verify a record's authenticity.
2. **Workspace Sandboxing:** All file reads, imports, and exports are restricted to canonicalized workspace directories (`WorkspaceSandbox`). Path traversal attempts (`..` or symlink escapes) trigger immediate `SandboxEscape` errors.
3. **No Arbitrary Command Execution:** `alp-mcp` exposes only domain-specific audio editing commands. It contains no shell execution, generic file writers, or network download tools.
4. **Append-Only Audit Log:** All tool calls are recorded in `$WORKSPACE/audit.jsonl` with client ID, tool name, a SHA-256 digest of the staged call arguments (via `stage_arguments`), and before/after project revisions.
5. **Session Rollback:** The initial project state is snapshotted upon client connection, allowing instantaneous complete rollback via `rollback_session`.

## 2. Live GUI Attach Boundary

The native control host is owner-only, not a remote service. Its parent
directory is `0700`, its Unix socket is `0600`, and `SO_PEERCRED` must report the
UI process's effective UID before authentication is parsed. A random token is
generated for every enabled host and compared without an early-exit comparison;
tokens are held only in memory, omitted from the activity log and workspace,
and revoked on Stop AI. Preventing another UID is a kernel boundary. Preventing
an unapproved process under the same UID additionally depends on keeping this
token secret.

`client_id` is a sanitized, self-declared label used for operator approval—not
an operating-system identity. “Always allow” persists only that label; every
new host still requires its fresh token. The first new label requires an
explicit GUI decision. Requests are bounded to 2 MiB and arrive on worker
threads. A full attached MCP call executes against a fresh, revision-tagged
snapshot of the live project; UI code merges only a successful result whose
base revision still matches the one `EditSession`. Permission tiers are copied
from the GUI, not trusted from `--tier` or socket input. Changing the tier
rebuilds attached workers and rejects their earlier completions.

The attached proxy has no project-initialization mode. Its worker-side sandbox
is rebuilt from the live project's own root on every call, preserving path and
symlink checks for imports, renders and exports. Voice cloning/profile creation,
model installation, application settings, generic file access and shell
execution remain outside the MCP catalog. Existing voice-generation callers
still enforce consent and synthetic-speech metadata. `rollback_session` is
refused in attached mode to protect operator work made after attachment.

Change subscriptions are explicit and carry no token in their events. Operator
and attached changes are published only after the live commit and view-model
synchronization. Socket I/O remains off the UI thread, and stale or revoked
worker results cannot mutate the live project.
