# Audio Labs Pro — AI Agent Operational Guide

Best practices and protocol conventions for AI models interacting with Audio Labs Pro.

---

## 1. Core Operating Principles

1. **Eyes and Ears First:** Always analyze before modifying. Call `project_info` and `analyze_loudness` before making edits, and re-analyze after edits to verify acoustic outcomes.
2. **Every Change is Undoable:** Every modification creates a project revision and pushes an undo step. When performing major restructuring, capture an explicit checkpoint with `checkpoint_create`.
3. **Session Rollback:** If an automated workflow goes astray, invoke `rollback_session` to instantly return the project to its state when the AI attached.
4. **Sample-Accurate Seconds:** All API time parameters (`split_secs`, `start_secs`, `end_secs`) accept fractional seconds (e.g. `12.345`). Internally, they are converted sample-accurately at the project sample rate.
5. **Sandbox Respect:** Only paths within the designated workspace can be read or written. Absolute paths outside the workspace or paths containing `..` will be rejected.

---

## 2. Standard Workflow: Voice Cleanup & Podcast Master

```
1. project_info        -> verify sample rate, track count, duration
2. analyze_loudness    -> inspect initial LUFS, true peak, noise floor
3. track_add           -> create narration or music tracks if needed
4. range_delete        -> cut silent gaps or unwanted sections with ripple editing
5. analyze_loudness    -> verify integrated loudness moved closer to -16 LUFS
6. export_audio        -> render master with "podcast" or "voice_over" preset
```
