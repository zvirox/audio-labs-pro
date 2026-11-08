# `alp-mcp` / `tool_defs`

## Purpose

Defines tool manifests, descriptions, and JSON Schema definitions for all tools exposed by the Model Context Protocol (MCP) server.

## Public API

- `tools_for_tier(tier: PermissionTier) -> Vec<McpTool>`: Returns the list of tools accessible under the given permission tier.

## Data flow

1. Accepts a `PermissionTier` (Observe, Edit, or Export).
2. Collects standard Observe-tier tools (read-only project, media, loudness, analysis, and discovery tools).
3. Conditionally appends Edit-tier tools (clip/track mutations, plugins, speech generation, voice design).
4. Conditionally appends Export-tier tools (audio delivery exports).
5. Returns a unified vector of `McpTool` descriptors.

## Allowed dependencies

- `serde_json::json`
- `alp_api::PermissionTier`
- `crate::protocol::McpTool`

## Threading / real-time notes

Pure schema generator function executing on caller thread; no locks or I/O involved.

## Error handling

Deterministic static definitions; does not fail.

## Testing

Verified by tool discovery and tier-filtering unit tests in `crates/alp-mcp/src/main.rs`.

## Known limits

- Schema definitions are static compile-time constants.
