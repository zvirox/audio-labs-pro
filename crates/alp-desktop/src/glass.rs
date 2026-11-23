//! Compositor glass negotiation and tint fallback for Tauri 2.
//!
//! KWin on X11 sets the atom `_KDE_NET_WM_BLUR_BEHIND_REGION` via safe `x11rb`
//! on the window's X11 handle. Everywhere else (Wayland, GNOME, non-KDE X11),
//! the app applies a translucent tint fallback (alpha >= 0.70) keeping text legible.
//! Honoring the invariant from ADR 0012 and ADR 0013:
//! - Hand-written code contains zero `unsafe`.
//! - Wayland background-effect attachment remains out of scope.
//! - Motion never encodes audio state.
//! - `ALP_FORCE_TINT=1` forces tint fallback regardless of desktop compositor.

use std::fmt;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use serde::{Deserialize, Serialize};

/// How the desktop window background is achieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GlassMode {
    /// The compositor blurs the desktop wallpaper behind the window.
    CompositorBlur,
    /// The app renders a translucent tint fallback (alpha >= 0.70).
    #[default]
    TintFallback,
}

impl GlassMode {
    /// Stable persisted string identifier.
    #[must_use]
    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CompositorBlur => "compositor_blur",
            Self::TintFallback => "tint_fallback",
        }
    }

    /// Parse a persisted identifier, falling back to safe tint.
    #[must_use]
    #[allow(dead_code)]
    pub fn parse(value: &str) -> Self {
        match value {
            "compositor_blur" => Self::CompositorBlur,
            _ => Self::TintFallback,
        }
    }
}

/// Inspection report of window handle, desktop environment, and glass mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlassReport {
    /// Session type (`wayland`, `x11` or `unknown`).
    pub session: String,
    /// Desktop name from `XDG_CURRENT_DESKTOP`.
    pub desktop: String,
    /// Handle kind reported by raw-window-handle.
    pub handle: String,
    /// Active rendering mode.
    pub mode: GlassMode,
    /// Whether compositor blur protocol is active.
    pub blur_available: bool,
    /// Human-readable explanation.
    pub detail: String,
}

impl GlassReport {
    /// Initial safe fallback used before the native window is created.
    #[must_use]
    pub fn pending() -> Self {
        Self {
            session: std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into()),
            desktop: std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".into()),
            handle: "pending".into(),
            mode: GlassMode::TintFallback,
            blur_available: false,
            detail: "Waiting for the window handle; using translucent tint fallback.".into(),
        }
    }

    /// Badge description for UI display.
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

/// Inspect the native Tauri WebviewWindow and configure KWin blur if applicable.
pub fn probe<W: HasWindowHandle + HasDisplayHandle>(window: &W) -> GlassReport {
    let handle = window
        .window_handle()
        .ok()
        .map(|h| match h.as_raw() {
            RawWindowHandle::Wayland(_) => "wayland",
            RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_) => "x11",
            _ => "unsupported",
        })
        .unwrap_or("unknown");

    let display = window
        .display_handle()
        .ok()
        .map(|d| match d.as_raw() {
            RawDisplayHandle::Wayland(_) => "wayland",
            RawDisplayHandle::Xlib(_) | RawDisplayHandle::Xcb(_) => "x11",
            _ => "unsupported",
        })
        .unwrap_or("unknown");

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
                report.detail = format!("X11 blur unavailable ({error}); tint fallback active.");
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

/// Safe KWin blur property application via `x11rb`.
pub fn enable_x11_blur<W: HasWindowHandle>(window: &W) -> Result<(), String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, PropMode};
    use x11rb::wrapper::ConnectionExt as _;

    let raw = window
        .window_handle()
        .map_err(|e| format!("no window handle available: {e}"))?
        .as_raw();

    let window_id = match raw {
        RawWindowHandle::Xlib(handle) => u32::try_from(handle.window)
            .map_err(|_| "Xlib window id does not fit in 32 bits".to_string())?,
        RawWindowHandle::Xcb(handle) => handle.window.get(),
        _ => return Err("window handle is not X11".into()),
    };

    let (conn, _screen) =
        x11rb::connect(None).map_err(|e| format!("cannot open X11 connection: {e}"))?;

    let geometry = conn
        .get_geometry(window_id)
        .map_err(|e| format!("cannot query X11 geometry: {e}"))?
        .reply()
        .map_err(|e| format!("cannot read X11 geometry: {e}"))?;

    let atom = conn
        .intern_atom(false, b"_KDE_NET_WM_BLUR_BEHIND_REGION")
        .map_err(|e| format!("cannot intern blur atom: {e}"))?
        .reply()
        .map_err(|e| format!("cannot read blur atom: {e}"))?
        .atom;

    conn.change_property32(
        PropMode::REPLACE,
        window_id,
        atom,
        AtomEnum::CARDINAL,
        &[0, 0, u32::from(geometry.width), u32::from(geometry.height)],
    )
    .map_err(|e| format!("cannot set blur property: {e}"))?
    .check()
    .map_err(|e| format!("blur property rejected: {e}"))?;

    conn.flush()
        .map_err(|e| format!("cannot flush X11 connection: {e}"))?;

    Ok(())
}

/// Pure mode resolution, separated for deterministic unit testing.
#[must_use]
pub fn resolve(session: &str, desktop: &str, handle: &str, force_tint: bool) -> GlassReport {
    let wayland = session.eq_ignore_ascii_case("wayland") || handle == "wayland";
    let detail = if force_tint {
        "ALP_FORCE_TINT is set; using translucent tint fallback.".to_string()
    } else if wayland {
        "Wayland background-effect attachment awaits a scoped bridge; tint keeps text legible."
            .to_string()
    } else if handle == "x11" {
        if should_attempt_x11_blur(desktop) {
            "KWin on X11 detected; ready to request _KDE_NET_WM_BLUR_BEHIND_REGION.".to_string()
        } else {
            "Non-KDE X11 desktop; tint fallback keeps text legible.".to_string()
        }
    } else {
        "Unsupported windowing platform; tint fallback active.".to_string()
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
    fn force_tint_always_selects_tint_fallback() {
        let report = resolve("x11", "KDE", "x11", true);
        assert_eq!(report.mode, GlassMode::TintFallback);
        assert!(!report.blur_available);
        assert!(report.detail.contains("ALP_FORCE_TINT"));
    }

    #[test]
    fn wayland_selects_tint_fallback() {
        let report = resolve("wayland", "KDE", "wayland", false);
        assert_eq!(report.mode, GlassMode::TintFallback);
        assert!(!report.blur_available);
        assert!(report.detail.contains("Wayland"));
    }

    #[test]
    fn non_kde_x11_selects_tint_fallback() {
        let report = resolve("x11", "GNOME", "x11", false);
        assert_eq!(report.mode, GlassMode::TintFallback);
        assert!(!report.blur_available);
    }

    #[test]
    fn glass_mode_string_roundtrip() {
        assert_eq!(
            GlassMode::parse("compositor_blur"),
            GlassMode::CompositorBlur
        );
        assert_eq!(GlassMode::parse("tint_fallback"), GlassMode::TintFallback);
        assert_eq!(GlassMode::parse("unknown_string"), GlassMode::TintFallback);
        assert_eq!(GlassMode::CompositorBlur.as_str(), "compositor_blur");
        assert_eq!(GlassMode::TintFallback.as_str(), "tint_fallback");
    }
}
