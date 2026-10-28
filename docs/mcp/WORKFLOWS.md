# Audio Labs Pro — MCP Workflow Recipes

Step-by-step tool invocation sequences for common audio engineering tasks.

---

## 1. Clean Up Voice Recording

**Goal:** Remove room noise, balance vocals, and master voice track to standard loudness.

1. `project_info`: verify session state.
2. `analyze_loudness`: check baseline levels.
3. `range_delete`: remove dead air or coughs at start/end.
4. `analyze_loudness`: confirm true peak $\le -1.0$ dBFS and loudness $\approx -16.0$ LUFS.
5. `export_audio`: preset `"voice_over"`, `output_path: "voice_clean.wav"`.

---

## 2. Produce Podcast Episode

**Goal:** Assemble voice narration with intro music bed and master for podcast distribution.

1. `track_add`: `name: "Voice"`.
2. `track_add`: `name: "Music Bed"`.
3. `clip_split`: split music bed to align under speech intro.
4. `analyze_loudness`: verify overall mix dynamics.
5. `export_audio`: preset `"podcast"`, `output_path: "episode_master.flac"`.
