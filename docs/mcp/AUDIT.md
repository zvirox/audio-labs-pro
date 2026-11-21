# MCP Tool Surface Audit — November 2026

This is the record of what the `alp-mcp` surface actually did before the audit,
what it does now, and what remains honest-but-partial. The rule applied
throughout: a tool either produces the artefact it names or it fails with an
explanation. Reporting success for work that did not happen is a bug, not a
stub.

Related: [`TOOLS.md`](TOOLS.md) (reference), [`EVAL.md`](EVAL.md) (evaluation
plan), [`SECURITY.md`](SECURITY.md), [`WORKFLOWS.md`](WORKFLOWS.md).

---

## 1. Inventory

`crates/alp-mcp/src/tool_defs.rs` defines **58 tools**; the dispatcher in
`dispatch.rs` matches all 58, and `every_advertised_tool_is_dispatched` in
`main.rs` fails if that ever stops being true. Before the audit
`MASTER_CHECKLIST.md` claimed 22 shipped and listed most of them as missing —
the checklist, not the server, was stale.

| Group | Tools | Tier |
|-------|-------|------|
| Projects & history | `project_list`, `project_info`, `project_create`, `project_open`, `project_save`, `project_close`, `history_list`, `history_undo`, `history_redo`, `checkpoint_create`, `checkpoint_restore`, `rollback_session` | Observe / Edit |
| Media | `media_list`, `media_import` | Observe / Edit |
| Tracks & clips | `track_list`, `track_add`, `track_update`, `track_remove`, `clip_add`, `clip_update`, `clip_split`, `clip_remove`, `range_delete`, `marker_add`, `edit_batch` | Observe / Edit |
| Effects | `effect_catalog`, `effect_apply`, `effect_update`, `effect_remove`, `preset_apply` | Observe / Edit |
| Smart edits | `edit_trim_silence`, `edit_shorten_pauses`, `edit_remove_fillers`, `edit_match_loudness` | Edit |
| Voice & TTS | `tts_voices`, `voice_profile_get`, `tts_generate`, `tts_script_generate`, `tts_regenerate`, `voice_design` | Observe / Edit |
| Transcription | `transcribe` | Observe |
| Analysis | `analyze_loudness`, `analyze_silence`, `analyze_quality`, `analyze_spectrum`, `render_waveform`, `render_spectrogram`, `compare_renders` | Observe |
| Export | `export_presets`, `export_audio`, `export_stems`, `export_transcript`, `preview_render` | Observe / Export |
| Jobs | `job_status`, `job_cancel` | Same as job |
| Plugins | `plugin_list`, `plugin_insert`, `plugin_param_set` | Observe / Edit |

Not shipped: `transcript_get` (search/paging over transcripts) and
`edit_cut_transcript_ranges` (needs word-level timestamps, see §4).

---

## 2. Fabricated behaviour — found and fixed

Every row below was verified by reading the implementation, then by a test that
fails against the old behaviour.

| Tool | Was | Now |
|------|-----|-----|
| `export_stems` | Wrote the **master mix** once per track under N file names | Renders each track in isolation (`render_track_mixdown`) and writes `stem_NN_name.wav`; the report carries that stem's own peak. Test: `stems_carry_their_own_track_not_the_master` asserts a 20 dB track difference survives and the stems differ sample-by-sample |
| `export_transcript` | Wrote a one-line header, never transcribed | Renders the project, runs the real transcription path (daemon STT or the built-in segmenter), and writes `text`/`srt`/`vtt` from `Transcript::to_plain_text`/`to_srt`/`to_vtt`. Report names the engine and segment count |
| `preview_render` | Returned numbers; produced no file | Slices the rendered mix, writes a real WAV into the media dir, reports the slice's own peak and duration |
| `render_waveform` | Computed an envelope, then returned a stub `data_uri` | Returns the ASCII art it computed in `ascii`, plus peak and duration. Column peaks are real, so a tone appears exactly where it is in time |
| `render_spectrogram` | Same stub `data_uri` | Returns per-column Goertzel band energy as ASCII with `band_centers_hz` (log-spaced, low at the bottom) and `range_db` |
| `edit_batch` | Returned `operations_applied: 1` while applying **nothing** | Accepts an `operations` array of 8 real operation kinds, applies them in order, and coalesces them into one `CompositeCommand` undo entry. A failing operation rolls the whole batch back and reports "no changes were applied" |
| `tts_script_generate` | `Ok(())` no-op with no output | Parses `{"segments":[...]}` or a bare array, synthesizes each line, imports the audio, lays clips out on the `Script` track with per-line gaps, reports clip ids/engines/durations |
| `tts_regenerate` | Synthesized a new file and left the clip alone | Replaces the clip's audio in place: same track, same start, one undo step |
| `edit_trim_silence` | Report-only: measured silence, cut nothing | Finds silent runs ≥ `min_silence_secs` and ripple-deletes them; reports measured before/after lengths |
| `edit_shorten_pauses` | Report-only | Keeps `max_pause_secs` of each long pause and cuts the excess before the next onset |
| `edit_remove_fillers` | Counted filler *mentions* and reported them removed | Cuts only from real word timestamps; refuses when the only timestamps are placeholder words (see §4) |
| `edit_match_loudness` | Applied gain with an inverted sign (`actual - target`) | Measures, applies `target - before` scaled per track, re-measures, and reduces the gain if the sample peak would pass the −1 dBFS ceiling; the report says when it was limited |
| `analyze_quality` / `analyze_loudness` | Peak was labelled "true peak"; noise floor was the smallest non-zero sample; "silence ratio" counted near-zero samples | Real 20 ms frame RMS statistics: sample peak, 10th-percentile noise floor, fraction of active frames, clipping count, BS.1770 loudness |
| `analyze_spectrum` | Fixed magnitudes with no relation to the render | 16 log-spaced Goertzel bands averaged over up to 64 blocks across the whole render |
| `analyze_silence` | Flagged any dip below threshold | Honours the ≥100 ms minimum run length and reports the threshold and minimum it used |
| `effect_apply` / `preset_apply` | Accepted any effect or preset string and inserted an inert chain entry | Validate against `effect_catalog` and fail with the known ids. Tests: `an_unknown_effect_id_is_rejected_with_the_catalog`, `an_unknown_preset_is_rejected_and_a_known_one_applies` |
| `job_status` / `job_cancel` | Job manager existed but nothing ever created a job | `tts_generate`, `transcribe` and `tts_script_generate` submit a real job, complete or fail it, and return `job_id` in their report |
| `alp://docs/agent-guide` resource | Inline two-sentence stub | Serves `docs/mcp/AGENT_GUIDE.md` with `include_str!`, so the resource cannot drift from the file |
| `history_list` | Fabricated labels: "Undoable edit step #1" derived from an availability flag | Reports the real command labels from the undo/redo stacks (`Rename track`, `Add clip`, …) with `index` and `is_undo`; `project_info.undo_steps`/`redo_steps` are real counts that agree with the list. Test: `history_list_reports_the_real_command_labels` |
| `alp://effects/catalog` resource | Hard-coded list of seven effect names | Serialises the live `session.effect_catalog()` |
| Prompts | `clean_up_voice_recording` existed; `produce_podcast_episode` was advertised but unimplemented | Both return real recipes that name tools that exist |

Field renames that removed false claims:

- `true_peak_dbfs` → `peak_dbfs` (sample peak; inter-sample true peak is not
  measured yet).
- `silence_ratio` → `active_ratio` (fraction of 20 ms frames above −45 dBFS,
  which is what was actually being computed).
- `speech_ratio` removed; `analyze_quality` reports `active_ratio` instead.

UI copy that claimed a live control socket was corrected in the same pass:
the AI card now says the MCP server is a separate process, the stop action is
"Revoke attach" (it cannot terminate a client's process), and "Undo AI changes"
became "Undo last edit" because that is what it does. See §5.

---

## 3. Test coverage added

| Test | File | Proves |
|------|------|--------|
| `stems_carry_their_own_track_not_the_master` | `crates/alp-api/tests/export_and_script.rs` | Per-track rendering |
| `transcript_export_contains_real_words_and_timestamps` | same | SRT content matches the report text |
| `preview_render_writes_the_requested_slice` | same | The written WAV is the requested length |
| `a_script_becomes_clips_on_its_own_track_in_one_undo_step` | same | Script → clips, undoable in one step |
| `regenerating_a_clip_replaces_its_audio_in_place` | same | No duplicated clip |
| `a_batch_of_edits_is_atomic` | same | Batch applies and groups |
| `a_batch_applies_every_operation_as_one_undo_step` | `alp-api/src/track_clip_ops.rs` | One undo reverts both operations |
| `a_failing_batch_leaves_the_project_untouched` | same | Rollback + revision restored |
| `trim_silence_actually_cuts_and_reports_the_new_length` | `alp-api/src/smart_edit_ops.rs` | Timeline shrinks |
| `shorten_pauses_keeps_the_allowed_pause_and_cuts_the_excess` | same | Pause maths |
| `filler_removal_refuses_when_only_placeholder_words_exist` | same | Refusal instead of guess |
| `match_loudness_applies_real_gain_and_respects_the_peak_ceiling` | same | Gain direction + ceiling |
| `edit_batch_applies_real_operations_and_collapses_into_one_undo` | `alp-mcp/src/main.rs` | MCP-level: real rename, one undo |
| `a_failing_edit_batch_reports_no_changes` | same | MCP-level atomicity |
| `render_waveform_returns_the_art_it_computed` | same | Art in text and structured content |
| `render_spectrogram_returns_a_real_time_frequency_map` | same | 220 Hz lands in its own band |
| `trim_silence_through_mcp_actually_shortens_the_timeline` | same | End-to-end duration change |
| analysis unit tests | `alp-api/src/analysis_ops.rs` | Percentile, level stats, Goertzel |
| composite unit tests | `alp-core/src/composite.rs` | Grouped apply/revert and mid-group failure |
| paging unit tests | `alp-api/src/paging.rs` | Cursor round-trip, single-pass walk, clamped limits, empty last page |
| limits unit tests | `alp-mcp/src/limits.rs` | Payload cap, sliding window refill, honest timeout reporting |
| `every_advertised_tool_declares_a_title_and_annotations` | `alp-mcp/src/main.rs` | Metadata table covers all 58; hints match reality |
| `list_tools_paginate_and_report_total` | `alp-mcp/src/main.rs` | Page envelope, zero-limit and junk-cursor rejections, past-end page |
| `history_list_reports_the_real_command_labels` | `alp-mcp/src/main.rs` | Real labels; `undo_steps` agrees with the list |
| `expected_revision_guards_writes_without_touching_state` | `alp-mcp/src/main.rs` | Stale guard refuses with no state change; current guard accepted |
| `reads_ignore_expected_revision_and_write_tools_advertise_it` | `alp-mcp/src/main.rs` | Guard is write-only in both behaviour and schema |
| `the_rate_limit_refuses_the_call_it_cannot_serve` | `alp-mcp/src/main.rs` | Refusal names the limit and the recovery |
| revocation_discards_a_late_worker_edit_without_touching_live_history | `alp-ui/src/app/ai_control.rs` | Stop AI rejects a prepared completion; project and history are unchanged |
| complete_mcp_catalog_edits_live_history_and_publishes_measured_changes | same | Real socket + full MCP catalog: live track edit, one shared undo/redo, AI/operator notifications, measured request-to-commit latency |
| a_worker_completion_cannot_overwrite_a_concurrent_operator_edit | same | A scratch edit based on an old revision cannot overwrite the subsequent human edit |
| exact_frame_boundary_is_readable_and_oversize_writes_are_rejected | `alp-api/src/control.rs` | 2 MiB framing has consistent reader/writer boundary behavior |
| response_timeouts_preserve_fragmented_frames | same | A change event split across a 500 ms read timeout is reassembled instead of corrupting the next frame |
| a_slow_subscriber_is_dropped_when_its_bounded_queue_fills | same | A subscriber that falls 64 changes behind is removed without blocking `publish` |
| attached_undo_restores_the_path_and_audio_context_of_the_original_project | `alp-ui/src/session/tests.rs` | Undo/redo restores project root and decoded samples when two projects reuse UUIDs |
| a_read_started_before_a_human_edit_is_not_returned_as_current | `alp-ui/src/app/ai_control.rs` | A worker read snapshot older than the live revision fails with both revision numbers |

---

## 4. Still partial — stated plainly

| Area | Limit |
|------|-------|
| Word-level timestamps (`10.0.3`) | Not implemented. `edit_remove_fillers` therefore refuses unless a daemon STT model with word alignment is running; transcript-based cutting is unavailable |
| Filler detection (`10.4.5`) | Word list exists (user-supplied or a conservative default) but detection depends on the missing alignment |
| Hum detection | `analyze_quality.detected_hum_hz` is always `None`. A narrowband mains test is not implemented and guessing a frequency would be fabrication |
| True peak | Sample peak only. No oversampled inter-sample measurement |
| Spectrum / spectrogram | Goertzel probes at log-spaced centres, not an FFT. Accuracy is good for band energy, and the docs say so |
| `preset_apply` | Validates the name against the catalog now, but still replaces the chain with a single `preset:<name>` ref. `voice_cleanup` / `podcast_master` are labels, not expanded per-effect chains |
| Derived DSP during render | Effects attached to a track are recorded in the project but the offline renderer does not yet apply them; `effect_apply` and `preset_apply` therefore change the project, not the audio |
| Export paths | `export_audio`, `export_stems`, `export_transcript` and `preview_render` are synchronous; only TTS/STT/script report jobs |
| `transcribe` with `engine: "builtin"` | Emits placeholder words ("Speech segment N") because no model is loaded. The field admits this; the text is not real speech recognition |
| Tool schema metadata | ~~`McpTool` carries `name`, `description`, `inputSchema` only~~ Fixed: every tool now carries `title` plus `annotations` (`readOnlyHint`/`destructiveHint`/`idempotentHint`/`openWorldHint`) derived from one table, and `tools/list` returns `ttlMs`/`cacheScope`. `outputSchema` is still missing |
| Pagination | ~~No `limit`/`cursor` on list tools~~ Fixed: all five list tools page (`alp-api/src/paging.rs`, default 50 / max 200, opaque cursors, `total` + `next_cursor`, malformed cursor = validation error). No ~20 KB text cap yet |
| Transports | stdio only. The Streamable HTTP transport (127.0.0.1, Host/Origin checks, bearer token) is not implemented |
| Rate limits / timeouts / payload caps | ~~Not implemented~~ Fixed: sliding-window rate limit (600/60 s), 1 MiB payload cap, advisory 120 s call budget — CLI-tunable, published in `server/discover`, and an over-budget call returns its real result with a note. No max-concurrent-jobs cap (jobs are synchronous) |
| `expected_revision` | ~~`verify_revision` existed but no write path accepted it~~ Fixed: every write tool accepts `expected_revision`; a stale guard fails before changing anything, reads ignore the argument. No cross-process file locking, so one-writer-per-project remains a convention |

---

## 5. Control host status (`10.5.2`)

The control host is implemented in `alp-api::control` and embedded by
`alp-ui::app::ai_control`. Enabling **Settings → AI control** binds
`$XDG_RUNTIME_DIR/alp/control.sock`; startup fails closed if the runtime
variable is absent or the directory/path is unsafe. The `alp` directory is
`0700`, the socket is `0600`, regular files/symlinks/foreign sockets are never
removed, and every accepted connection is checked against the app's effective
UID with `SO_PEERCRED` before its first byte is parsed.

Each enable creates a fresh random token (two UUIDv4 values), available only from the running UI.
The authentication handshake precedes all project operations. A new sanitized
`client_id` displays a human approval dialog with **Deny**, **This session**, and
**Always allow client**; the persistent choice stores only that ID, never the
token. Stop AI revokes active clients and removes the socket. Framing is capped
at 2 MiB, partial reads survive timeouts, and socket workers wait for replies
off the UI thread. An oversized MCP result becomes a bounded error rather than
being truncated.

`alp-mcp --attach` forwards complete JSON-RPC envelopes over protocol version 2;
it does not initialize a workspace or open a disk project. The desktop exposes
the shared `McpServer` catalog from `alp-mcp`, not an attached-only list. Each
approved client has a worker and a revision-tagged, scratch `ApiSession`. Every
call rebases that scratch state from the live GUI project and dispatches through
the same code as headless mode. Only an unchanged live revision can merge a
successful result into `EditSession`, where it becomes one live undo entry.
`history_list`, `history_undo` and `history_redo` operate on that live stack.
Decode, analysis, rendering, generation and export run on the worker, not the UI
thread. `rollback_session` refuses in attached mode because its old scratch
checkpoint could otherwise discard intervening human edits. An attached save
that races a human edit reports the conflict even though the worker may already
have written its older snapshot.

`ControlRequest::Subscribe` opens an explicit change feed. After an operator or
attached edit commits and the view model is synchronized, the feed publishes
project summary, origin, operation and (for attached edits) measured
request-to-commit time. The attach proxy gates these native events on MCP
`resources/subscribe` and emits both `notifications/resources/updated` and the
documented `notifications/alp/project_changed` extension. The proxy keeps a
partial frame across timeouts, and the host keeps at most 64 pending changes per
subscriber, dropping a slow reader instead of blocking publication or revocation.
Every attached result reports `snapshotRevision` and `liveRevision`; a call whose
snapshot is older than the live project fails rather than superseding a newer
change event. Attached undo/redo restores the project root and decoded audio
alongside the project, even when a copied project reuses UUIDs. Calls are
currently serialized per attached connection; there is no claim of pipelining,
generic cancellation, or actual pixel-presentation timing.

The production integration tests authenticate against the real host, compare the
attached catalog with `tools_for_tier`, perform `track_add`, use shared history
to undo and redo it, and observe operator and AI notifications on another real
socket. A cache-ready structural call includes actual elapsed worker decoding
and live commit latency and must stay within 200 ms. Separate tests prove stale
conflict rejection, late-result revocation, and the exact framing boundary.
