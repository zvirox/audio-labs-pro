# `alp-voice` / `text`

## Purpose

Provides text normalization (numbers, currency, abbreviations, ordinals, dates) and boundary-aware sentence and paragraph splitting.

## Public API

- `TextNormalizer`: `normalize`, `split_paragraphs`, `split_sentences`
- `number_to_words(n: i64) -> String`

## Data flow

1. `split_sentences` breaks prose into individual sentence units while protecting abbreviations (e.g. `Dr.`, `Mr.`, `etc.`).
2. `normalize` tokenizes words and converts numeric amounts, currencies (`$`, `€`, `£`), dates (`2026` -> `twenty twenty-six`), and abbreviations into spoken English words.

## Allowed dependencies

Standard library only.

## Threading / real-time notes

Pure string transformation; thread-safe and re-entrant.

## Error handling

Unrecognized tokens or malformed numbers are retained in their original text form without panics.

## Testing

Unit tests in `text.rs` verify normalization of numbers, currency, ordinals, and abbreviation-safe sentence splitting.

## Known limits

- English-only normalization rules.
