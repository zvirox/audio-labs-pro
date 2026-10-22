//! `alp-ui-kit` — design tokens, glass components, motion and waveform drawing.
//!
//! The toolkit depends only on external crates: it must never pull in
//! `alp-core`, `alp-media` or the engine, so the UI layer stays replaceable.
//! Media types are converted at the `alp-ui` boundary.

pub mod components;
pub mod fonts;
pub mod motion;
pub mod overlays;
pub mod theme;
pub mod tokens;
pub mod waveform;

pub use components::ButtonKind;
pub use overlays::CommandItem;
pub use theme::ThemeColors;
pub use tokens::{Control, Duration, Motion, Radius, Spacing, ThemeMode, TypeScale};
pub use waveform::{PeakLevel, WaveformError, WaveformPeaks, WaveformWidget};
