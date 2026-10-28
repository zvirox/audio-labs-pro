# `alp-voice` / `lexicon`

## Purpose

Provides custom pronunciation dictionary management and inline speech markup parsing for pauses (`[pause=500ms]`) and emphasis.

## Public API

- `SpeechToken`: `Words(String)`, `PauseMs(u32)`, `Emphasis(String)`
- `PronunciationDictionary`: `new`, `add`, `lookup`, `remove`, `apply`
- `MarkupParser`: `parse`

## Data flow

1. `PronunciationDictionary::apply` performs case-insensitive word replacement across input text.
2. `MarkupParser::parse` extracts bracketed tags into a sequence of structured `SpeechToken` variants for synthesis timing control.

## Allowed dependencies

- Standard library collections (`std::collections::HashMap`).

## Threading / real-time notes

Pure string processing; thread-safe and re-entrant.

## Error handling

Unrecognized tags or malformed durations fall back to default pause intervals (250 ms) without panicking.

## Testing

Unit tests in `lexicon.rs` verify case-insensitive dictionary replacements and structured token parsing of mixed text, pause, and emphasis tags.

## Known limits

- Only basic pause durations and emphasis tags are supported; complex SSML trees are intentionally avoided per build plan minimalism.
