//! Motion wrapper over `lilt`, the toolkit-agnostic candidate for
//! `alp-ui-kit::motion` (D8). Durations follow the plan's tokens.

use std::time::Instant;

use lilt::{Animated, Easing};

pub const FAST_MS: f32 = 120.0;
pub const BASE_MS: f32 = 200.0;
pub const PANEL_MS: f32 = 320.0;

/// A two-state animated value with an explicit on/off target.
#[derive(Debug)]
pub struct Toggle {
    animation: Animated<f32, Instant>,
    on: bool,
    reduce_motion: bool,
    duration_ms: f32,
}

impl Toggle {
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

    #[allow(dead_code)]
    pub fn with_duration(mut self, duration_ms: f32) -> Self {
        self.duration_ms = duration_ms;
        self.animation = self.animation.duration(duration_ms);
        self
    }

    pub fn set_reduce_motion(&mut self, reduce_motion: bool) {
        self.reduce_motion = reduce_motion;
        if reduce_motion {
            self.duration_ms = 0.0;
        }
    }

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

    pub fn tick(&mut self, now: Instant) {
        // `animate` advances the internal clock and returns the current value.
        let _ = self.animation.animate(|value| value, now);
    }

    pub fn value(&self) -> f32 {
        self.animation
            .animate_wrapped(Instant::now())
            .clamp(0.0, 1.0)
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    pub fn is_animating(&self) -> bool {
        self.animation.in_progress(Instant::now())
    }
}
