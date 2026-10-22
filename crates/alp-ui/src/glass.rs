//! Compositor glass negotiation and the tint fallback.
//!
//! iced 0.14 exposes `window::run(id, |&dyn Window| …)`, which gives us the raw
//! window/display handles safely. This module uses them to identify the session
//! and to keep a precise report for the shell. KWin on X11 gets a real
//! `_KDE_NET_WM_BLUR_BEHIND_REGION` request through `x11rb`; everything else
//! paints the opaque glass tint. Wayland background-effect attachment still
//! needs a scoped unsafe bridge to the foreign display (see ADR 0003 and ADR
//! 0012), so it is reported as unavailable rather than silently claimed.

use std::fmt;

use iced::window::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

/// How the window background is achieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlassMode {
    /// The compositor blurs the desktop behind the window.
    CompositorBlur,
    /// The app paints a translucent tint; legible without compositor support.
    #[default]
    TintFallback,
}

impl GlassMode {
    /// Stable persisted value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CompositorBlur => "compositor_blur",
            Self::TintFallback => "tint_fallback",
        }
    }

    /// Parse a persisted value, falling back to the safe mode.
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "compositor_blur" => Self::CompositorBlur,
            _ => Self::TintFallback,
        }
    }
}

/// Result of inspecting the window and desktop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlassReport {
    /// Session type (`wayland`, `x11` or `unknown`).
    pub session: String,
    /// Desktop name from `XDG_CURRENT_DESKTOP`.
    pub desktop: String,
    /// Handle kind reported by iced.
    pub handle: String,
    /// Active rendering mode.
    pub mode: GlassMode,
    /// Whether a compositor blur protocol is reachable in this build.
    pub blur_available: bool,
    /// Human-readable explanation for the settings panel.
    pub detail: String,
}

impl GlassReport {
    /// A safe fallback used before the window exists.
    #[must_use]
    pub fn pending() -> Self {
        Self {
            session: std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into()),
            desktop: std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".into()),
            handle: "pending".into(),
            mode: GlassMode::TintFallback,
            blur_available: false,
            detail: "Waiting for the window handle; using the tint fallback.".into(),
        }
    }

    /// One-line status for the chrome.
    #[must_use]
    pub fn badge(&self) -> String {
        match self.mode {
            GlassMode::CompositorBlur => "Glass: compositor blur".into(),
            GlassMode::TintFallback => "Glass: tint fallback".into(),
        }
    }
}

impl fmt::Display for GlassReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} · {} · {})",
            self.badge(),
            self.session,
            self.desktop,
            self.handle
        )
    }
}

/// Inspect the running window.
#[must_use]
pub fn probe(window: &dyn iced::window::Window) -> GlassReport {
    let handle = window
        .window_handle()
        .ok()
        .map(|handle| match handle.as_raw() {
            RawWindowHandle::Wayland(_) => "wayland",
            RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_) => "x11",
            _ => "unsupported",
        })
        .unwrap_or("unknown");
    let display = match window.display_handle().ok().map(|handle| handle.as_raw()) {
        Some(RawDisplayHandle::Wayland(_)) => "wayland",
        Some(RawDisplayHandle::Xlib(_)) | Some(RawDisplayHandle::Xcb(_)) => "x11",
        Some(_) => "unsupported",
        None => "unknown",
    };
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| display.into());
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".into());
    let force_tint = std::env::var_os("ALP_FORCE_TINT").is_some();
    let mut report = resolve(&session, &desktop, handle, force_tint);
    report.handle = format!("{handle}/{display}");

    if !force_tint && handle == "x11" && should_attempt_x11_blur(&desktop) {
        match enable_x11_blur(window) {
            Ok(()) => {
                report.mode = GlassMode::CompositorBlur;
                report.blur_available = true;
                report.detail =
                    "KWin X11 blur is active via _KDE_NET_WM_BLUR_BEHIND_REGION.".into();
            }
            Err(error) => {
                report.detail = format!("X11 blur unavailable ({error}); tint fallback is active.");
            }
        }
    }
    report
}

/// KWin is the one X11 compositor with a documented per-window blur property.
#[must_use]
pub fn should_attempt_x11_blur(desktop: &str) -> bool {
    desktop.to_ascii_lowercase().contains("kde") || desktop.to_ascii_lowercase().contains("plasma")
}

fn enable_x11_blur(window: &dyn iced::window::Window) -> Result<(), String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, PropMode};
    use x11rb::wrapper::ConnectionExt as _;

    let raw = window
        .window_handle()
        .map_err(|_| "no window handle".to_string())?
        .as_raw();
    let window_id = match raw {
        RawWindowHandle::Xlib(handle) => u32::try_from(handle.window)
            .map_err(|_| "Xlib window id does not fit in 32 bits".to_string())?,
        RawWindowHandle::Xcb(handle) => handle.window.get(),
        _ => return Err("window is not X11".into()),
    };

    let (connection, _screen) =
        x11rb::connect(None).map_err(|error| format!("cannot open X11 connection: {error}"))?;
    let geometry = connection
        .get_geometry(window_id)
        .map_err(|error| format!("cannot query X11 geometry: {error}"))?
        .reply()
        .map_err(|error| format!("cannot read X11 geometry: {error}"))?;
    let atom = connection
        .intern_atom(false, b"_KDE_NET_WM_BLUR_BEHIND_REGION")
        .map_err(|error| format!("cannot intern blur atom: {error}"))?
        .reply()
        .map_err(|error| format!("cannot read blur atom: {error}"))?
        .atom;
    connection
        .change_property32(
            PropMode::REPLACE,
            window_id,
            atom,
            AtomEnum::CARDINAL,
            &[0, 0, u32::from(geometry.width), u32::from(geometry.height)],
        )
        .map_err(|error| format!("cannot set blur property: {error}"))?
        .check()
        .map_err(|error| format!("blur property rejected: {error}"))?;
    connection
        .flush()
        .map_err(|error| format!("cannot flush X11 connection: {error}"))?;
    Ok(())
}

/// Pure mode resolution, separated for tests.
#[must_use]
pub fn resolve(session: &str, desktop: &str, handle: &str, force_tint: bool) -> GlassReport {
    let wayland = session.eq_ignore_ascii_case("wayland") || handle == "wayland";
    let detail = if force_tint {
        "ALP_FORCE_TINT is set; using the opaque glass tint.".to_string()
    } else if wayland {
        "Wayland background-effect attachment awaits a scoped unsafe bridge; tint keeps text legible."
            .to_string()
    } else if handle == "x11" {
        "Native X11 window detected; KWin blur is not applicable, so the tint fallback is active."
            .to_string()
    } else {
        "No compositor handle available; tint fallback is active.".to_string()
    };
    GlassReport {
        session: session.to_string(),
        desktop: desktop.to_string(),
        handle: handle.to_string(),
        mode: GlassMode::TintFallback,
        blur_available: false,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forced_tint_wins() {
        let report = resolve("wayland", "KDE", "wayland", true);
        assert_eq!(report.mode, GlassMode::TintFallback);
        assert!(report.detail.contains("ALP_FORCE_TINT"));
    }

    #[test]
    fn wayland_is_reported_as_fallback_not_fake_blur() {
        let report = resolve("wayland", "GNOME", "wayland", false);
        assert_eq!(report.mode, GlassMode::TintFallback);
        assert!(!report.blur_available);
        assert!(report.detail.contains("Wayland"));
    }

    #[test]
    fn only_kde_gets_the_kwin_property() {
        assert!(should_attempt_x11_blur("KDE"));
        assert!(should_attempt_x11_blur("KDE:Plasma"));
        assert!(!should_attempt_x11_blur("GNOME"));
        assert!(!should_attempt_x11_blur("niri"));
    }

    #[test]
    fn mode_round_trips() {
        for mode in [GlassMode::CompositorBlur, GlassMode::TintFallback] {
            assert_eq!(GlassMode::parse(mode.as_str()), mode);
        }
        assert_eq!(GlassMode::parse("nonsense"), GlassMode::TintFallback);
    }
}
