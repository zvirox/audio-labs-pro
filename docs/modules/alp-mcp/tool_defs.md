# `alp-mcp` / `tool_defs`

## Purpose

Defines tool manifests, descriptions, and JSON Schema definitions for all 58
tools exposed by the Model Context Protocol server, plus the single behavioural
metadata table (`title` + annotations) and the cross-cutting schema arguments
(`limit`/`cursor` on list tools, `expected_revision` on writes).

## Public API

- `tools_for_tier(tier: PermissionTier) -> Vec<McpTool>` — the tier-filtered
  tool list, with every input schema decorated.
- `metadata_for(name: &str) -> ToolMetadata` — `title` plus
  `ToolAnnotations` (`readOnlyHint`, `destructiveHint`, `idempotentHint`,
  `openWorldHint`) for one tool. Unknown names fall back to the conservative
  answer (not read-only, destructive, not idempotent, open world) so a client
  never assumes an unknown call is safe to retry.
- `PAGED_TOOLS` — the five list tools that accept `limit`/`cursor`.

## Data flow

1. `tools_for_tier` collects Observe-tier tools, then appends Edit-tier
   (mutations, plugins, speech generation, voice design) and Export-tier
   (delivery exports) tools.
2. Before returning, every schema runs through `decorate_schema`, which adds
   `limit`/`cursor` to `PAGED_TOOLS` and `expected_revision` to every tool
   whose annotations say it mutates state. Doing it in one pass keeps 58
   hand-written schemas from restating the same three properties, and makes
   the contract impossible to forget on a new tool.
3. `McpTool`'s `Serialize` impl looks up `metadata_for` so the `title` and
   `annotations` fields on the wire always come from the table.

## Allowed dependencies

- `serde_json::{Value, json}`
- `alp_api::{PermissionTier, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE}`
- `crate::protocol::{McpTool, ToolAnnotations, ToolMetadata}`

## Threading / real-time notes

Pure functions over static definitions; no locks or I/O.

## Error handling

Deterministic static definitions; does not fail. `metadata_for` never panics on
an unknown name — it degrades to the conservative annotations.

## Testing

- `every_advertised_tool_declares_a_title_and_annotations` — all 58 tools carry
  a real title and all four boolean annotations, and spot-checks that the
  hints match reality (`project_info` read-only, `clip_remove` destructive,
  `clip_add` not, `plugin_list` open-world).
- `reads_ignore_expected_revision_and_write_tools_advertise_it` — write tools
  advertise the guard in their schema, reads do not, and all five list tools
  advertise `limit`/`cursor`.
- Tier visibility tests in `crates/alp-mcp/src/main.rs`.

## Known limits

- Schema definitions are static; there is no `outputSchema` yet.
- Annotation hints describe the tool's contract, not a guarantee about any one
  invocation (e.g. `export_audio` is marked destructive because it can
  overwrite a file).
