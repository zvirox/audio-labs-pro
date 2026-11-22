//! Motion primitives backed by `lilt`.
//!
//! Durations come from [`crate::tokens::Duration`]. Reduce-motion makes a
//! transition instantaneous rather than merely faster, so it is safe for
//! vestibular-sensitive users.

use std::time::Instant;

use iced::advanced::Renderer as _;
use iced::{Element, Vector};
use lilt::{Animated, Easing};

use crate::tokens::{Duration, Motion};

const BASE_MS: f32 = 200.0;
/// Subtle settle offset for entering content (logical pixels).
const SETTLE_OFFSET: f32 = 8.0;

/// A two-state animated value with an explicit target.
#[derive(Debug)]
pub struct Toggle {
    animation: Animated<f32, Instant>,
    on: bool,
    reduce_motion: bool,
    duration_ms: f32,
}

impl Toggle {
    /// Create a toggle at `initial` (0.0 or 1.0).
    #[must_use]
    pub fn new(initial: f32) -> Self {
        Self {
            animation: Animated::new(initial)
                .duration(BASE_MS)
                .easing(Easing::EaseOut),
            on: initial > 0.5,
            reduce_motion: false,
            duration_ms: BASE_MS,
        }
    }

    /// Use a different duration token.
    #[must_use]
    pub fn with_duration(mut self, duration_ms: u16) -> Self {
        self.duration_ms = f32::from(duration_ms);
        self.animation = self.animation.duration(self.duration_ms);
        self
    }

    /// Apply the Reduce motion setting.
    pub fn set_reduce_motion(&mut self, reduce_motion: bool) {
        self.reduce_motion = reduce_motion;
        self.animation =
            self.animation
                .clone()
                .duration(if reduce_motion { 0.0 } else { self.duration_ms });
    }

    /// Move to a new target.
    pub fn toggle(&mut self, on: bool) {
        if on == self.on {
            return;
        }
        self.on = on;
        let target = if on { 1.0 } else { 0.0 };
        let now = Instant::now();
        if self.reduce_motion {
            self.animation.transition_instantaneous(target, now);
        } else {
            self.animation.transition(target, now);
        }
    }

    /// Advance the clock.
    pub fn tick(&mut self, now: Instant) {
        let _ = self.animation.animate(|value| value, now);
    }

    /// Current value in `0.0..=1.0`.
    #[must_use]
    pub fn value(&self) -> f32 {
        self.animation
            .animate_wrapped(Instant::now())
            .clamp(0.0, 1.0)
    }

    /// Whether the target is on.
    #[must_use]
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// Whether a transition is still running.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.animation.in_progress(Instant::now())
    }
}

/// One frame of an open/close/settle animation.
#[derive(Debug, Clone, Copy)]
pub struct PanelMotion {
    /// Open progress in `0.0..=1.0`.
    pub open: f32,
    /// Settle offset in logical pixels.
    pub offset: f32,
}

impl PanelMotion {
    /// Compose a panel animation from its open toggle.
    #[must_use]
    pub fn from_open(toggle: &Toggle) -> Self {
        let open = toggle.value();
        Self {
            open,
            offset: (1.0 - open) * SETTLE_OFFSET,
        }
    }
}

/// Build a blink/fade toggle for transient overlays.
#[must_use]
pub fn overlay_toggle() -> Toggle {
    Toggle::new(0.0).with_duration(Duration::BASE_MS)
}

/// Build a panel-open toggle.
#[must_use]
pub fn panel_toggle() -> Toggle {
    Toggle::new(1.0).with_duration(Duration::PANEL_MS)
}

/// Apply the motion policy to a duration token.
#[must_use]
pub fn scaled(motion: Motion, duration: u16) -> u16 {
    motion.duration_ms(duration)
}

/// Draw a child with a small translation, without changing its layout bounds.
///
/// This is used for workspace and overlay settle motion. The transform only
/// affects painting, so it cannot reflow surrounding content or cause layout
/// shift; keep offsets below the surrounding padding.
#[must_use]
pub fn offset<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    x: f32,
    y: f32,
) -> Element<'a, Message> {
    Element::new(Offset {
        content: content.into(),
        x,
        y,
    })
}

struct Offset<'a, Message> {
    content: Element<'a, Message>,
    x: f32,
    y: f32,
}

impl<Message> iced::advanced::Widget<Message, iced::Theme, iced::Renderer> for Offset<'_, Message> {
    fn size(&self) -> iced::Size<iced::Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> iced::Size<iced::Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &iced::Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        renderer.with_translation(Vector::new(self.x, self.y), |renderer| {
            self.content
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        });
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        self.content.as_widget().diff(tree);
    }

    fn operate(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &iced::Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &iced::Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduce_motion_is_instantaneous() {
        let mut toggle = Toggle::new(0.0).with_duration(Duration::PANEL_MS);
        toggle.set_reduce_motion(true);
        toggle.toggle(true);
        assert!((toggle.value() - 1.0).abs() < 1e-6);
        assert!(!toggle.is_animating());
    }

    #[test]
    fn panel_motion_starts_offset_and_ends_flush() {
        let mut toggle = panel_toggle();
        toggle.set_reduce_motion(true);
        let closed = PanelMotion::from_open(&toggle);
        assert!((closed.offset - 0.0).abs() < 1e-6);
        toggle.toggle(false);
        let open = PanelMotion::from_open(&toggle);
        assert!((open.offset - SETTLE_OFFSET).abs() < 1e-6);
    }
}
