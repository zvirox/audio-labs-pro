//! Non-destructive volume and pan automation envelopes.
//!
//! Envelopes provide multi-point automation curves overlaid on audio clips.
//! Each point specifies a frame offset relative to clip start and a target
//! multiplier.

use serde::{Deserialize, Serialize};

use crate::project::EnvelopePoint;

/// Curve interpolation strategy between envelope control points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationMode {
    /// Direct linear interpolation between adjacent points.
    #[default]
    Linear,
    /// Smooth S-curve transition using cosine smoothing.
    Smooth,
    /// Perceptually linear exponential transition (useful for decibel fade curves).
    Exponential,
}

/// A multi-point automation curve for clip gain or pan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EnvelopeCurve {
    /// Sorted control points in ascending frame order.
    pub points: Vec<EnvelopePoint>,
    /// Interpolation algorithm between adjacent points.
    pub mode: InterpolationMode,
}

impl EnvelopeCurve {
    /// Create an empty envelope curve with the default interpolation mode.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an envelope curve with an explicit interpolation mode.
    #[must_use]
    pub fn with_mode(mode: InterpolationMode) -> Self {
        Self {
            points: Vec::new(),
            mode,
        }
    }

    /// Create an envelope curve with pre-sorted control points.
    #[must_use]
    pub fn with_points(mut points: Vec<EnvelopePoint>, mode: InterpolationMode) -> Self {
        points.sort_by_key(|p| p.frame);
        Self { points, mode }
    }

    /// Number of control points in the envelope.
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Whether the envelope has no control points.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Remove all control points from the curve.
    pub fn clear(&mut self) {
        self.points.clear();
    }

    /// Insert or update a control point, maintaining ascending frame order.
    pub fn add_point(&mut self, frame: u64, gain: f32) {
        let clamped_gain = gain.clamp(0.0, 4.0);
        if let Some(existing) = self.points.iter_mut().find(|p| p.frame == frame) {
            existing.gain = clamped_gain;
            return;
        }

        let idx = self
            .points
            .binary_search_by_key(&frame, |p| p.frame)
            .unwrap_or_else(|insert_idx| insert_idx);

        self.points.insert(
            idx,
            EnvelopePoint {
                frame,
                gain: clamped_gain,
            },
        );
    }

    /// Remove a control point within `tolerance_frames` of `frame`. Returns `true` if removed.
    pub fn remove_point_at(&mut self, frame: u64, tolerance_frames: u64) -> bool {
        if let Some(pos) = self
            .points
            .iter()
            .position(|p| p.frame.abs_diff(frame) <= tolerance_frames)
        {
            self.points.remove(pos);
            true
        } else {
            false
        }
    }

    /// Evaluate the curve at a relative frame offset inside a clip of `clip_len` frames.
    ///
    /// If no points exist, unity gain (`1.0`) is returned.
    /// If `frame` is before the first point, the first point's gain is returned.
    /// If `frame` is past the last point, the last point's gain is returned.
    #[must_use]
    pub fn evaluate_at(&self, frame: u64, _clip_len: u64) -> f32 {
        if self.points.is_empty() {
            return 1.0;
        }

        if frame <= self.points[0].frame {
            return self.points[0].gain;
        }

        if let Some(last) = self.points.last()
            && frame >= last.frame
        {
            return last.gain;
        }

        // Binary search to find segment [p0, p1]
        let idx = match self.points.binary_search_by_key(&frame, |p| p.frame) {
            Ok(exact) => return self.points[exact].gain,
            Err(next_idx) => next_idx,
        };

        let p0 = &self.points[idx - 1];
        let p1 = &self.points[idx];

        let span = (p1.frame - p0.frame).max(1) as f32;
        let delta = (frame - p0.frame) as f32;
        let t = (delta / span).clamp(0.0, 1.0);

        interpolate(p0.gain, p1.gain, t, self.mode)
    }
}

/// Compute interpolated gain between two values based on normalized fraction `t` in `[0.0, 1.0]`.
#[inline]
fn interpolate(v0: f32, v1: f32, t: f32, mode: InterpolationMode) -> f32 {
    match mode {
        InterpolationMode::Linear => v0 + (v1 - v0) * t,
        InterpolationMode::Smooth => {
            // Cosine S-curve: (1 - cos(pi * t)) / 2
            let smooth_t = (1.0 - (std::f32::consts::PI * t).cos()) * 0.5;
            v0 + (v1 - v0) * smooth_t
        }
        InterpolationMode::Exponential => {
            // Perceptually convex curve for audio fades
            if (v1 - v0).abs() < f32::EPSILON {
                v0
            } else {
                let curve = t * t;
                v0 + (v1 - v0) * curve
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_envelope_returns_unity() {
        let env = EnvelopeCurve::new();
        assert_eq!(env.evaluate_at(0, 1000), 1.0);
        assert_eq!(env.evaluate_at(500, 1000), 1.0);
    }

    #[test]
    fn single_point_returns_constant() {
        let mut env = EnvelopeCurve::new();
        env.add_point(500, 0.5);
        assert_eq!(env.evaluate_at(0, 1000), 0.5);
        assert_eq!(env.evaluate_at(500, 1000), 0.5);
        assert_eq!(env.evaluate_at(1000, 1000), 0.5);
    }

    #[test]
    fn linear_interpolation_works() {
        let mut env = EnvelopeCurve::with_mode(InterpolationMode::Linear);
        env.add_point(0, 0.0);
        env.add_point(1000, 1.0);

        assert!((env.evaluate_at(0, 1000) - 0.0).abs() < 1e-6);
        assert!((env.evaluate_at(500, 1000) - 0.5).abs() < 1e-6);
        assert!((env.evaluate_at(1000, 1000) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn smooth_interpolation_is_bounded() {
        let mut env = EnvelopeCurve::with_mode(InterpolationMode::Smooth);
        env.add_point(0, 0.0);
        env.add_point(1000, 1.0);

        let mid = env.evaluate_at(500, 1000);
        assert!((mid - 0.5).abs() < 1e-4);
        assert!(env.evaluate_at(250, 1000) < 0.5);
        assert!(env.evaluate_at(750, 1000) > 0.5);
    }

    #[test]
    fn add_and_remove_points() {
        let mut env = EnvelopeCurve::new();
        env.add_point(100, 0.8);
        env.add_point(200, 0.4);
        assert_eq!(env.len(), 2);

        // Update existing point
        env.add_point(100, 0.9);
        assert_eq!(env.len(), 2);
        assert!((env.evaluate_at(100, 1000) - 0.9).abs() < 1e-6);

        // Remove within tolerance
        assert!(env.remove_point_at(102, 5));
        assert_eq!(env.len(), 1);
        assert!(!env.remove_point_at(500, 5));
    }
}
