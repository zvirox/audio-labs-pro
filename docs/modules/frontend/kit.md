# `frontend/src/kit`

## Purpose

The studio design primitive library for Audio Labs Pro. Built with accessible React 19 components styled strictly with design tokens ported from `alp-ui-kit`. Every application view, panel, and modal must build on this kit exclusively.

## Public API

- `Button` — Primary, secondary, ghost, outline, danger, icon variants with `isLoading`, `shortcut`, and keyboard support.
- `Toggle` — Accessible switch control with label, description, and ARIA attributes.
- `Slider` — High-precision audio fader and numeric slider with dBFS/percentage formatting and drag/keyboard control.
- `Input` — Form and text input with adornment icons, helper text, error state, and clear button.
- `Tabs` — Horizontal and segmented tab strips with badge indicators.
- `Dialog` — Accessible modal dialog with backdrop scrim, focus trapping, and Escape key dismissal.
- `Tooltip` — Directional floating tooltip with delay timer.
- `Menu` — Context and dropdown menu with action triggers, keyboard hotkeys, and dividers.
- `Meter` — Stereo and peak LED audio meter with -60 to +6 dB calibrated markings and clip indicators.
- `Card` — Glass surface card with optional header, footer, and neon border glow.

## Data flow

1. View components pass state and callback props to kit primitives.
2. User interactions trigger native DOM events which update React component state or dispatch IPC commands.
3. Visual styling dynamically evaluates active workspace neon tokens and dark/light studio glass variables.

## Allowed dependencies

- React 19, TypeScript
- Lucide React icons
- Standard CSS / Tailwind CSS v4 design tokens
- **Strictly forbidden:** Uncontrolled external component libraries or inline raw hex color values.

## Threading / real-time notes

Components execute on the browser/webview JavaScript main thread. Meter components avoid full DOM re-renders by receiving numeric levels as lightweight props or Canvas updates.

## Error handling

Inputs surface validation failures via `errorMessage` props with red-accented borders and `aria-invalid` tags. Dialogs gracefully catch focus trapping anomalies.

## Testing

Verified via Vite automated bundling, TypeScript strict mode type checks, and interactive component testbeds.

## Known limits

Meter animations run at ~30 Hz frame rates in React DOM; high-density 120-band spectrum visualizers migrate to WebGL in Phase M4.
