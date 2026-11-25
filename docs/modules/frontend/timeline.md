# `frontend/src/timeline`

Interactive HTML5 Canvas 2D multitrack arranger (`MultitrackCanvas.tsx`), hit-testing (`timelineMath.ts`), and drag/trim/slip state machine (`dragLogic.ts`).

## Linear Canvas Styling

- Reads active CSS theme tokens via `src/utils/linearCanvasTheme.ts` on initial draw and whenever `theme-changed` fires, ensuring legibility in both **Dark** (`#050505` / `#111213`) and **Light** (`#F3F4F6` / `#FFFFFF`) modes.
- Track lanes render on alternating `--bg-surface` / `--bg-surface-2` rows with `--border` dividers.
- Clips render as flat `4 px` rounded containers with `1 px` borders cycling series accents (`--accent-blue`, `--accent-sky`, `--accent-teal`, `--accent-purple`, `--accent-orange`) — no drop shadows or gradients.
- Playhead renders as a flat red (`--accent-red`) vertical line with a top triangle handle over a dotted `--bg-surface-2` timecode ruler.
- Tested deterministically in `frontend/src/timeline/timelineMath.test.ts` (coordinate roundtrips, hit testing, snapping, trim bounds, fade math, and drag commands).
