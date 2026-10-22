# Audio Lab Pro

A Linux-only, English-only audio studio written in Rust: multitrack
recording/editing/mixing/export plus local AI text-to-speech, dictation and voice
profiles. Fully offline — the only network traffic is user-initiated model
downloads.

- License: [Apache-2.0](LICENSE)
- Platform: Linux (Wayland and X11)
- UI: iced 0.14 with an in-house component kit, frosted-glass floating panels

## Status

Phase 0 (research spikes and decisions) is in progress. See:

- [Documentation index](docs/README.md)
- [Architecture](docs/architecture/ARCHITECTURE.md)
- [Phase 0 results](docs/spikes/PHASE0-RESULTS.md)
- [Third-party licenses](THIRD_PARTY.md)

No application crates exist yet; Phase 1 creates the workspace.
