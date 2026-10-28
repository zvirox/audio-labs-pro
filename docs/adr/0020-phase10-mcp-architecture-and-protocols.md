# 0020 — MCP Architecture, Protocol Layer, and Automation Service Design

- Status: accepted
- Date: 2026-10-27
- Phase: 10 (MCP: AI-driven editing, creation, and export)

## Context

Phase 10 enables AI clients, Claude Desktop, and autonomous coding assistants to inspect, edit, mix, clean up, transcribe, and export audio in Audio Labs Pro via the Model Context Protocol (MCP). Decisions D9, D10, and D11 define the SDK strategy, live transport architecture, and permission defaults. The design must adhere to strict security invariants: human-only consent gates cannot be bypassed, operations must be workspace-sandboxed, every edit must be undoable, and sessions must support instantaneous complete rollback ("Undo all AI changes").

## Decision

### 1. Decision D9: MCP SDK and Protocol Implementation
- Implemented a clean, zero-bloat, typed hand-rolled MCP layer in `alp-mcp` on top of `serde_json` and standard I/O (stdio).
- Conforms to Model Context Protocol specification revision `2026-07-28`:
  - `initialize`: capability handshake.
  - `server/discover`: discovery protocol.
  - `tools/list` & `tools/call`: deterministic tool catalog with strict JSON Schema 2020-12 input definitions and structured content responses.
  - `resources/list` & `resources/read`: `alp://` URIs for active project state, effects catalog, and AI agent guidelines.
  - `prompts/list` & `prompts/get`: standard workflow recipes (`clean_up_voice_recording`, `produce_podcast_episode`).

### 2. Decision D10: Transport Architecture
- **Headless Mode:** Standard Input/Output (`stdio`) streaming line-delimited JSON-RPC 2.0 messages. This integrates natively with Claude Desktop, Cursor, and IDE extensions without binding network ports.
- **Attach Mode:** Dedicated Unix domain socket in `$XDG_RUNTIME_DIR/alp/control.sock` mode 0600 with peer UID checking.

### 3. Decision D11: Permission Tiers and Approval Modes
- Defined hierarchical permission tiers in `alp-api::types::PermissionTier`:
  1. `Observe`: read-only project info, track listing, and loudness/quality analysis.
  2. `Edit`: non-destructive edits, track manipulation, clip splitting, ripple deletes, and checkpoint management.
  3. `Export`: master audio rendering and file writes to the exports folder.
- Enforced session checkpointing: an initial checkpoint is automatically captured upon connection, enabling immediate full rollback.
- Hardcoded safety invariants: voice cloning consent gates and external arbitrary shell execution remain strictly prohibited and unreachable through MCP.

### 4. Layering and Service Isolation
- Separated protocol handling (`alp-mcp`) from automation service logic (`alp-api`).
- `alp-mcp` depends strictly on `alp-api` and `alp-core`, preserving clean architectural layering.

## Consequences

Audio Labs Pro gains native AI co-producer capabilities with complete undoability, audit trail logging, and robust security sandboxing.
