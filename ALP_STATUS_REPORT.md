# Audio Labs Pro — Build Plan Status Report

_Generated 2026-10-29 against `ALP_BUILD_PLAN.md` (10 phases, D1–D12, post-1.0 backlog)._
_Code state: `main` @ `ebc2177` · published release `v0.0.1-alpha.2` · all workspace gates green (fmt, clippy `-D warnings`, 24 test binaries)._

Evidence sources: `crates/*` sources, `docs/adr/0001–0021`, `docs/spikes/PHASE0–11-RESULTS.md`, `scripts/phase*-demo.sh`, `docs/mcp/*`, `packaging/*`, `.github/workflows/release.yml`.

---

## 1. Verdict at a glance

| Plan section | Status | Summary |
|---|---|---|
| Phase 0 — spikes & decisions | ✅ Done | ADR 0001–0010; glass/waveform, cpal, peaks, design & motion spikes measured; D1–D8 closed |
| Phase 1 — foundation | ✅ Done | 9 crates, CI + 3 guard scripts, undo stack, peak cache; measured in `PHASE1-RESULTS.md` |
| Phase 2 — UI shell & glass | ✅ Done¹ | alp-ui-kit + Inter, KWin/X11 blur + fallback, panel manager, palette, Reduce motion; 60 fps verified |
| Phase 3 — record & single-track edit | ✅ Done | Crash-safe capture + recovery, full edit toolset, WAV/FLAC export; null test 1.8e-7 |
| Phase 4 — multitrack & mixing | ✅ Done² | Multitrack model, markers/loop, pan law, presets, stems; 10-track render null error 0.00 |
| Phase 5 — effects & cleanup | ✅ Done³ | EQ/comp/limiter/gate/de-esser/de-hum/de-click, BS.1770 LUFS, Voice Cleanup, CLI `lufs`/`cleanup` |
| Phase 6 — model manager & TTS | ✅ Core + downloads + UI + engines | Real Kokoro-82M ONNX engine in `alp-infer` with genuine `LoadModel`; manifests hash-verified against verified bytes; `alp-cli model download` with resume + SHA-256 verification (ADR 0022); Models panel + Voice Lab Neural Tier card expose download/progress/load with daemon-first generation routing; Kitten Nano engine wired over a shared StyleTTS2 layer (ADR 0023); long-text chunked synthesis (ADR 0024) |
| Phase 7 — dictation & transcription | ⚠️ Partial⁴ | VAD, transcript model, SRT/VTT export, live dictation; STT is the built-in baseline, not Moonshine ONNX |
| Phase 8 — profiles & cloning | ⚠️ Partial⁵ | Profiles, consent gate, reference audit, design, cache, compare, hardware tiering shipped; **no heavy clone models** installed behind it |
| Phase 9 — polish & packaging | ✅ Done | deb/rpm/flatpak + GitHub release pipeline, app id, wizard, user guide, fuzz/stress tests |
| Phase 10 — MCP automation | ⚠️ Partial | Core works (14 tools, 3 tiers, attach mode, checkpoints/rollback, sandbox/audit); tool surface ≈ 14 of ~55 planned; no eval harness |
| Phase 11 — CLAP/LV2 hosting | ✅ Done (beyond plan) | Post-1.0 backlog item pulled forward: `alp-plugin` crate, Effects browser, CLI, sandboxed host |
| Release milestones | 🔀 Superseded | Plan’s `v0.1 "`Editor"` … v1.1 "Agent"` were never tagged; actual releases are `v0.0.1-alpha.1/.2` |

¹ UI direction later superseded by the REA-driven seven-workspace shell (see `plan.md`); ² MP3 export deliberately deferred (license); ³ no RNNoise noise reduction; ⁴ model *catalogue* exists but neural execution isn’t wired; ⁵ infrastructure complete, optional heavy tier missing.

---

## 2. What is implemented (per phase)

### Phase 0 — Spikes & decisions ✅
- Spikes: iced glass/waveform demo, cpal playback/capture harness, symphonia + peak-cache benchmark, Kokoro ONNX inference spike, Wayland blur-protocol probe (`spikes/`).
- Decisions D1–D8 closed in ADR 0001–0010; D9–D11 closed in ADR 0020 (Phase 10); D12 in ADR 0019 (Phase 9).
- Measured (recorded in `docs/spikes/PHASE0-RESULTS.md`): 60 fps 1-hour waveform scroll, 10.66 ms callback period, 1-hour decode+peaks in 998 ms.

**Remaining:** the cross-desktop/NVIDIA manual matrix (KDE Wayland, GNOME Wayland, X11, NVIDIA) has never been run end-to-end.

### Phase 1 — Foundation ✅
- Cargo workspace, CI (fmt/clippy/test/build) + guards `check-file-length.sh`, `check-deps.sh`, `check-docs.sh` (plus an extra `check-ui.sh` forbidding raw iced widgets in `alp-ui`).
- `alp-core` project model + command stack; `alp-media` decode/resample/peaks + ALPPEAKS disk cache; `alp-engine` cpal transport/mixer; `alp-cli` headless tools (`scripts/phase1-demo.sh`).
- Measured: peak-cache hit 0.013–0.015 s; 8×60 s clips → 2.88 M frames in 1.5 s; 10-minute soak no errors.

**Remaining:** none material.

### Phase 2 — UI shell & glass ✅
- `alp-ui-kit`: tokens, bundled Inter, buttons/toggles/sliders/dropdowns/tabs/tooltips/progress, cards, modal, toast, command palette, peak-pyramid waveform; motion via `lilt` with Reduce-motion.
- Transparent window; KWin/X11 `_KDE_NET_WM_BLUR_BEHIND_REGION` via x11rb (ADR 0012 supersedes ADR 0003); optimistic tint fallback.
- Panel manager (drag/resize/z-order/snap/collapse/dock), versioned-atomic `workspace.json` persistence, keyboard shortcuts.
- Measured: 60–61 fps 1-hour autoscroll; idle CPU 0.00 %.

**Remaining:** GNOME has no supported blur path in this build (platform limitation); no full accessibility tree / screen-reader labels / in-app text scale; `phase2-demo.sh` was never created (deviation from rule 9 — every other phase has a demo script). The floating-panel layout was later replaced by the docked seven-workspace shell (see §4).

### Phase 3 — Recording & single-track editing ✅
- Crash-safe capture (raw f32 spool + chunk-synced JSON manifest), `recover_capture`, peak/RMS input metering, monitor toggle, punch-in, drag-and-drop import with resample/conversion.
- Selection/snap model, cut/copy/paste/delete/silence/trim/split/duplicate, normalize/reverse/invert, envelopes, fades; WAV/FLAC export with bit depth, dither, tags (ADR 0013).
- Measured: 30-minute soak 86.4 M frames zero drops; forced-kill recovery 379,904 frames; export null test 1.8e-7 vs threshold 0.001.

**Remaining (recorded known gaps):** hardware acceptance ran on the default 48 kHz stereo input only; 30-min captures need ~1.5 GB spool; no project picker / save-as / recent list; import is synchronous and can briefly block the UI; normalization uses source peak, not a per-clip measurement; monitoring is a nearest-frame copy, not a filtered resampler.

### Phase 4 — Multitrack timeline & mixing ✅
- Per-track gain/pan/mute/solo/arm/colors, markers, regions, loop range, undoable commands; stereo balance pan law, master limiter, live meters (`alp-engine`).
- `ExportPreset` (podcast/audiobook/voice-over) with TPDF dither; stems export; direct preset buttons in Export Queue (ADR 0014).
- Measured: 10-track scene render null error 0.00.

**Remaining:** MP3 export deferred (LGPL — deliberate deviation recorded in ADR 0005; WAV/FLAC/OGG Vorbis ship), Opus and AAC are post-1.0; spectrogram view ✗; minimap ✗; multi-input overdub recording is only partially realized (arming exists in the model; multi-device capture untested).

### Phase 5 — Effects & voice cleanup ✅
- `alp-dsp`: RBJ biquads, 5-band ParametricEq, compressor, gate, peak limiter, DeEsser/DeHum (4 harmonics)/DeClick, BS.1770 K-weighting + `measure_lufs` + normalization gain; composable chains and the Voice Cleanup preset (ADR 0015).
- CLI `alp-cli lufs`, `alp-cli cleanup`; Effects panel wired with per-effect enable/bypass and loudness targets.

**Remaining:** RNNoise (`nnnoiseless`) noise reduction and the classical spectral fallback were never implemented — the plan’s noise-reduction requirement is unmet (de-hum/de-click only); silence/breath tools (auto-trim silence, pause shortening, breath reduction) are not in `alp-dsp` (they exist only as a segment-level range-delete in editing/MCP).

### Phase 6 — Model manager & built-in TTS ✅ (core, downloads, UI, both TTS engines)
**Implemented:** `ModelManager` catalogue (manifests for `kokoro-82m`, `kitten-nano`, `moonshine-tiny` with licenses/tiers), SHA-256 verification, atomic `.part` downloads, license-acceptance flow, offline mode; `alp-infer` daemon (Ping/LoadModel/Synthesize/…) over length-prefixed JSON Unix-socket protocol with in-process fallback; TextNormalizer + PronunciationDictionary (`lexicon.rs`); Script & ScriptLine model with gapless stitching; CLI `tts`/`model`/`script render --stitch` (ADR 0016). **As of this pass, the neural tier is real:** `alp-infer` hosts a Kokoro-82M engine (`kokoro.rs`) running the v1.0 fp32 export through `ort` 2.0.0-rc.15 (CPU, per ADR 0008) with `piper-plus-g2p` phonemization using the crate's bundled CMUdict (ADR 0010), per-voice style vectors and 24 kHz output. `LoadModel` genuinely loads and activates the engine; a failed load keeps the previously active engine and answers `ready: false` (pinned by tests). All manifest entries were rewritten from **locally verified bytes** (each SHA-256 computed after download and cross-checked against upstream LFS ids): Kokoro points at `onnx-community/Kokoro-82M-v1.0-ONNX` v1.0 (326 MB; was stale v0.19 with a nonexistent `voices.bin`), `kitten-nano` points at the Apache-2.0 `onnx-community/kitten-tts-nano-0.1-ONNX` export (was a fabricated `models.audiolabspro.internal` URL with a placeholder hash), and `moonshine-tiny` lists the real encoder + `decoder_model_merged.onnx` + tokenizer/configs (was encoder-only with an invented size/hash). End-to-end over the socket protocol: load 745 ms, 3.83 s of speech in 1.139 s (**RTF 0.298**, debug build), within the spike's 0.26–0.36 band.

**Remaining:** `alp-cli model download` now ships (ADR 0022): resumable per-file `.part` continuation, SHA-256 verification before the atomic rename, upstream-length agreement before transfer, disk-space pre-check, CLI progress line, and gates (built-in / offline / license — the last now persisted via `<storage>/licenses/<id>.accepted` markers so it survives across processes). Verified live against Hugging Face: kitten-nano (12 files, 22.7 MB) downloads and verifies, a rerun skips all 12 files with zero network, and a 1 MB pre-seeded `.part` resumed over a real CDN 206 (fetching exactly the 21.7 MB remainder). As of this pass the app exposes the neural tier end to end: the Models panel offers Download per model behind the persisted license gate with live progress bars and inline errors (an interrupted stream reports the exact byte boundary and the retry-to-resume contract), Voice Lab gained a Neural Tier card (voices, install status, on-demand `Check engine` / `Load engine` against the `alp-infer` daemon, one clear next action per row), and generation (Generate and Story renders) routes through the daemon when it runs, falling back to the built-in acoustic engine otherwise, with the job detail line naming the engine that actually rendered the audio. Both catalogue TTS engines are now wired (ADR 0023): `kitten-nano` loads and synthesizes through the same shared `style_text_to_speech_2` layer as Kokoro — RTF 0.12 single-line, the app's *Load engine* works — and long text is reconstructed from sentence chunks with per-chunk style buckets instead of one clamped token sequence (ADR 0024, 20-22 s paragraphs verified through both engines). Still open for this phase: `moonshine-tiny` is `ModelKind::Stt`, so its engine belongs to the Phase 7 dictation slice; the D3 heavy tier (Qwen3-TTS et al.) awaits its own downloads and decision.

### Phase 7 — Dictation & transcription ⚠️
**Implemented:** streaming/batch VAD (energy RMS + zero-crossing, hangover, onset backdating); `Transcript` model with SRT (`00:00:01,234`) / WebVTT / TXT export, sample-rate-aware SRT parsing, `cut_range_for_segment` for ripple edits; `DictationSession` with SpeechStarted/Partial/Final/Silence events; `BuiltinSttBackend` baseline; CLI `transcribe`/`vad`; `moonshine-tiny` manifest registered (ADR 0017). UI: Voice & Dictation Lab, transcript browser with click-to-seek, one-click ripple delete, transport dictation toggle.

**Remaining:** Moonshine ONNX execution (same `ort` gap — manifest exists, engine doesn’t); Whisper `.en` alternative (D6 option) not present; word-level timestamps ✗ (blocks MCP smart-edit filler removal); system-wide dictation injection under Wayland was never attempted (plan allowed in-app-only fallback — currently in-app only).

### Phase 8 — Voice profiles & cloning ⚠️
**Implemented:** `ProfileStore` (`$XDG_DATA_HOME/alp/voices`) with prebuilt profiles (“Aura”, “Atlas”, “Echo”, “Sol”); `ConsentRecord` human-only gate enforced at library level (also unreachable via MCP — verified in Phase 10 security tests); `analyze_reference_quality` (duration, clipping, noise floor, silence); `design_voice` brief-based design; SHA-256 `VoiceGenerationCache`; `compare_audio` A/B metrics; hardware probe + Fast/Best tiering; zero-dependency `.alpvoice` ZIP; full CLI profile lifecycle (ADR 0018). v0.0.1-alpha.2 hardened this whole surface (id traversal, export containment, consent-gate honesty, zip bounds).

**Remaining:** the optional heavy clone/design backends — Qwen3-TTS 0.6B/1.7B, VoxCPM2, OmniVoice — are **not shipped** (not even in the manifest catalogue). The pipeline, consent gate, storage, audit-quality checks and hardware tiering are complete and waiting for engines. The synthetic-voice metadata tag on exports (plan §8) is not evidenced in code.

### Phase 9 — Polish, packaging & 1.0 ✅
- Packaging: `.deb`, `.rpm` (pre-release ordering `0.0.1~alpha.x`), Flatpak manifest, AppImage build script, desktop file, hi-res SVG icon, MIME registrations, AppStream metainfo (OARS-1.1).
- GitHub release pipeline (`release.yml`): tag `v*` → deb/rpm/flatpak + sha256sums published as pre-releases. Published: `v0.0.1-alpha.1`, `v0.0.1-alpha.2`.
- Reliability: `reliability_and_stress.rs` fuzzing/corrupt-JSON/autosave/stress (50 tracks/500 clips/500 undos < 30 ms); first-run wizard; F1 shortcuts modal; USER_GUIDE, KEYBOARD_SHORTCUTS, ETHICS, TROUBLESHOOTING; THIRD_PARTY audit; app id `io.github.zvirox.AudioLabsPro` (D12, ADR 0019).

**Remaining:** the per-format “clean machine” checklist has only been exercised through CI’s single runner; the cross-desktop matrix (above) is still outstanding.

### Phase 10 — MCP automation ⚠️ (core done, breadth missing)
**Implemented as planned:**
- `alp-api`: typed operations, explicit handles (UUID-based since alpha.2), project revisions, session checkpoints + `rollback_session`, undo grouping, job manager with progress/cancel, 3-tier permission engine (Observe < Edit < Export; approval modes include `ConfirmDestructive` and `Review`), workspace sandbox (canonicalized, `..`/symlink-escape hardened in alpha.2), audit log (real SHA-256 arg digests since alpha.2).
- `alp-mcp`: hand-rolled typed JSON-RPC 2.0 MCP engine, spec revision 2026-07-28 (D9 — deliberate deviation from `rmcp`, ADR 0020); stdio transport + desktop attach mode via Unix control socket (D10); tiers D11; human-consent invariant enforced.
- Tool surface (**14 tools**): `project_info`, `project_save`, `history_undo/redo`, `checkpoint_create/restore`, `rollback_session`, `track_list/add/remove`, `clip_split`, `range_delete`, `analyze_loudness`, `export_audio` (podcast/voice_over/master presets — dispatch matched to advertisement in alpha.2). Resources: `alp://project/active`, `alp://effects/catalog`, `alp://docs/agent-guide`. Prompts: `clean_up_voice_recording`, `produce_podcast_episode`.
- GUI: Settings → AI control (tier, folders, approval), AI activity panel with **Stop AI** and **Undo all AI changes**, persistent AI indicator.
- Docs: `docs/mcp/TOOLS.md`, `WORKFLOWS.md`, `SECURITY.md`, `CLIENT_SETUP.md`, `AGENT_GUIDE.md`; `scripts/phase10-demo.sh`.

**Remaining vs plan §10:**
- **Tool breadth (~55 planned / hard cap 60 → 14 shipped):** missing `media_import`, full track/clip CRUD (`track_update`, `clip_add/update/remove`, `marker_add`, `edit_batch`), all effect tools (`effect_apply`, `preset_apply` — only a read-only catalog resource exists), all smart edits (`edit_trim_silence`, `edit_shorten_pauses`, `edit_remove_fillers`, `edit_cut_transcript_ranges`, `edit_match_loudness`), all voice tools (`tts_voices`, `tts_generate`, `tts_script_generate`, `tts_regenerate`, `voice_design`, `voice_profile_get`), transcription tools, most analysis (`analyze_silence/quality/spectrum`, `render_waveform`, `render_spectrogram`, `compare_renders`), stems/transcript export, `preview_render`, `job_status`/`job_cancel` (job manager exists in `alp-api` but isn’t exposed over MCP).
- **Gap-fill domain features (§10.4):** auto-ducking ✗, room-tone fill ✗, inter-clip loudness matching ✗, word-level transcript cutting ✗ (only segment-level exists), filler-word detection ✗, batch apply ✗, deterministic `seed` mode ✗.
- **Eval harness (§10.8):** no `docs/mcp/EVAL.md`, no scripted-agent replays, no LLM-in-the-loop trials — the plan’s feedback loop for tuning tool descriptions never ran.
- **Transport extras:** Streamable HTTP with token/Host/Origin validation ✗ (stdio + socket only — stdio-default is compliant with the plan); Tasks-extension negotiation and polling fallback over MCP ✗; pagination/size caps (~20 KB) and rate limits/timeouts/concurrency caps are not evidenced in `alp-api`/`alp-mcp`.
- **Review approval mode:** enum exists; the staged-checkpoint accept/reject flow (marked “stretch”) is not built.
- Attach-mode latency (AI edit visible ≤ 200 ms) is not measured anywhere.

### Phase 11 — CLAP/LV2 plugin hosting ✅ (beyond plan; was post-1.0 backlog)
- `alp-plugin` crate: CLAP 1.2+ and LV2 hosts, C-ABI loading with real-time-safe isolation rules, scanner/cache, `alp-plugin` sandboxed host process; Effects workspace hosted-plugin browser with Rescan; CLI scan/list/inspect; ADR 0021; `scripts/phase11-demo.sh`.

### Post-backlog items still not started
Time-stretch/pitch (license check pending: Rubber Band GPL vs Signalsmith MIT), spectral editing, macros, MIDI, Opus/AAC exports, authenticated remote MCP, multi-client MCP sessions.

---

## 3. Decisions D1–D12 — final state

| D | Topic | Resolution | ADR |
|---|---|---|---|
| D1 | License | Apache-2.0, Linux-only, English-only | 0001 |
| D2 | UI toolkit | iced 0.14 pinned (Tauri/Slint rejected in spike) | 0002 |
| D3 | Heavy-model runtime | **Open in practice** — no heavy runtime shipped; spike explored GGUF/ggml vs `candle` vs sidecar; production uses the worker-process + builtin backend | 0008 |
| D4 | Internal audio format | f32 / 48 kHz default, per-project override | 0004 |
| D5 | Export formats | WAV + FLAC + OGG Vorbis; **MP3 deferred** (LGPL); Opus later | 0005 |
| D6 | Dictation model | Moonshine planned (manifest present); executing engine is the built-in baseline STT | 0006 |
| D7 | Component library | In-house `alp-ui-kit`; `iced-plus`/`iced_aw`/`iced_ui` rejected | 0007 |
| D8 | Animation crate | `lilt` (panel spring settle deferred, recorded) | 0007 |
| D9 | MCP SDK/spec | Hand-rolled typed JSON-RPC, rev 2026-07-28 (no `rmcp` dependency) | 0020 |
| D10 | Live transport | stdio default + Unix control socket attach (`$XDG_RUNTIME_DIR/alp/control.sock`) | 0020 |
| D11 | AI permissions | 3 tiers, session checkpoint, confirm-destructive default | 0020 |
| D12 | App ID | `io.github.zvirox.AudioLabsPro` (desktop/MIME/metainfo) | 0019 |

---

## 4. Deviations from the plan worth knowing

1. **Release scheme superseded.** Plan milestones `v0.1 "Editor"` → `v1.1 "Agent"` were never tagged. Actual published releases: `v0.0.1-alpha.1` (2026-10-28) and `v0.0.1-alpha.2` (2026-10-29); a release-history inherited from a reference app was removed from changelog/metainfo on `main`.
2. **UI direction changed.** The plan’s Motrix-style floating-panel glass layout was replaced by the seven-workspace shell (nav rail, per-workspace card canvas, docked player, `ALP_START_TAB/OVERLAY` env pins) after the REA pass documented in `plan.md`. Grid/panel motion, tokens and Reduce motion survived the change.
3. **Model strategy is currently “infrastructure without engines.”** Manager, manifests, daemon protocol, consent and licensing flows all exist; the executing synthesis/recognition engines are deliberately dependency-free builtins pending the `ort`/ONNX integration.
4. **MCP is a working subset**, built bottom-up (history/rollback + edit + one analysis + export) rather than the plan’s full breadth-first surface.
5. **Plugin hosting happened early** (Phase 11) — a post-1.0 backlog item.

---

## 5. Remaining work, prioritized

**P0 — make the model tier real (`ort`/ONNX).**
Kokoro-82M TTS and Moonshine STT execution behind the existing `alp-infer` protocol; heavy clone tier (Qwen3-TTS 0.6B/1.7B minimum) behind the existing `CloneBackend`/consent/profile plumbing; synthetic-voice export tag. This is the single biggest gap between plan and product: everything around the models is built.

**P1 — finish Phase 10 breadth.**
Expose the job manager (`job_status`/`job_cancel`), `media_import`, effect application, TTS/profile tools, transcription tools, smart edits (requires word-level timestamps first), `edit_batch`, pagination + size/rate limits, then build the §10.8 eval harness (`EVAL.md` + scripted replays) and re-tune tool descriptions. Optional: Streamable HTTP transport; Review-mode staged checkpoints; attach-latency measurement.

**P2 — Phase 5 completion.** RNNoise/spectral noise reduction (the one plan-required DSP block that never landed); silence/breath tools propagated from editing to DSP.

**P3 — recording/timeline depth.** Multi-input overdub, spectrogram + minimap views, monitoring resampler, per-clip measured normalization, asynchronous import, monitoring-filter, project picker/save-as/recent projects.

**P4 — release assurance.** Cross-desktop matrix (KDE/GNOME Wayland, X11, NVIDIA; PipeWire + ALSA); per-format clean-machine release passes; add the missing `phase2-demo.sh`; MP3/Opus exports if licensing is revisited.

**P5 — engineering debt.** The 16-finding review pass (unused job-cancel plumbing in export closures, same-rate resample copy, FLAC buffer copies, duplicated decodes on export, dead envelope parameters, error flattening) from the 2026-10-28 audit — none are correctness bugs, but several are the same hot paths the Phase 10 export tools will lean on.
