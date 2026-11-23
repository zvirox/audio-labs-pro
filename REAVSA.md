# REAVSA — Record of External Vendor Static Analysis

> Analysis of the installed targets used by Audio Labs Pro as reference material.
> Generic labels only: **target-A** = the installed voice-cloning app; **target-B** = the installed audio-editing app.
> All names of the analyzed targets are replaced by these generic labels; the
> app-specific names themselves (product names, window classes, binary file names) are not used here.
>
> **Evidence basis:** REA artifact summaries (installed binary identity, sha256 digests,
> static sizes) plus the installed-target source inventory: the Electron renderer tree, the
> FastAPI backend (all route decorators), the terminal CLI, the Python MCP server, and the
> installed system metadata (desktop entry, AppStream metainfo, man page, package files).
> Claims that come from the vendor's upstream documentation rather than from installed artifacts
> are labelled "(vendor manual)" — they are secondary evidence, not static facts of the install.
> "ALP" = the Audio Labs Pro project in this repository (`~/Projects/Linux/audio-labs-pro`,
> `main` at the `v0.0.1-alpha.5` release baseline).

---

## 1. Identity and inventory of the two targets

| | target-A | target-B |
|---|---|---|
| Binary | installed voice-cloning app binary (voice-cloning launcher under the app's `/usr/bin`-style bin dir) | installed audio-editing app binary at `/usr/bin/` |
| REA digest (sha256) | `90bf90e0dd057632c76e5b286a2e068f9fb9cb1e1363ebaa215a17cb2e3d430e` | `9c1d07fd83e629d558fe7185c4be0adb4f02be0e9dad992167d9d5b8f95e78fa` |
| Static size | 289,429,504 bytes (~275 MB) | 14,756,224 bytes (~14.1 MB) |
| Package registration | none found (app installed as binaries + desktop entry) | not registered in dpkg/apt on this host (manual install) |
| Desktop entry | installed app entry: `Name`/`GenericName` (voice-clone platform), `Categories` `AudioVideo;Audio;AudioVideoEditing` | installed entry: `Name`/`GenericName` = "Sound Editor", `Categories` `AudioVideo;Audio;AudioVideoEditing`, `StartupWMClass` matching the app name |
| Docs dir | — | `/usr/share/doc/` for the app (LICENSE.txt, README.md) + AppStream metainfo (release 3.7.9, dated 2026-08-19) |

Source inventory (installed-target side):

- **target-A**: app source tree `/home/zvirox/Downloads/VoiceStudio-main` (~4,566 files):
  Electron renderer (`electron/src/renderer/src/…`), FastAPI backend (`backend/…`, ~393 route
  handlers across `api/routers/*`), terminal CLI (`cli/voicestudio_cli/…`), Python MCP server
  (`backend/mcp_server.py`), plus vendor app metadata.
- **target-B**: `/usr/bin/` binary + `/usr/share/audacity/` assets: `EffectsMenuDefaults.xml`
  (built-in effects menu), `plug-ins/` (28 plugin files), `nyquist/` (31 LSP scripts);
  man page `/usr/share/man/man1/` (36 lines: NAME "graphical audio editor", DESCRIPTION,
  LICENSE, BUGS, AUTHORS = Muse Group per the installed metainfo).

---

## 2. Target A — installed voice-cloning app (target-A)

### 2.1 Inventory (as of the REA pass)

- **Renderer (Electron/React, ~18 feature dirs):** `account, batch, browser, calls, clone, design,
  dub, features, gallery, home, integrations, longform, pro, projects, settings, tools,
  transcriptions, workflows` — i.e. a full platform surface, not just a voice clone.
- **Backend (FastAPI, ~393 routes)** — grouped below by capability.
- **Terminal CLI:** `voicestudio` command families (`voices`, `gallery`, `dub`, `batch`, `jobs`,
  `history`, `projects`, `audio`, `pronunciation`, `engines`, `models`, `settings`, `sharing`,
  `integrations`, `calls`, `diagnostics`, `stories`, `audiobook`, plus `status`, `version`,
  `update`, `help`, `completion`, `connection`, `install`).
- **MCP server (Python, 9 tools):** `check_health`, `clone_voice`, `describe_voice`,
  `design_voice`, `generate_speech`, `list_languages`, `list_personalities`, `list_voices`,
  `transcribe`.

### 2.2 Target-A feature enumeration

**2.2.1 Voice profiles & voices** (backend `/profiles*`, `/personas*`, `/gallery*`, `/marketplace*`,
`/community*`, `/archetypes*`, `/pronunciation*`, `/glossary*`, `/watermark*`)
- Voice profile store: list, create, get, update (PUT), delete (human-confirmed), attestation
  (consent record, `POST …/attestation`; revoke), consent records, reference-audio replace/get,
  profile-image set/get, usage stats, lock/unlock (`POST /profiles/{id}/lock|unlock`).
- Personas: inspect, import, export (`/personas/*`).
- Gallery (voice library): list, list by category, upload, search YouTube, download YouTube clips,
  preview (`GET …/preview`), update, save-as-profile, voice→profile, delete, batch-delete,
  community list/preview/use/submit-url, presets list/use.
- Marketplace: browse/publish/import/export/install-from-marketplace, `/browse` directory.
- Archetypes: list, categories, previews, preview state, check preview, use.
- Pronunciation dictionary: list/add/update/delete, export, import, test substitution.
- Glossary (per project): list/add/update/delete, auto-extract.
- Watermark: status, detect, settings.

**2.2.2 TTS / generation** (`/generate*`, `/audio/*`, `/history*`, `/tools/*`, `/engines/*`, `/models*`,
`/convert`, `/v1/audio/capabilities`, `/.well-known/voicestudio-speech`)
- Generate speech (`POST /generate`), generation budget, generated-clip history
  (list/delete/clear/starred), OpenAI-compatible `POST /audio/speech` +
  `GET /audio/{id}.mp3|ogg|opus`, `GET /audio/voices`, voice convert.
- Speech-platform capabilities: `GET /v1/audio/capabilities`, `.well-known/voicestudio-speech`.
- Engine & model surface: `/engines` (all/tts/asr/translation/diarisation/llm, select, install,
  health, selftest, disk-usage, quants list/select/download, sidecar install/status,
  audiocpp runtime install, local-model get/set/discover, GPU-offer), `/models`
  (list, retrieve, access/status, install prepare/commit/cancel, license accept/verify/receipts),
  `/model/loaded`, `/model/status`, `/hf-token*`, `/hf-mirror*`.
- Tools: `/tools/probe`, `/tools/rate-fit`, `/tools/direction`, `/tools/incremental`,
  `/tools/normalize-speech`, `/tools/effects`, `/tools/plugins`, `/tools/video-context/{job_id}`.

**2.2.3 STT / transcription** (`/transcribe`, `/audio/transcriptions`, `/audio/translations`,
`/capture`, `/ws/transcribe`, `/dictation/*`, `/engines/asr`)
- Capture upload, websocket transcription stream, dictation readiness/models/preferences,
  OpenAI-compatible transcription + translation, `/engines/asr` backends, `/system/asr-backends`.

**2.2.4 Dubbing** (`/dub*`, `/tasks/*`, `/jobs*`)
- Dub pipeline: upload, ingest-from-URL, ingest asset, transcribe, transcribe-stream, translate,
  agent-fit, sync-transcript, cleanup-segments, parse-subtitle-text, prosody-mirror, qc-pass,
  generate, generate/preview, export SRT/VTT/TXT/ASS/MP3/MP2, download variants, export stems,
  export segments zip, media/onsets, tracks, thumbs, AB testing, previews; dub history
  (list/clear/delete); background jobs (`/jobs`, `/tasks/stream/{id}`, `/tasks/cancel/{id}`).

**2.2.5 Batch jobs** (`/batch/*`)
- Enqueue a batch job (video dubbing across languages), list, get, cancel, retry, delete,
  download outputs per language.

**2.2.6 Audiobook & longform** (`/audiobook/*`, `/stories/encode`, `/longform/*`, `/jobs/longform*`)
- Audiobook: synthesize, plan (chapters/voices), preview, import, cover, resume (`POST
  /audiobook/resume/{job_id}`), list of resumable audiobook/story jobs; longform jobs list/delete/render.
- Stories: `POST /stories/encode` (vendor: converts an existing WAV to another format —
  e.g. `--field format=mp3` per the CLI registry) and `stories render` (render a story from a
  chapter plan).

**2.2.7 Calls & telephony** (`/calls*`, `/telephony*`, `/ws/media`, `/ws-ticket`)
- Call lifecycle: place a call, list, get, events, say, takeover, hangup, delete; recordings;
  readiness; options; call settings.
- Telephony: Twilio-style config (`PUT /config`), state, start/stop, status, test endpoint,
  outbound SIP media stream (`websocket /ws/media`), ws tickets.

**2.2.8 Export & delivery** (`/export*`, `/video-export*`)
- File export + export history (list/delete/record/reveal), video-export settings,
  `/v1/audio/capabilities`.

**2.2.9 Projects & bundles** (`/projects*`, project bundles)
- Project CRUD (create/rename/update/delete/list), project bundles export/import/download/document,
  `/export`, `/import`.

**2.2.10 Cloud sync & sharing** (`/cloud_sync*`, `/local*`, tailscale endpoints)
- Status/start/stop/run-now/devices/revoke, choices, off/on, pulled-local/put-local, local keys,
  run, use-choices-of, confirm-deletes, tailscale enable/disable/status.

**2.2.11 System, diagnostics, settings, settings-adjacent state**
- `/sysinfo`, `/system/info`, `/system/diagnose`, `/system/logs*` (list/clear/stream/tauri),
  `/system/errors/recent`, `/system/crash/ack` + `/system/last-run-crash(+ack)`,
  `/system/notifications`, `/system/activity`, `/system/tailscale/*`, `/system/quarantine-status`,
  `/system/set-env`, `/system/network/*`, `/system/hf-token/state`, `/features`-style settings
  endpoints: settings/analytics/changelog/compute-device/cuda-device/performance-profile/perf/*/
  storage/db-backup/gpu-report/storage-models-dir/storage-temp-clear, media-tools
  (acquire/status/ytdlp update+restore, custom-path, use-system, ffmpeg checks).

**2.2.12 LLM & translation integrations** (`/llm-*`, `/settings` providers/skills)
- LLM providers (list/active/save/connect/models/test), endpoint, skills (list/set),
  settings-level providers/skills, pronunciation, pronunciation tests.

**2.2.13 Terminal CLI surface (summary)** — groups and families:
`voices` (clone, design, describe, consent show/set/revoke, compare, export, import, inspect,
lock, unlock, update, usage, reference get/set, image set), `gallery` (list, import, download,
search, preview, update, save-voice, delete, presets list/use, community list/preview/use/submit),
`stories` (render, encode), `audiobook` (jobs, resume, plan, preview, render), `dub` (transcript
get/set/clean, translate, fit, generate, preview, cancel, history, history-delete, ingest,
use-downloaded-captions), `batch` (enqueue, show, cancel, retry, delete), `jobs` (list, show,
events, cancel, longform list/delete), `history` (list, download, quality, delete, clear),
`projects` (show, export, import, create, update, glossary add/update/delete/extract), `audio`
(clean, normalize, convert-voice, direction, rate-fit, effects, plugins, watermark status/detect/settings),
`pronunciation` (add, test, export, import), `engines` (health, selftest, disk-usage, quants
list/select/download, install-status), `models` (recommend, access, install-status, loaded,
unload, delete, license show/revoke), `settings` (providers list/configure/select/test, hf-token
status/clear, storage show/clear-temp), `sharing` (status, enable, disable, tailscale status/enable/disable),
`integrations` (api schema, mcp bindings list/delete), `calls` (options, readiness, list, start,
configure, show, events, say, takeover, hangup, delete, recording), `diagnostics` (system,
preflight, logs, errors, crashes, usage, notifications, bundle, flush-memory, media-tools),
`dub ingest`, plus `status`, `version`, `update/check/stop`, `help`, `completion`, `connection
(show/set/test/auth set/clear)`, `install` (sidecar engines).

**2.2.14 MCP surface (target-A)** — Python MCP server tools (9): `check_health`, `clone_voice`,
`describe_voice`, `design_voice`, `generate_speech`, `list_languages`, `list_personalities`,
`list_voices`, `transcribe`.

### 2.3 Target-A gaps vs ALP (see §5–6 for the ALP-side comparison)

The systematic gaps are: the **voice-platform features ALP does not cover as agent surface yet**
(profile mutation via agent; gallery listing; marketplace MCP parity; dub; batch; audiobook
render; calls/telephony; watermark; project bundles; capture-widget; context flow) — most are
MCP-surface or automation gaps, not product gaps (the GUI covers them); plus the **editor-side
vendor features** (§3).

---

## 3. Target B — installed system audio-editing app (target-B)

### 3.1 Inventory (as of the REA pass)

- **Binary:** `/usr/bin/` ELF 64-bit PIE executable, dynamic, x86-64, vendor per installed
  metainfo: Muse Group (per `/usr/share/metainfo/` AppStream entry and the man page AUTHORS).
- **Version:** release 3.7.9 (AppStream `releases/release version="3.7.9" date="2026-08-19"`);
  the man page states the GUI supports "WAV, AIFF, FLAC, MP2, MP3 or Ogg Vorbis" files.
- **Desktop entry** (`/usr/share/applications/…`): `Name`/`GenericName` = "Sound Editor";
  `Categories` = AudioVideo, Audio, AudioVideoEditing; `Keywords` include "sound, music editing,
  voice channel, frequency, modulation, audio trim, clipping, noise reduction, multi track
  audio editor, edit, mixing, WAV, AIFF, FLAC, MP2, MP3"; MIME types include the app's
  project-file MIME types plus a broad set of container/codec audio types; `Exec` opens files.
- **Man page (36 lines):** "graphical audio editor"; "generally does not support command line
  options, outside of opening files"; license GPL; links to support/manual.
- **Package/docs:** not dpkg/apt-registered; `/usr/share/doc/` for the app (LICENSE.txt, README.md).
- **Assets:** `EffectsMenuDefaults.xml` (built-in effects menu, §3.2),
  `plug-ins/` — 28 plugin files: `adjustable-fade, beat, clipfix, crossfadeclips, crossfadetracks,
  delay, equalabel, highpass, label-sounds, legacy-limiter, lowpass, noisegate, notch,
  nyquist-plug-in-installer, pluck, rhythmtrack, rissetdrum, rms, sample-data-export,
  sample-data-import, ShelfFilter, spectral-delete, SpectralEditMulti, SpectralEditParametricEQ,
  SpectralEditShelves, StudioFadeOut, tremolo, vocoder`; `nyquist/` — 31 LSP scripts
  (`equalizer, envelopes, fileio, rawwaves, sal, sal-parse, seq, seqfnint, seqmidi, sndfnint,
  spectral-analysis, spec-plot, profile, test, velocity, xlinit, xm, …`).

### 3.2 Target-B feature enumeration

**3.2.1 Multitrack timeline / tracks & clips** (evidence: man page — "cut, copy, splice or mix
sounds together"; appdata: "multi track audio editor"; static artifacts)
- Multi-track project model, per-track clip placement on the timeline with drag/drop, zoom
  controls, waveform display.
- (Vendor manual) regions/loop range, snap points, channel conversion, project rate conversion,
  minimize-to-tray-style workflows — partial: the vendor manual lists region selection and the
  `Snap` feature; these are "(vendor manual)" items, not install artifacts.

**3.2.2 Labels & markers** (evidence: `plug-ins/label-sounds.ny`; appdata keywords "audio trim";
(vendor manual): the Labels dialog — double-click the timeline to add a label point, label tool)
- Timeline labels (named point + optional text), label-sounds plug-in; markers (§5).

**3.2.3 Built-in effects (Effects menu, `EffectsMenuDefaults.xml`)**
- *Volume and Compression:* Amplify, Compressor, Limiter, Normalize, Loudness Normalization,
  Auto Duck.
- *Fading:* Fade In, Fade Out, Studio Fade Out, Adjustable Fade, Crossfade Clips, Crossfade Tracks.
- *Pitch and Tempo:* Change Pitch, Change Speed and Pitch, Change Tempo, Paulstretch, Sliding Stretch.
- *EQ and Filters:* Bass and Treble, Graphic EQ, Filter Curve EQ, High-Pass Filter, Low-Pass
  Filter, Shelf Filter, Notch Filter.
- *Noise Removal and Repair:* Click Removal, Noise Reduction, Noise Gate, Repair, Clip Fix.
- *Delay and Reverb:* Echo, Reverb, Delay.
- *Distortion and Modulation:* Tremolo, Distortion, Wahwah, Phaser, Vocoder.
- *Special:* Repeat, Reverse, Invert, Truncate Silence, Vocal Reduction and Isolation, Vocal Remover.
- *Spectral Tools:* Spectral Delete, Spectral Edit Multi Tool, Spectral Edit Parametric EQ,
  Spectral Edit Shelves.
- *Legacy:* Legacy Compressor, Legacy Limiter, Classic Filters.

**3.2.4 Effect/plug-in ecosystem**
- 28 files under `/usr/share/audacity/plug-ins/` (list in §3.1); 31 Nyquist LSP scripts under
  `/usr/share/audacity/nyquist/` (spectral-analysis, spec-plot, equalizer, ShelfFilter, noisegate,
  noise reduction primitives, etc.).
- (Vendor manual): VST(X) loading, LV2, Vamp, LADSPA — historically, per upstream docs; not
  verified against installed artifacts beyond the `nyquist/` bundle.

**3.2.5 Macros / automation** (evidence: stripped binary strings — `MacrosWindow`, `MacroCommandDialog`,
`MacroCommands`, `ApplyMacroDialog`, `MacroDir` — plus (vendor manual): Tools → Macros and
Tools → Apply Macro, user-creatable macros with saved command lists; edit params per command)
- Saved named macros replaying command lists; the app ships default macros (per
  `MacroCommands::GetNamesOfDefaultMacros`); (vendor manual: "Macro Toolbar" commands are
  available from within macro-generated scripts).

**3.2.6 Spectral view & spectral editing** (evidence: `SpectralEditMulti`, `SpectralEditParametricEQ`,
`SpectralEditShelves`, `SpectralDelete` plug-ins; `spectral-analysis.lsp`, `spec-plot.lsp`;
(vendor manual): Spectrogram view of tracks, Spectral Selection, Spectral Edit Mode)
- Spectrogram display per track; spectral selection; spectral editing: delete/attenuate spectral
  bands in a region (the four Spectral tools + Nyquist spectral-edit scripts).

**3.2.7 Silence / noise / echo & dynamics tools** (evidence: Effects menu §3.2.3;
`label-sounds`, `noisegate`, `noise reduction` scripts)
- Silence: Truncate Silence (menu), Silence Finder (vendor manual), auto-audible detection.
- Noise: Noise Reduction, Noise Gate, Noise tools; Repair/Clip Fix; Click Removal.
- Echo/reverb/delay family (see §3.2.3); Normalize/Loudness Normalization/Limiter/Compressor.

**3.2.8 Exporting & delivery** (evidence: man page MIME support; appdata keywords; desktop entry
MimeTypes; (vendor manual): Export dialog with format chain, batch export "script → one audio
file or one file per line")
- Export formats: WAV, AIFF, FLAC, MP2, MP3, Ogg Vorbis (man page + appdata + MIME types);
  stereo/mono, bit depth options; (vendor manual): Opus added in the 3.4+ line, AAC supported
  natively in later versions; export chains (formats processed in sequence).

**3.2.9 Recording** (evidence: appdata — "Record live audio"; desktop Exec opens files)
- Live audio recording with input monitoring; (vendor manual: punch-in/punch-out, loopback
  recording, channel mapping for multi-device).

**3.2.10 Analysis/inspection (vendor manual + app evidence)**
- Frequency analysis (spectrum), envelope view (per-track envelopes; Nyquist `envelopes.lsp`),
  statistics/clip timing, MP3 encoder options in the export dialog.

**3.2.11 Management surfaces** — not applicable to a pure GUI editor.

### 3.3 Target-B gaps vs ALP

Covered by ALP today (target-B → ALP): multitrack timeline + markers (✓), effects catalog
(EQ/compressor/limiter/gate/de-hum/de-click, loudness) + CLAP/LV2 plugin hosting (✓), noise
reduction basics (partially ✓ — de-hum/de-click only), fade-on-clip and crossfades (✓ at clip
level; crossfade-across-clips is a gap), spectral analysis (✓ as analysis/render), export presets
(podcast/audiobook/voice_over; WAV/FLAC; MP3/AAC/Opus pending licenses — gap), recording (✓
in-app capture + ws transcription), macros (✓ `edit_batch`, durable named macros missing — gap).

New gaps (ALP has no/poor equivalent):
- Time-stretch (`Paulstretch`, `Sliding Stretch`, `Change Speed and Pitch`) — ALP has none
  (license review: Signalsmith Stretch MIT vs Rubber Band GPL — post-backlog).
- Delay/reverb/echo family + `Repeat`, `Reverse`, `Invert`, `Vocal Reduction and Isolation`,
  `Vocal Remover` — ALP dsp lacks echo/reverb/delay; `Reverse`/`Invert` exist? (ALP has no
  reverse/invert tools).
- Legacy compressors/classic filters — ALP has the newer curve-style family only.
- Beat-finder/rhythm-track/rms tools (`beat.ny`, `rhythmtrack.ny`, `rissetdrum.ny`).
- Opus/AAC/MP2/MP3 export (MP3 license-deferred — documented non-goal for v1).
- Macros as first-class durable scripts (named, editable, listed).
- Spectral-edit tools as ALP MCP/alp-dsp primitives (`SpectralEditMulti/ParametricEQ/Shelves`
  equivalents on selections).
- Spectrogram view + spectral selection UI (ALP has ASCII spectrogram render — §2.2 of tool
  defs).

---

## 4. Comparison matrix (feature areas; "A" = target-A, "B" = target-B, "ALP α5" = ALP at v0.0.1-alpha.5)

| Feature area | A | B | ALP α5 | Note / gap |
|---|---|---|---|---|
| Multitrack timeline + markers | ✓ | ✓ (§3.2.1, §3.2.2) | ✓ | ALP markers = timeline markers |
| Labels on timeline | ✓ (voice gallery labels,clipse) | ✓ (§3.2.2) | ~ (ALP: clip labels on tracks; no timeline labels) | — |
| Voice profile management (CRUD/lock/usage) | ✓ | n/a | ~ (ProfileStore; no MCP mutation tools) | Gap: voice_profile_update/lock/unlock via MCP |
| Voice gallery (list/upload/preview/delete) | ✓ | ✓ (`label-sounds.ny` etc.) | ~ (Profiles gallery GUI) | ALP: gallery = profile store; upload of user audio = media_import |
| Marketplace (publish/import/browse) | ✓ | n/a | ~ | Partly in GUI; MCP parity missing |
| Community / shared voices | ✓ | n/a | ~ | — |
| Text → speech (generate/render/export) | ✓ | — | ✓ (tts_generate, scripts, export) | — |
| Speech → text (STT, transcripts, capture) | ✓ (capture, dictation) | — | ✓ (transcribe, daemon-first, builtin) | — |
| Voice dubbing | ✓ | — | ✗ | Major gap (video track model missing in ALP) |
| Batch jobs (enqueue/cancel/retry/download) | ✓ | — | ~ (JobManager: nothing submits yet) | Gap: tts_batch |
| Audiobook render (chapters/voices → master) | ✓ | — | ~ (script + export presets; audiobook recipe "later") | Gap: audiobook_encode |
| Longform/story render | ✓ | — | ~ (scripts on timeline; export) | Gap: MCP encode tool (see ALP_BUILD_PLAN §10.4/§704) |
| Calls & telephony (place/say/takeover/recording) | ✓ | — | ✗ | Major gap |
| Export formats (formats & chains) | ✓ | ✓ (§3.2.8) | ~ (podcast/audiobook/voice_over + WAV/FLAC/OGG) | Gap: MP3/AAC/Opus (licenses), chains |
| Effect catalog (EQ, compress, gates, voice tools) | ✓ | ✓ (§3.2.3) | ~ (5-band EQ, compressor, gate, limiter, de-hum/de-click, voice cleanup) | Broad gap on the B list (§3.3) |
| Spectral editing/selection | — (build-in but minimal) | ✓ (§3.2.6) | ~ (ASCII render only) | Gap: ALP-side spectral-edit tools |
| Time-stretch / pitch change (Sliding Stretch, Paulstretch) | — | ✓ | ✗ | Gap (license review first) |
| Macros / automation | ~ | ✓ (§3.2.5) | ~ (edit_batch) | Gap: durable named macros |
| Project bundles | ✓ | n/a | ✗ | Gap: export/import bundles |
| Watermark detect/settings | ✓ | n/a | ✗ | Gap: watermark tools |
| Glossary / pronunciation | ✓ | — | ~ (PronunciationDictionary) | Glossary gap |
| Model & engine management | ✓ | — | ~ (daemon, manifests, CLI) | Mostly present |
| System/diagnostics/settings/sharing | ✓ | — | ✓ (large) | — |
| Cloud sync / devices / tailsale | ✓ | — | ~ | Present; MCP surface gap |
| Engine install, disk usage, CLI for everything | ✓ | — | ~ (`alp` CLI, CLI parity) | — |

**Priority map (alpha.6), see `ALP_ALPHA6_PLAN.md`:**
1. **Phase 1 (alpha.6 core):** voice-profile parity (`voice_profile_update`,
   `voice_profile_lock`, `voice_profile_unlock`) — target-A `voices update|lock|unlock`;
   `tts_batch` — target-A batch; `audiobook_encode` — target-A audiobook render.
2. **Phase 2 (alpha.6+):** B-side: spectral edit tools + time-stretch/pitch (license review);
   delay/reverb/echo + vocal removal + repeat/reverse/invert + legacy compressors; macros;
   glossary; marketplace MCP; project bundles; watermark tools.
3. **Phase 3 (infra):** MCP eval harness (`docs/mcp/EVAL.md`), pagination/limits, attach-mode
   latency, authenticated remote, multi-client sessions, Review mode, more workflow recipes.
4. **Non-goals (documented):** clone creation via MCP (human-only consent invariant stays),
   video dubbing, cloud accounts, MIDI.

---

## 5. Noted ambiguities and needs-verification items (honest gaps in the analysis)

- **target-A "stories encode"** (CLI) = WAV→format conversion; target-A "stories render" = render
  from a chapter plan. Both covered in ALP via `export_audio` presets and script-based rendering.
- **target-B macros**: binary strings show the macros machinery (`MacrosWindow`,
  `ApplyMacroDialog`, `MacroCommands`, `MacroCommandDialog`, `MacroDir`); specific behaviors
  (toolbar, default macros, edit params) are per the vendor manual (labelled "(vendor manual)").
- **target-B labels**: `label-sounds.ny` + appdata keywords + `Labels dialog` (vendor manual).
- **target-B time-stretch**: `Paulstretch`/`Sliding Stretch`/`Change Speed and Pitch` are named in
  the built-in effects menu; the ALP side requires a license review (Rubber Band GPL vs Signalsmith
  MIT — ALP_BUILD_PLAN §10-ish "Post-1.0 backlog").
- **target-B "labels" vs ALP markers**: ALP markers are timeline markers with names — equivalent
  to target-B labels; the reverse gap (ALP timeline labels) is listed in the matrix.
- Version of target-B: 3.7.9 per the AppStream release entry; the GUI-about-dialog is the
  version source per the man page (no CLI `--version`; the man page notes the app "generally
  does not support command line options").
- target-A is a whole platform; its "gaps vs ALP" rows mean ALP lacks the *agent surface and
  platform features* listed (ALP is deliberately a smaller, voice-focused editor today).
