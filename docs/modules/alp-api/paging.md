# `alp-api::paging`

## Purpose

Cursor pagination shared by the list tools (`track_list`, `media_list`,
`project_list`, `history_list`, `plugin_list`). MCP responses have a practical
size cap, so a client must be able to walk a long list in bounded pages without
computing offsets itself or silently looping when a page request is malformed.

## Public API

- `Page<T>` — one page: `items`, `total`, `offset`, optional `next_cursor`.
  `is_last()` reports whether `next_cursor` is absent.
- `PageRequest` — a resolved `{ offset, limit }` pair.
- `PageRequest::resolve(limit: Option<u64>, cursor: Option<&str>) -> Result<PageRequest, ApiError>` —
  absent `limit` uses `DEFAULT_PAGE_SIZE`; values above `MAX_PAGE_SIZE` are
  clamped (a client asking for "everything" still gets a well-formed page);
  `limit: 0` and a malformed cursor are validation errors.
- `encode_cursor(offset) -> String` / `decode_cursor(&str) -> Result<usize, ApiError>`.
- `paginate(items: Vec<T>, request) -> Page<T>` — slices an ordered list.
- Re-exported constants: `DEFAULT_PAGE_SIZE` (50), `MAX_PAGE_SIZE` (200).

## Data flow

1. The MCP layer reads `limit`/`cursor` from tool arguments and calls
   `PageRequest::resolve`.
2. The API list method builds the full ordered list and hands it to `paginate`,
   which slices it and emits `next_cursor` when items remain.
3. The client passes `next_cursor` back verbatim; it is an opaque token
   (`v1` + hex offset), never something a client should compute.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `crate::error::ApiError`

No external crates: the cursor is a short `v1<hex>` token rather than base64 so
`alp-api` stays dependency-free.

## Threading / real-time notes

Pure functions over owned data; no locks, no I/O, never used on the audio
callback.

## Error handling

`ApiError::Validation` for `limit: 0` and for any cursor that does not decode.
The message tells the caller exactly how to recover (pass `next_cursor` from
the previous page, or omit the cursor to restart). An offset past the end of
the list is *not* an error: it returns an empty page carrying the real `total`.

## Testing

Five unit tests in `paging.rs`:

- cursors round-trip and reject junk tokens (`""`, `v1`, `offset:3`, `v2a`, …);
- walking a 7-item list with `limit: 3` sees every item exactly once in order;
- default/clamped/zero limits behave as documented;
- an offset past the end yields an empty page with the real total;
- an exactly-full final page offers no cursor.

## Known limits

- Pages are offset cursors into the *current* list, not snapshots: if the
  project changes between pages, the next page continues from the same offset
  into the new list. Writes bump the project `revision`, so a caller that needs
  a stable sequence can compare `revision` between pages.
- There is no keyset pagination (no "items after this id"); the lists are small
  and offset cursors keep the contract simple.
