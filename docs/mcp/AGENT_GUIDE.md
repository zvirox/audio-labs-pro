# Audio Labs Pro — AI Agent Operational Guide

Best practices and protocol conventions for AI models interacting with Audio Labs Pro.

---

## 1. Core Operating Principles

1. **Eyes and Ears First:** Always analyze before modifying. Call `project_info` and `analyze_loudness` before making edits, and re-analyze after edits to verify acoustic outcomes.
2. **Every Change is Undoable:** Every modification creates a project revision and pushes an undo step. When performing major restructuring, capture an explicit checkpoint with `checkpoint_create`.
3. **Optimistic Concurrency:** Any write tool accepts `expected_revision` — pass the `revision` you saw in `project_info`. If another client edited the project since, the write fails *without changing anything* and tells you to re-read; retry from a fresh `project_info`.
4. **Session Rollback:** If an automated workflow goes astray, invoke `rollback_session` to instantly return the project to its state when the AI attached.
5. **Sample-Accurate Seconds:** All API time parameters (`split_secs`, `start_secs`, `end_secs`) accept fractional seconds (e.g. `12.345`). Internally, they are converted sample-accurately at the project sample rate.
6. **Sandbox Respect:** Only paths within the designated workspace can be read or written. Absolute paths outside the workspace or paths containing `..` will be rejected.
7. **Paginated Lists:** `track_list`, `media_list`, `project_list`, `history_list` and `plugin_list` return `{ items, total, offset, next_cursor? }`. Pass `next_cursor` back verbatim for the next page; do not compute offsets yourself.

---

## 2. Standard Workflow: Voice Cleanup & Podcast Master

```
1. project_info        -> verify sample rate, track count, duration
2. analyze_loudness    -> inspect initial LUFS, sample peak, noise floor
3. track_add           -> create narration or music tracks if needed
4. range_delete        -> cut silent gaps or unwanted sections with ripple editing
5. analyze_loudness    -> verify integrated loudness moved closer to -16 LUFS
6. export_audio        -> render master with "podcast" or "voice_over" preset
7. dsp_macro_execute   -> batch-process a directory of files with a DSP macro chain (poll with job_status)
8. acx_qa_analyze      -> check chapter WAVs against ACX targets before export
9. audiobook_export_m4b -> mux a rendered master into a chaptered M4B
```
