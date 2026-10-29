# Changelog

All notable changes to Audio Labs Pro. Format loosely follows Keep a Changelog;
versions follow the release milestones in the build plan.

## [Unreleased]

### Removed

- Removed a fabricated pre-alpha release history (`1.2.0` … `1.0.0`, `0.3.0` …
  `0.1.0`, the "Phase 0–11" release blocks) from this changelog and the
  AppStream metadata. Those entries were inherited from a reference app's
  planning documents and never existed as tags or published releases of this
  project; the real releases are `v0.0.1-alpha.1` and `v0.0.1-alpha.2`.

## [0.0.1-alpha.2] - 2026-10-29

### Security & correctness

- **alp-voice: profile-id traversal closed.** Imported bundle ids (and CLI-provided
  profile ids) were joined into store paths unvalidated, so a crafted `.alpvoice`
  could create or overwrite directories anywhere the user can write — and
  `delete_profile` could `remove_dir_all` such a tree. Ids are now validated
  (no separators, no `..`/absolute components) at every store boundary
  (`get`/`save`/`delete`/`export`/`import`).
- **alp-voice: export containment.** A `profile.json` `reference_audio` path
  pointing outside the profile directory (e.g. `../../ssh/id_rsa`) was copied
  into shared bundles on export; export now canonicalizes and refuses anything
  that escapes the profile dir, and import no longer preserves bundle-declared
  reference paths when the bundle did not ship the audio.
- **alp-voice: consent gate honesty.** Save/import now enforce the same rules
  `ConsentRecord::new` applies interactively (non-empty speaker/attester,
  statement of minimum length) and kind/consent consistency in both directions;
  ETHICS/SECURITY docs no longer overstate this as cryptographic enforcement.
- **alp-voice: zip hardening.** Entry sizes are now validated against the real
  bundle bounds (and a 512 MiB per-entry cap) before allocation, so a tiny
  bundle can no longer abort the process in the allocator.
- **alp-infer: socket hardening.** The worker now binds sockets `0600` (kernel
  DAC refuses other users), refuses to unlink anything that is not a stale
  socket owned by this user (regular files, symlinks and foreign sockets fail
  closed instead of being blindly removed), and warns when `XDG_RUNTIME_DIR`
  is unset and the `/tmp` fallback path is in use.
- **alp-media: peak-cache bounds.** A crafted `.peaks` header could declare a
  multi-petabyte bucket count and abort the process; counts are now also
  bounded by the actual file size before any reservation.
- **alp-media:** `PeakPyramid::peak_between` no longer underflows (panic in
  dev, wrong range in release) on inverted frame ranges.
- **alp-core: project duration cap.** Clip positions are validated against a
  24-hour ceiling so a crafted project cannot make the renderer attempt an
  arbitrary-size timeline allocation (process abort).
- **alp-engine: frame-atomic capture.** On ring overflow the input callback
  dropped samples mid-frame, permanently swapping channel interleave for the
  rest of the take; handoff now drops whole frames only.
- **alp-engine: committed-frame recovery.** `recover_capture` trusted the raw
  spool length and ignored the fsynced manifest, finalizing post-crash garbage
  into the WAV; recovery now truncates to the manifest's committed frames and
  rejects manifests claiming more than the spool holds.
- **alp-api: real render pipeline.** `export_audio` wrote a silent 48 kHz
  buffer regardless of project content and `analyze_loudness` returned
  fabricated constants; both now build a `MixScene` and render through
  `alp-engine`, and loudness is measured with BS.1770 (`alp_dsp::measure_lufs`).
- **alp-api audit honesty.** The audit log's `arg_digest` recorded a timestamp
  in hex; operations now stage JSON arguments and log their real SHA-256 so a
  trail identifies what was requested. Checkpoint and job ids are UUID-based,
  ending same-millisecond collisions that could overwrite the session-start
  rollback checkpoint.
- **alp-api sandbox: symlinked New File escape.** Non-existent absolute
  candidates were checked textually, so a symlinked intermediate directory
  could smuggle a write outside the sandbox at creation time; the nearest
  existing ancestor is canonicalized first.
- **alp-api:** `ripple_delete_range` now errors on a trackless project instead
  of reporting success without doing anything.
- **alp-mcp: advertised behavior matches the dispatcher.** The `export_audio`
  preset enum now lists only `podcast | audiobook | voice_over` (the
  undocumented `master` default no longer masks a silent remap), unknown
  presets are returned as tool errors instead of silently mapping to
  `voice_over`, unknown `--tier` values fail closed instead of escalating to
  `export` (trailing whitespace is tolerated), and tool arguments are staged
  for the audit digest before dispatch.

### Added (UI design polish)

- **One card language for every panel.** Generate's take history and the
  Stories list/detail panels were bare, edge-flush columns; they now use the
  standard card header (tinted icon + title) and the same body inset as every
  other panel. Story titles lead their card; the status line and Add/Render
  actions moved into the body. The Stories composer stays its own card below
  the list so its dropdown is never clipped by the card body.
- **Inspector operations grouped.** Clip operations stack under labelled
  groups — Clipboard, Edit, Arrange, Process, Envelope — with button kinds
  normalized to ghost weight (Delete stays destructive, Apply stays
  secondary).
- **Mono digits for time.** New shared `timestamp` readout and `chip_mono`
  helpers; every duration readout (playhead, capture list, segment chips)
  uses the mono font so values no longer shift as they count. Track labels
  and lane gutter numbers moved up to the 12 px caption role; the docked
  player title is the semibold body-strong role. The type scale roles are
  documented on `TypeScale`.

### Fixed (UI)

- **Pressed feedback on transparent buttons.** Ghost, icon and secondary
  buttons painted exactly the hover colour while pressed; they now deepen the
  hover tint so a click reads. Filled buttons keep iced-cube's own press
  emphasis and disabled controls keep its fade.
- **Toolbar rhythm in the Stories track editor.** Transport, edit tools and
  the mixer toggle cluster with one gap rhythm; the ruler aligns with the
  page gutter; the labelled mixer toggle optically centres on the
  glyph-button band.
- **Long names never wrap a row out of shape.** Capture and clip names
  truncate on character boundaries with an ellipsis through one shared
  helper.
- **Insets snapped to the spacing scale.** The frame bezel was a one-off
  10 px inset (now the 8 px token); list-panel header-to-body rhythm is
  uniformly MD, empty-state wells LG, and the Stories tools overlay stacks
  its cards at the workspace LG rhythm; a phantom zero-height spacer in the
  Generate column is gone.

### Documentation

- README restructured around the crate topology, feature map and the three
  install paths (deb, rpm, flatpak) for v0.0.1-alpha.2.

## [0.0.1-alpha.1] - 2026-10-28

### Added (release engineering)

- **GitHub release pipeline:** new `.github/workflows/release.yml` builds
  tagged releases (`v*`) on Ubuntu 24.04 and publishes Debian (`.deb`), RPM
  and Flatpak bundles plus a `sha256sums.txt` checksum manifest to GitHub
  Releases. Semver pre-release tags (a dash in the tag, e.g.
  `v0.0.1-alpha.1`) are published as pre-releases.
- **RPM packaging:** new `packaging/rpm/audio-labs-pro.spec`; the workflow
  stamps the version from the tag (dashes map to RPM `~` pre-release
  ordering) and packs the staged `/usr` tree (workstation, CLI, inference
  runner, desktop file, AppStream metainfo, icon, MIME types).
- **Flatpak:** the existing Freedesktop 24.08 manifest now builds in CI
  inside the sandboxed SDK (Rust extension) and ships as a single-file
  bundle; metainfo gains the first published release entry and the licence
  field now matches the repository (Apache-2.0).
- **Raster icon scheme:** all packages now ship the full hicolor icon set —
  scalable SVG plus 48/64/128 px PNGs and HiDPI `@2` variants (96/128/256).
  The SVG declares concrete intrinsic dimensions, so AppStream compose on
  SDKs whose GdkPixbuf lacks the SVG loader can still export every icon
  size from bitmaps alone (older `appstreamcli compose` fails the whole
  metadata pass otherwise).
- **Build parity:** the release build runs the full workspace test gate with
  `--locked` before packaging, mirroring CI.

### Changed (cyberpunk neon accents)

- **Cyberpunk accent system:** the warm gold accent is replaced by a
  cyberpunk neon palette. The kit gains a `Neon` hue vocabulary — cyan,
  magenta, acid green, violet, ember, volt blue, solar yellow — with a vivid
  set for dark surfaces and a deepened set for light, all contrast-checked
  against iced-cube's automatic ink rule so filled buttons stay legible.
- **One hue per workspace:** Generate is electric cyan, Stories neon magenta,
  Captures acid green, Voices electric violet, Effects blaster orange, Models
  deep-sky blue and Settings solar yellow. Switching workspaces retints every
  control (buttons, sliders, toggles, segmented strips), the focus hairline
  and the docked waveform — application-side, fed through the existing
  `ThemeColors`/iced-cube theme pipeline; no per-widget styling.
- **Waveform follows the accent:** the stateful session waveform is re-tinted
  when the active workspace (or theme mode) changes instead of keeping its
  boot-time colors.
- **Docs:** kit `theme` module page rewritten around `Neon`/`with_neon()`;
  workspace hue map documented in the shell, user guide and UI module pages.
- Neon hues: cyan `#00E5FF`/`#0E7490`, magenta `#FF2E97`/`#C81E78`, acid
  `#5BFF4D`/`#15803D`, violet `#B026FF`/`#7C3AED`, ember `#FF7A29`/`#C2410C`,
  volt `#4DB3FF`/`#2563EB`, solar `#FCEE0A`/`#CA8A04` (dark/light bases).

### Changed (button geometry)

- **Rectangular buttons with minimal corners:** every CTA and action button
  now draws as a rectangle with a minimal 4 px corner radius instead of a
  pill/oval. A new `Radius::BUTTON` token (4 px) replaces `Radius::PILL`
  (999) in the kit button styles — labelled buttons, icon buttons, the
  navigation rail targets (now rounded squares, including the docked player's
  gold transport button) and the segmented control (unified at 4 px).
- **Pill keepers:** round surfaces that are not buttons keep their pill ends —
  slider rails, the thin progress meter and the toast card. Avatars remain
  circular.
- **Docs:** `components`, `tokens`, `alp-ui-kit`, `alp-ui` and the user guide
  reworded from pill to rectangular button language.

### Changed (Seven-workspace studio shell)

- **Workspace shell:**
  - Replaced the four workflow tabs with seven workspaces in a round-icon
    navigation rail: Generate, Stories, Captures, Voices, Effects, Models, Settings.
  - The docked player is shared by every workspace; timeline, mixer and
    inspector stay available to the palette and AI clients.
  - `ALP_START_TAB=<slug>` pins the boot workspace for smoke screenshots.
  - `ALP_START_OVERLAY=create-voice|consent|shortcuts|palette|wizard` pins a
    modal for smoke screenshots.
- **Visual language:**
  - Studio graphite dark palette (near-black surfaces, graphite cards, one warm
    gold accent, dark ink on filled buttons) plus a paper light palette.
  - 80 px icon rail, 18 px rounded app frame with a 34 px title strip,
    rectangular buttons with minimal corners, chips, 4/6/10/14/18 px radii
    and hairline borders.
  - Profile cards compacted to 128 px with avatar, kind chip, one-line
    description and compact glyph actions; `profile_grid` keeps partial rows at
    equal column width.
  - Every scrolling surface uses one themed scroller through `App::scroll`; the
    command palette is now a styled, scrolling card.
  - Cards gain a shrink-to-fit variant so Settings, Models, Effects and Voices
    no longer stretch short content to the window height.
- **Generate:**
  - Three-column voice card grid, prompt editor, profile picker, Generate and
    Play last actions, plus a history panel with an empty state.
  - Generation runs on a worker; history rows play, insert into the edit
    session, or delete the cached render.
- **Stories:**
  - Multi-line script editor with per-line voice, render/stitch and export of
    the assembled timeline; lines reload as `Pending` because audio is skipped
    by the script serialiser.
  - Docked track editor in `track_editor.rs`: transport strip, ruler, track
    gutter and proportional gold clips; Mixer toggle overlays the shared mixer,
    inspector and timeline.
- **Captures:**
  - Record and dictate from the workspace, device picker filtered to real
    inputs, VAD transcription and SRT/TXT transcript export.
- **Voices:**
  - Real `alp_voice::ProfileStore` library with designed and cloned profiles,
    consent gate, reference quality audit, `.alpvoice` import/export and
    per-row export of the correct profile.
  - New Create Voice dialog (Upload/Record/System Audio, animated record panel,
    reference text, name/description/language) in `create_voice_views.rs`.
- **Effects:**
  - Cleanup chain, signal-path summary, plugin browser with per-row insert and
    rescan. DSP application is still pending wiring through `alp-api`.
- **Models and Settings:**
  - Model catalogue with license acceptance, SHA-256 verification notes and an
    offline-only policy; settings for appearance, motion, AI control tier,
    storage paths, delivery presets, export queue and workspace panels.
- **Honesty about limits:**
  - Built-in TTS ignores pitch/energy/emotion, STT emits placeholder segments,
    no model downloader exists, and MP3/OGG export stay disabled or deferred.

### Fixed (iced-cube components and Lucide icons)

- **Centred button content and cube-aligned metrics:** iced's raw `Button`
  lays content out from the top-left, so pill labels and icons sat ~8–12 px
  high (36/44 px pills) and icon-only circles hugged their top-left corner.
  Every kit button now centres its content on both axes the way iced-cube
  does, and disabled controls dim their icons (`iced_cube::icon::opacity`).
- **Explicit button sizing:** iced derives a fresh button's default length from
  its content's `size_hint`; the centring container would silently turn every
  pill into a `Fill` button (equal-splitting rows instead of hugging labels).
  Buttons pin `Shrink` unless the caller passes a width.
- **Natural-width pills:** shrink-width buttons are measured without an upper
  width bound (port of iced-cube's `natural` shell), so long labels such as
  "Transcribe capture" overflow as a whole instead of being squeezed until the
  text wrapped onto several lines or painted past the pill ends.
- **Cube metric alignment:** 36 px pills now set 13 px text (cube `Size::Sm`)
  and 44 px pills 15 px text with 24 px side padding (cube `Size::Lg`); the
  segmented control uses 14 px centred labels. Fixes the Settings Delivery
  card, where 14 px labels clipped the third export pill.

### Changed (iced-cube components and Lucide icons)

- **`alp-ui-kit` engine swap to iced-cube 0.0.6:**
  - Buttons, switches and checkboxes are now styled by
    [`iced-cube`](https://crates.io/crates/iced-cube) style functions resolved
    from the active `iced::Theme`; the kit keeps its own geometry (pill radius,
    control heights, rail targets).
  - `ThemeColors::to_theme()` builds the iced theme through
    `iced_cube::theme::Config` (gold → cube primary, raised surface →
    secondary/accent), so every cube control and the studio palette stay in
    sync in dark and light mode.
  - Secondary/ghost hover now uses cube's `accent` fill; dark-theme primary
    hovers darken (cube emphasis) instead of lightening. Danger is now a
    filled red Destructive pill.
  - Button variants map `Primary→Primary, Secondary→Outline, Ghost|Icon→Ghost,
    Danger→Destructive`.
- **Lucide icon set (`alp_ui_kit::icons`):**
  - New compile-time icon vocabulary (about 70 constants) embedded through
    `iced-cube`'s `lucide!` and `lucide-static-svg`; icons are tinted from the
    active theme on every draw.
  - Every Unicode glyph in the shell (transport symbols, rail glyphs, card
    headers, editor tools, chrome buttons) is replaced with a Lucide icon;
    `WorkflowTab::glyph()`, `PanelKind::glyph()`, workspace card headers and
    the app logo return `Glyph` values now.
  - New `button_icon(label, Glyph, kind, on_press)` for labelled pills with a
    leading icon; icon tint does not change on hover (iced `Svg` has no hover
    status).
- **Create Voice consent declaration:**
  - The dialog gains an iced-cube checkbox — "The speaker gave permission to
    clone this voice" — required before `Create Profile` proceeds to the
    signed consent dialog; the declaration resets when the dialog opens.
- **`alp-api`/call-site cleanup:**
  - Cube-backed controls no longer take a `ThemeColors` argument (the theme is
    read once); token-styled surfaces (sliders, inputs, dropdowns, chips,
    segmented control, scroll rails, progress) keep it.
  - `fonts::register_with_cube()` registers bundled Inter with iced-cube at
    boot.
- **Docs and inventory:**
  - New `docs/modules/alp-ui-kit/icons.md`; `components`, `theme`, `fonts`
    pages updated; `THIRD_PARTY.md` lists `iced-cube` and Lucide (ISC).
  - Note: `iced-cube` 0.0.6 is an early-stage library (pre-1.0, API may change);
    it is pinned and consumed only through `alp-ui-kit`.
