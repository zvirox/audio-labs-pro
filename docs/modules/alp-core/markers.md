# `alp-core::markers`

## Purpose

Timeline marker, region and loop range data types.

## Public API

- `Marker`, `MarkerId`
- `Region`, `RegionId`

## Data flow

These types live directly on `Project` and serialise with the project file.

## Allowed dependencies

`alp-core` only.

## Threading / real-time notes

Plain data; no audio-thread use.

## Error handling

The `Project` validator rejects duplicate ids and empty labels.

## Testing

Covered through commands in `commands::tests`.

## Known limits

The timeline frame is not bounded to project duration yet.
