# Audio Labs Pro — architecture

Source of truth for crate boundaries and dependency directions. If this file and
`cargo metadata` disagree, the code or this document is wrong and must be fixed
in the same commit.

Status: Phase 3 complete on this machine and wired into the shell (crash-safe
capture, single-track edits, import conversion, WAV/FLAC export and autosave
recovery). The 30-minute dropout soak and forced-kill recovery pass; other audio
devices remain untested. Phase 1 and Phase 2 remain the base.

## Processes

| Process | Responsibility |
|---|---|
| `alp-ui` | One transparent OS window, floating panels, input, rendering. Never touches cpal or ONNX. |
| `alp-engine` (in-process audio thread) | cpal callbacks, mixer, transport, recorder. Real-time safe: no allocation, no locks, no logging. |
| `alp-infer` (separate process) | TTS/STT/clone models over ONNX Runtime and GGUF. Isolated so an OOM or GPU crash never kills the editor. Talks over a versioned Unix-socket protocol owned by `alp-voice`. |
| Worker threads | Decode, peak building, effect rendering, export. Cancellable jobs with progress channels; the shell uses this for real background mix export. |

## Crate graph

Solid arrows are compile-time dependencies. The dotted arrow is the runtime
socket connection and must never become a Cargo dependency.

```mermaid
flowchart TD
  subgraph APP["Applications"]
    UI["alp-ui<br/>shell, panels, glass"]
    CLI["alp-cli<br/>headless tools"]
    KIT["alp-ui-kit<br/>tokens, Inter, components, motion, waveform"]
  end
  subgraph DOMAIN["Domain crates"]
    ENG["alp-engine<br/>cpal I/O, mixer, transport"]
    VOICE["alp-voice<br/>traits, model manager, protocol"]
    DSP["alp-dsp<br/>effects"]
    MEDIA["alp-media<br/>decode, encode, peaks"]
    CORE["alp-core<br/>project model, commands, types"]
  end
  INFER["alp-infer (separate process)<br/>ONNX / GGUF backends"]

  UI --> KIT
  UI --> ENG
  UI --> VOICE
  UI --> MEDIA
  UI --> CORE
  CLI --> ENG
  CLI --> VOICE
  CLI --> DSP
  CLI --> MEDIA
  CLI --> CORE
  ENG --> MEDIA
  ENG --> CORE
  VOICE --> MEDIA
  VOICE --> CORE
  DSP --> CORE
  MEDIA --> CORE
  INFER --> VOICE
  VOICE -. "Unix socket (runtime)" .-> INFER
```

## Allowed dependency directions

Enforced by `scripts/check-deps.sh`.

- `alp-core` depends on no other ALP crate.
- `alp-media` and `alp-dsp` depend on `alp-core` only.
- `alp-engine` depends on core and media. Never on dsp, ui or voice.
- `alp-voice` depends on core and media. It reaches `alp-infer` only over the
  socket protocol.
- `alp-infer` depends on `alp-voice` (traits and protocol), core and media.
- `alp-ui-kit` has **no ALP dependencies**. It uses `iced` and `lilt` plus the
  Rust standard library only.
- `alp-ui` depends on ui-kit, core, engine, media and voice. It never touches
  cpal, ONNX or GGUF directly.
- `alp-cli` depends on core, engine, media, dsp and voice. Never on ui.

### Phase 3 additions

- Capture writes a raw f32 spool plus a chunk-synced JSON manifest; recovery
  truncates to the committed frame count and finalizes a WAV atomically. A
  ten-second SPSC ring absorbs chunk-sync stalls; the writer polls at 1 ms.
- `alp-core` owns the selection model, clipboard fragments and undoable range
  edits (`DeleteRange`, `SilenceRange`, `PasteClips`, `DuplicateClip`,
  `PunchIn`, `NormalizeClip`, reverse/invert). Split ids are cached across redo.
- `alp-media` owns WAV/FLAC export, deterministic TPDF dither, RIFF/Vorbis
  metadata, and the null-test comparison helper.
- `alp-ui` keeps a live `EditSession` with a real project and command stack;
  views send messages and never mutate project state. Mix export runs on the
  worker queue from a cloned `MixScene`.
- `ProjectPaths` writes an atomic autosave recovery copy and restores it only
  after schema validation. The desktop session opens a fixed `ui-demo` project
  and debounces autosaves after edits.
- Hardware acceptance is automated by `scripts/phase3-demo.sh` but still needs a
  real input device.

### Phase 2 additions

- `alp-ui-kit` implements tokens, bundled Inter, dark/light glass palettes,
  semantic controls, overlays, motion and the peak-pyramid waveform canvas.
- `alp-ui` implements the shell, eight floating-panel stubs, drag/resize/z-order/
  snap/collapse/dock, versioned workspace persistence, command palette and
  keyboard shortcuts, a cooperative worker queue, and the glass probe.
- `alp-media::PeakPyramid` is converted at the `alp-ui` boundary into
  `alp-ui-kit::waveform::WaveformPeaks`; the kit never links `alp-media`.
- `scripts/check-ui.sh` rejects a raw `iced::widget::button` in `alp-ui` (the
  component kit is the only button constructor).
- `scripts/check-file-length.sh` still enforces 1,500 lines; `app.rs` is close to
  the ceiling and should be split before the next feature.
- Peak cache format remains owned by `alp-media`; the UI reads it through a
  public type, never by parsing the file.

## Threading and real-time rules

| Context | May do | Must not do |
|---|---|---|
| UI thread | Render, input, cheap state updates, drain worker events | Decode, infer, block, wait on workers |
| Audio callback | Read pre-decoded buffers, mix, write the device buffer, drain `rtrb` commands | Allocate, lock, log, syscall, join, sleep |
| Worker threads | Decode, peaks, offline render, export, import conversion | Touch the audio callback directly |
| `alp-infer` | Model load/inference, its own memory | Depend on UI state; hold the audio device |

The UI subscribes to `window::frames()` only while a panel animates, a recording
runs, the demo transport plays, a job runs, or a diagnostic env var is set. When nothing moves
there is no redraw subscription, which is why idle CPU is zero. Worker progress
arrives over `std::sync::mpsc` and is drained with `try_recv()` from a frame.

Phase 1 measurements (release build, this machine):

| Check | Result |
|---|---|
| 1-hour mono 48 kHz WAV peak build | 1.42 s |
| same cache reopened | 0.013 s |
| eight simultaneous 60 s clips rendered | 2,880,000 frames in 1.5 s |
| 10 s device playback | 931 callbacks, 476,672 frames, no stream error |
| 10-minute eight-clip soak | see `docs/spikes/PHASE1-RESULTS.md` |

Phase 2 measurements (release build, this machine):

| Check | Result |
|---|---|
| 1-hour waveform autoscroll | 60–61 frames/s, 16.6 ms average frame time |
| idle CPU over 4 s | 0.00% (0 scheduler ticks) |
| idle RSS | about 88 MB |
| module/unit tests | 30 in `alp-ui`, 18 in `alp-ui-kit`; 104 workspace-wide |

Phase 3 headless measurements (release build, this machine):

| Check | Result |
|---|---|
| import conversion | 44.1 kHz source converted to 48 kHz |
| autosave recovery | clean 3 clips; recovered edited project 4 clips |
| 10 s reference render | 480,000 frames |
| WAV/FLAC null test | max sample difference 1.8e-7 at 24-bit |
| full workspace tests | 144 tests plus doctests |
| 30-minute capture soak | 86,401,536 frames · 0 dropped samples |
| forced-kill capture recovery | 379,904 frames · `recovered=true` |

The mixer is allocation-free by construction; an automated allocator guard is
tracked in ADR 0011 as a follow-up.

## Non-destructive editing

- Source media is immutable; imports are copied into `media/`.
- A project is tracks → clips `{source_id, source_range, timeline_start, gain,
  fade_in, fade_out, envelope, reversed, inverted, effects[]}`.
- Every mutation is a `Command` with `apply`/`revert`; undo/redo is the stack.
- Range edits cache the complete replacement clip list once, so redo restores
  the same ids. `SilenceRange` keeps later clips in place; `DeleteRange` can
  ripple or leave a gap.
- Reverse and invert are clip flags rendered by the mixer; normalization uses
  the source peak measured at import.
- Playback, bounce and export walk the same clip graph and the same `Effect`
  implementations, so a null test can compare preview and render.

## Project format

Directory `*.alp/` containing `project.json` (versioned + migrations), `media/`,
`peaks/` (i16 min/max pyramid, 3 levels, ~3.4 MB per hour), `renders/`, `voices.ref`
and `autosave/project.json` while edits are unsaved. Saves are atomic: sibling
temp file, `sync_all`, rename. A clean save removes the recovery copy; recovery
validates it before replacing `project.json`. A capture in progress uses
`<output>.alp-capture` plus `<output>.alp-capture.json`. A corrupt or stale peak
cache is rebuilt; it is never authoritative.

The UI workspace is a separate `~/.config/alp/workspace.json`, also written
atomically. It stores schema version 1, theme, Reduce motion, last glass mode and
the eight panel geometries. A missing, corrupt or newer file is ignored in favour
of defaults.

## Glass strategy

1. Transparent undecorated window with in-window floating panels (one OS
   window). iced 0.14 exposes `window::run(id, |&dyn Window| …)`, so the app can
   read `RawWindowHandle` and `RawDisplayHandle` safely.
2. On KWin/X11, set `_KDE_NET_WM_BLUR_BEHIND_REGION` through `x11rb` for the
   window rectangle. Report `Glass: compositor blur` only after the property
   write succeeds.
3. Everywhere else paint the translucent tint (panel alpha ≥ 0.70) plus a darker
   root scrim. The fallback is legible without a compositor blur effect.
4. `ALP_FORCE_TINT=1` disables the blur attempt for testing and screenshots.
5. Motion is decorative: Reduce motion disables it; no motion ever encodes audio
   state.

Wayland `ext_background_effect_v1` remains unimplemented because attaching a
foreign display requires `unsafe`, and the workspace forbids unsafe code. This
is recorded in ADR 0012, which supersedes the vendor-patch assumption in ADR
0003. The current matrix is:

| Session | Result |
|---|---|
| niri/Wayland | transparent tint; no blur |
| GNOME/Wayland | transparent tint; no blur |
| KDE/X11 | KWin blur property + tint |
| KDE/Wayland | tint fallback (unless the app is run through an X11 handle) |
| other X11 | tint fallback |

### Glass module boundary

```text
alp-ui::glass
  ├─ probe()       read iced's raw window/display handles once
  ├─ resolve()     identify session/desktop and choose a mode for the report
  └─ x11 blur      set _KDE_NET_WM_BLUR_BEHIND_REGION for KWin
```

It is the only place that touches platform windowing beyond iced, and it must
never block the audio thread or the UI frame.
