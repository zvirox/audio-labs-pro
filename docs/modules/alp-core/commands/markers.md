# # `alp-core::commands::markers`

## Purpose

Commands for markers, regions and the loop range.

## Public API

Commands for markers, regions and the loop range.

## Data flow

Commands keep sorted order by frame or start frame and store `Option` / tuple snapshots so undo restores labels and positions.

## Allowed dependencies

`alp-core` only.

## Threading / real-time notes

No audio-thread use.

## Error handling

Empty labels and invalid ranges are rejected before mutation.

## Testing

Interleaving marker and region commands is covered conceptually and now has tests.

## Known limits

Frame ranges are not constrained to project length yet.
