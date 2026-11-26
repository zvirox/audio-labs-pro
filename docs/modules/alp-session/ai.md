# alp-session::ai

Headless AI control host and attached Model Context Protocol (MCP) server for Audio Labs Pro.

## Purpose

`alp-session::ai` coordinates external AI clients and agents connecting over local IPC. It owns:
- The owner-only Unix domain control socket listener at `$XDG_RUNTIME_DIR/alp/control.sock` (mode 0600, peer UID verified).
- Session authentication token generation (`ALP_CONTROL_TOKEN`) and client approval workflows.
- Permission tier enforcement (`Observe`, `Edit`, `Export`).
- Background MCP client worker pools with revision-tagged scratch snapshot isolation.
- Live project commit into `alp_session::EditSession` where multi-step worker operations commit as a single atomic undo entry.
- Real-time change feed broadcasting to subscribed agents and desktop UI notifications.

## Public API

```rust
pub struct AiControlStatus {
    pub enabled: bool,
    pub socket_path: Option<String>,
    pub token: Option<String>,
    pub tier: PermissionTier,
    pub trusted_clients: Vec<String>,
    pub pending_clients: Vec<String>,
    pub commits: u64,
    pub last_commit_ms: Option<u64>,
    pub max_commit_ms: u64,
    pub last_status: String,
}

pub struct AiActivityEntry {
    pub timestamp: String,
    pub client_id: String,
    pub operation: String,
    pub outcome: String,
    pub queue_to_commit_ms: Option<u64>,
    pub committed: bool,
}

pub enum AiEvent {
    RevisionChanged(u64),
    ApprovalRequired { client_id: String },
    Activity(AiActivityEntry),
}

pub struct AiControlManager {
    pub fn new(trusted: BTreeSet<String>) -> Self;
    pub fn is_enabled(&self) -> bool;
    pub fn tier(&self) -> PermissionTier;
    pub fn set_tier(&mut self, tier: PermissionTier);
    pub fn set_transport(&mut self, transport: Arc<Mutex<TransportController>>);
    pub fn start(&mut self, runtime_dir: &Path, tier: PermissionTier) -> Result<AiControlStatus, String>;
    pub fn stop(&mut self, reason: &str);
    pub fn status(&self) -> AiControlStatus;
    pub fn activity_log(&self) -> Vec<AiActivityEntry>;
    pub fn respond_approval(&mut self, client_id: &str, decision: ApprovalDecision, remember: bool) -> bool;
    pub fn tick(&mut self, session: &mut EditSession) -> Vec<AiEvent>;
}
```

## Data flow

1. **Host Initialization:** Operator triggers AI attach. `start()` creates `$XDG_RUNTIME_DIR/alp/control.sock` with permission 0600 and generates an in-memory random session token.
2. **Client Attachment:** External CLI or desktop tool (`alp-mcp --attach`) connects to the socket and sends `ControlRequest::Authenticate`.
3. **Approval:** If client is in `trusted_clients`, connection is approved immediately. Otherwise, `AiEvent::ApprovalRequired` notifies the operator for a Session/Always/Deny decision.
4. **Live Transport Interception:** `transport_*` tools are parsed on the UI tick thread, checked against the permission tier, and applied to the injected `TransportController`; no project snapshot or revision commit is involved.
5. **Scratch Execution:** Other incoming tool calls (`ControlRequest::Mcp`) are dispatched to dedicated worker threads (`alp-attached-mcp-{client_id}`). Each worker executes against an ephemeral `ApiSession` snapshot bound to the project's base revision.
6. **Atomic Commit:** On tool completion, `PreparedTransaction::Edit` is sent back to `AiControlManager`. During `tick()`, if base revision matches `EditSession::revision()`, changes commit atomically as one undo step.
7. **Publication:** Committed changes are published to socket subscribers via `ControlHost::publish` and emitted to the UI via `AiEvent::RevisionChanged`.

## Allowed dependencies

- `alp-api`: Control protocol frames (`ControlHost`, `ControlRequest`, `ControlResponse`, `ProjectChange`), permission tiers, sandbox limits, and API sessions.
- `alp-core`: Core source IDs and model definitions.
- `alp-engine`: Source audio representations.
- `alp-mcp`: Shared tool catalog dispatcher and MCP server protocol.
- `alp-media`: Source decoding and format validation.
- Standard library: Mutex, mpsc channels, process threads, file permissions, and Unix domain sockets.
- Strictly forbidden: `iced`, `tauri`, `alp-ui`, `alp-ui-kit`.

## Threading / real-time notes

- The control socket listener runs on background thread `alp-control-listener`.
- Each connected client runs on background worker thread `alp-control-client` and `alp-attached-mcp-{client_id}`.
- Neither the audio rendering thread nor the Tauri webview thread ever blocks on client I/O.
- `AiControlManager::tick()` runs non-blockingly via `try_recv()` (bounded to at most 64 requests per tick).
- Transport tools are handled inside that same tick through `ai::transport_control`; the transport mutex is held only for one command.
- Audio decoding for attached imports runs on worker threads before committing to the live session.

## Error handling

- **Revision Conflict:** If an operator edits the project while an agent prepares a mutation, the commit fails closed with `revision_conflict`, preserving human work and returning actionable guidance.
- **Permission Denied:** Tool calls exceeding the current permission tier (`Observe`, `Edit`, `Export`) are rejected immediately with descriptive explanations.
- **Payload & Rate Limiting:** Requests exceeding frame size limits or window budgets are clamped or rejected with `-32600` JSON-RPC errors.
- **Rollback Safety:** Attached mode explicitly forbids `rollback_session` to avoid destroying human edits made after attachment; `history_undo` is used instead.

## Testing

- `test_ai_control_observe_tier_blocks_edits`: Verifies that read-only observe tier blocks project mutations.
- `test_ai_control_edit_tier_adds_track_and_is_undoable`: Confirms track addition commits cleanly, increments revision, rejects stale revisions, and undoes in a single step.
- `test_ai_control_manager_socket_lifecycle_and_tick`: Validates socket bind, connection, approval handshake, and project info requests.
- `test_attached_mcp_full_catalog_and_tool_commit`: Verifies end-to-end full MCP `tools/list` query, `tools/call track_add` transaction commit, and `history_undo` rollback.

## Known limits

- Control socket hosting is Linux/Unix specific (`$XDG_RUNTIME_DIR/alp/control.sock`).
- Attached tool calls are currently serialized per client connection to preserve deterministic revision tracking.
- Audio rendering for attached clip synthesis is bounded by system memory and workspace sandbox storage quotas.
