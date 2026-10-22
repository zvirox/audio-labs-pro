//! Glass primitives for the Phase 0 spike: backdrop, panel tint, and a
//! compositor-capability report used to decide the real fallback strategy.

use iced::widget::canvas;
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, mouse};

/// What the running session can offer for a frosted-glass window.
#[derive(Debug, Clone)]
pub struct BlurSupport {
    pub session: String,
    pub desktop: String,
    pub compositor: String,
    pub protocol_guess: &'static str,
}

impl BlurSupport {
    pub fn detect() -> Self {
        let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".into());

        let compositor = std::env::var("WAYLAND_DISPLAY")
            .ok()
            .map(|d| format!("wayland ({d})"))
            .or_else(|| std::env::var("DISPLAY").ok().map(|d| format!("x11 ({d})")))
            .unwrap_or_else(|| "headless".into());

        // Populated with real measurements by the wayland-probe spike; this is
        // the runtime fallback the app will use in Phase 2.
        let protocol_guess = if desktop.to_lowercase().contains("kde") {
            "org_kde_kwin_blur candidate"
        } else if session == "wayland" {
            "ext-background-effect-v1 if advertised, else tint fallback"
        } else if session == "x11" {
            "_KDE_NET_WM_BLUR_BEHIND_REGION candidate"
        } else {
            "tint + frost gradient fallback"
        };

        Self {
            session,
            desktop,
            compositor,
            protocol_guess,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "session={} desktop={} display={} → {}",
            self.session, self.desktop, self.compositor, self.protocol_guess
        )
    }
}

/// A fake desktop behind the panel. The window is transparent, so this layer is
/// only visible where the panel is not; it lets us judge contrast without a
/// wallpaper dependency during measurement runs.
pub fn backdrop() -> Element<'static, crate::Message> {
    canvas(Backdrop)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

struct Backdrop;

impl canvas::Program<crate::Message> for Backdrop {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let size = bounds.size();

        frame.fill_rectangle(Point::ORIGIN, size, Color::from_rgb(0.075, 0.085, 0.12));

        // Soft glow blobs make the panel tint legible in screenshots. iced 0.14
        // only exposes linear gradients in canvas, so glow is faked with
        // layered translucent discs.
        for (x, y, r, color) in [
            (0.18, 0.24, 260.0, Color::from_rgba(0.25, 0.55, 0.95, 0.16)),
            (0.78, 0.32, 220.0, Color::from_rgba(0.95, 0.35, 0.55, 0.13)),
            (0.62, 0.82, 300.0, Color::from_rgba(0.20, 0.85, 0.62, 0.11)),
        ] {
            let center = Point::new(size.width * x, size.height * y);
            for ring in 0..6 {
                let radius = r * (1.0 - ring as f32 * 0.14);
                frame.fill(&canvas::Path::circle(center, radius), color);
            }
        }

        vec![frame.into_geometry()]
    }
}

pub fn panel_style() -> iced::widget::container::Style {
    use iced::widget::container;
    use iced::{Background, Border, Shadow, Vector};

    container::Style {
        background: Some(Background::Color(Color::from_rgba(
            0.055, 0.06, 0.075, 0.72,
        ))),
        text_color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.92)),
        border: Border {
            radius: 18.0.into(),
            width: 1.0,
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.14),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
            offset: Vector::new(0.0, 18.0),
            blur_radius: 48.0,
        },
        snap: true,
    }
}

pub fn card_style() -> iced::widget::container::Style {
    use iced::widget::container;
    use iced::{Background, Border, Shadow, Vector};

    container::Style {
        background: Some(Background::Color(Color::from_rgba(
            0.08, 0.085, 0.105, 0.97,
        ))),
        text_color: Some(Color::WHITE),
        border: Border {
            radius: 14.0.into(),
            width: 1.0,
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
            offset: Vector::new(0.0, 24.0),
            blur_radius: 64.0,
        },
        snap: true,
    }
}
