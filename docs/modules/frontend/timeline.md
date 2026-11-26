# `frontend/src/timeline`

Interactive HTML5 Canvas 2D multitrack arranger (`MultitrackCanvas.tsx`), hit-testing (`timelineMath.ts`), and drag/trim/slip state machine (`dragLogic.ts`).

## Linear Canvas Styling

- Reads active CSS theme tokens via `src/utils/linearCanvasTheme.ts` on initial draw and whenever `theme-changed` fires, ensuring crisp legibility in both **Dark** (`#1C1C1C` / `#292929`) and **Light** (`#F8F9FA` / `#FFFFFF`) modes.
- Track lanes render on alternating `--bg-surface` / `--bg-surface-2` rows with `--border` dividers and `--bg-sidebar` headers.
- Clips render as flat `4 px` rounded containers with `1 px` borders cycling `[--primary, --sky, --amber, --pink, --purple]` (`--primary` when selected) and `--sky` waveform bars — no drop shadows or gradients.
- Track strip badges use Mute `--amber`, Solo `--sky`, Record-Arm `--red`.
- Playhead renders as a flat red (`--red`) vertical line with a top triangle handle over a dotted `--bg-surface-2` timecode ruler.
- Tested deterministically in `frontend/src/timeline/timelineMath.test.ts`.
