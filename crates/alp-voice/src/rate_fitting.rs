//! Speech-rate planning and take fitting for dubbing alignment.
//!
//! TTS speed is the first-choice tool for matching a translated take to a
//! reference dialogue window. When the required speed is outside the engine's
//! range, the planner falls back to pitch-preserving WSOLA time stretching.

use alp_media::{PcmBuffer, time_stretch};
use serde::{Deserialize, Serialize};

/// Default slowest accepted TTS speed multiplier.
pub const DEFAULT_MIN_SPEED: f32 = 0.5;
/// Default fastest accepted TTS speed multiplier.
pub const DEFAULT_MAX_SPEED: f32 = 2.0;

/// A reference subtitle/dialogue window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FitWindow {
    /// Window start in seconds.
    pub start_secs: f64,
    /// Window end in seconds.
    pub end_secs: f64,
}

impl FitWindow {
    /// Window duration in seconds, never negative.
    #[must_use]
    pub fn duration_secs(&self) -> f64 {
        (self.end_secs - self.start_secs).max(0.0)
    }
}

/// How a take should be adjusted to fit a target window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FitPlan {
    /// Duration of the rendered take in seconds.
    pub current_duration_secs: f64,
    /// Target window duration in seconds.
    pub target_duration_secs: f64,
    /// TTS speed to use when re-synthesising the line.
    pub recommended_speed: f32,
    /// Residual WSOLA stretch ratio (1.0 means no time stretching).
    pub stretch_ratio: f32,
    /// Whether WSOLA is required after TTS speed adjustment.
    pub uses_wsola: bool,
    /// Whether TTS speed alone can hit the window.
    pub within_tts_range: bool,
}

/// Build a fit plan for a take of `current_duration_secs` targeting `target_duration_secs`.
#[must_use]
pub fn plan_fit(
    current_duration_secs: f64,
    target_duration_secs: f64,
    min_speed: f32,
    max_speed: f32,
) -> FitPlan {
    let min_speed = min_speed.max(0.01);
    let max_speed = max_speed.max(min_speed);
    if current_duration_secs <= 0.0 || target_duration_secs <= 0.0 {
        return FitPlan {
            current_duration_secs,
            target_duration_secs,
            recommended_speed: 1.0,
            stretch_ratio: 1.0,
            uses_wsola: false,
            within_tts_range: true,
        };
    }

    let required_speed = (current_duration_secs / target_duration_secs) as f32;
    let within = required_speed >= min_speed && required_speed <= max_speed;
    let recommended = required_speed.clamp(min_speed, max_speed);
    let speed_adjusted_duration = current_duration_secs / f64::from(recommended);
    let stretch_ratio = (target_duration_secs / speed_adjusted_duration).clamp(0.01, 4.0) as f32;
    let uses_wsola = (stretch_ratio - 1.0).abs() > 0.01;

    FitPlan {
        current_duration_secs,
        target_duration_secs,
        recommended_speed: recommended,
        stretch_ratio: if uses_wsola { stretch_ratio } else { 1.0 },
        uses_wsola,
        within_tts_range: within,
    }
}

/// Fit an already-rendered take to a target duration.
///
/// The returned plan's `recommended_speed` describes the equivalent TTS speed;
/// this function applies only the residual WSOLA stretch needed now.
pub fn fit_take(
    take: &PcmBuffer,
    target_duration_secs: f64,
) -> Result<(PcmBuffer, FitPlan), alp_media::MediaError> {
    let planning = plan_fit(
        take.duration_secs(),
        target_duration_secs,
        DEFAULT_MIN_SPEED,
        DEFAULT_MAX_SPEED,
    );
    let direct_ratio = if take.duration_secs() > 0.0 && target_duration_secs > 0.0 {
        (target_duration_secs / take.duration_secs()).clamp(0.01, 4.0) as f32
    } else {
        1.0
    };
    let plan = FitPlan {
        stretch_ratio: direct_ratio,
        uses_wsola: (direct_ratio - 1.0).abs() > 0.01,
        ..planning
    };
    let fitted = time_stretch(take, plan.stretch_ratio)?;
    Ok((fitted, plan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn tone(rate: u32, frames: usize) -> PcmBuffer {
        let samples: Vec<f32> = (0..frames)
            .map(|i| 0.3 * (TAU * 220.0 * i as f32 / rate as f32).sin())
            .collect();
        PcmBuffer::new(rate, 1, samples).expect("pcm")
    }

    #[test]
    fn window_duration_is_non_negative() {
        let window = FitWindow {
            start_secs: 4.0,
            end_secs: 2.0,
        };
        assert_eq!(window.duration_secs(), 0.0);
    }

    #[test]
    fn plan_prefers_tts_speed_when_in_range() {
        let plan = plan_fit(4.0, 2.0, 0.5, 2.0);
        assert!(plan.within_tts_range);
        assert!(!plan.uses_wsola);
        assert!((plan.recommended_speed - 2.0).abs() < 0.01);
        assert!((plan.stretch_ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn plan_falls_back_to_wsola_beyond_speed_range() {
        let plan = plan_fit(6.0, 1.0, 0.5, 2.0);
        assert!(!plan.within_tts_range);
        assert!(plan.uses_wsola);
        assert!((plan.recommended_speed - 2.0).abs() < 0.01);
        assert!((plan.stretch_ratio - 0.3333).abs() < 0.01);
    }

    #[test]
    fn fit_take_reaches_the_target_duration() {
        let take = tone(48_000, 48_000);
        let (fitted, plan) = fit_take(&take, 0.75).expect("fit");
        assert!(plan.uses_wsola);
        assert!((fitted.duration_secs() - 0.75).abs() < 0.02);
    }
}
