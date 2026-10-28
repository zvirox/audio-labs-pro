//! GPU-backed waveform drawing from a min/max peak pyramid.
//!
//! The toolkit owns the display format so it has no dependency on `alp-media`.
//! `alp-ui` converts a `PeakPyramid` once when media is loaded. Drawing touches
//! only the buckets that intersect the viewport, which keeps a one-hour source
//! inside the frame budget.
//!
//! The widget is a `canvas::Program`, so iced renders geometry through its
//! active GPU renderer (`iced_renderer`'s wgpu backend by default).

use std::fmt;
use std::sync::Arc;

use iced::widget::canvas;
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

use crate::fonts;
use crate::theme::ThemeColors;

/// One pyramid level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeakLevel {
    bucket_samples: u32,
    pairs: Vec<(i16, i16)>,
}

impl PeakLevel {
    /// Create a level. Empty pairs are allowed but a zero bucket is not.
    pub fn new(bucket_samples: u32, pairs: Vec<(i16, i16)>) -> Result<Self, WaveformError> {
        if bucket_samples == 0 {
            return Err(WaveformError::ZeroBucket);
        }
        Ok(Self {
            bucket_samples,
            pairs,
        })
    }

    /// Frames represented by one bucket.
    #[must_use]
    pub fn bucket_samples(&self) -> u32 {
        self.bucket_samples
    }

    /// Min/max pairs in file order.
    #[must_use]
    pub fn pairs(&self) -> &[(i16, i16)] {
        &self.pairs
    }

    /// Number of buckets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether the level has no buckets.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

/// Invalid waveform data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveformError {
    /// No pyramid levels were supplied.
    NoLevels,
    /// The sample rate was zero.
    ZeroSampleRate,
    /// No frames were supplied.
    ZeroFrames,
    /// A level had a zero bucket size.
    ZeroBucket,
    /// Levels were not ordered from finest to coarsest.
    UnorderedLevels,
}

impl fmt::Display for WaveformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoLevels => f.write_str("waveform needs at least one peak level"),
            Self::ZeroSampleRate => f.write_str("waveform sample rate must be positive"),
            Self::ZeroFrames => f.write_str("waveform needs at least one frame"),
            Self::ZeroBucket => f.write_str("peak level bucket must be positive"),
            Self::UnorderedLevels => f.write_str("peak levels must be ordered fine to coarse"),
        }
    }
}

impl std::error::Error for WaveformError {}

/// A complete peak pyramid prepared for drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaveformPeaks {
    sample_rate: u32,
    frames: u64,
    levels: Vec<PeakLevel>,
}

impl WaveformPeaks {
    /// Validate and own a pyramid.
    pub fn new(
        sample_rate: u32,
        frames: u64,
        levels: Vec<PeakLevel>,
    ) -> Result<Self, WaveformError> {
        if sample_rate == 0 {
            return Err(WaveformError::ZeroSampleRate);
        }
        if frames == 0 {
            return Err(WaveformError::ZeroFrames);
        }
        if levels.is_empty() {
            return Err(WaveformError::NoLevels);
        }
        for pair in levels.windows(2) {
            if pair[0].bucket_samples >= pair[1].bucket_samples {
                return Err(WaveformError::UnorderedLevels);
            }
        }
        Ok(Self {
            sample_rate,
            frames,
            levels,
        })
    }

    /// Frames per second.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Total frames.
    #[must_use]
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Total duration in seconds.
    #[must_use]
    pub fn duration_secs(&self) -> f32 {
        self.frames as f32 / self.sample_rate as f32
    }

    /// Levels from finest to coarsest.
    #[must_use]
    pub fn levels(&self) -> &[PeakLevel] {
        &self.levels
    }

    /// Choose the finest level whose bucket is not larger than one pixel.
    #[must_use]
    pub fn level_for(&self, samples_per_pixel: f64) -> &PeakLevel {
        let mut chosen = &self.levels[0];
        for level in &self.levels {
            if f64::from(level.bucket_samples) <= samples_per_pixel {
                chosen = level;
            }
        }
        chosen
    }

    /// Aggregate min/max over a half-open sample range.
    #[must_use]
    pub fn peak_between(&self, level: &PeakLevel, start: u64, end: u64) -> (i16, i16) {
        if level.is_empty() {
            return (0, 0);
        }
        let bucket = u64::from(level.bucket_samples);
        let first = (start / bucket) as usize;
        let last = ((end.saturating_sub(1)) / bucket) as usize;
        let first = first.min(level.pairs.len() - 1);
        let last = last.min(level.pairs.len() - 1);
        let mut lo = i16::MAX;
        let mut hi = i16::MIN;
        for &(min, max) in &level.pairs[first..=last] {
            lo = lo.min(min);
            hi = hi.max(max);
        }
        (lo, hi)
    }

    /// Total buckets across levels, for diagnostics.
    #[must_use]
    pub fn buckets(&self) -> usize {
        self.levels.iter().map(PeakLevel::len).sum()
    }

    /// Approximate retained bytes.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.levels
            .iter()
            .map(|level| level.pairs.len() * std::mem::size_of::<(i16, i16)>())
            .sum()
    }
}

/// A scrollable waveform view with a playhead and optional selection.
#[derive(Debug, Clone)]
pub struct WaveformWidget {
    peaks: Arc<WaveformPeaks>,
    scroll_secs: f32,
    visible_secs: f32,
    playhead_secs: f32,
    selection: Option<(f32, f32)>,
    colors: ThemeColors,
}

impl WaveformWidget {
    /// Smallest visible window.
    pub const MIN_VISIBLE_SECS: f32 = 0.25;
    /// Largest visible window.
    pub const MAX_VISIBLE_SECS: f32 = 3_600.0;

    /// Create a waveform view showing the whole source (up to one hour).
    #[must_use]
    pub fn new(peaks: Arc<WaveformPeaks>, colors: ThemeColors) -> Self {
        let visible_secs = peaks.duration_secs().clamp(Self::MIN_VISIBLE_SECS, 120.0);
        Self {
            peaks,
            scroll_secs: 0.0,
            visible_secs,
            playhead_secs: 0.0,
            selection: None,
            colors,
        }
    }

    /// Set the playhead.
    pub fn set_playhead(&mut self, secs: f32) {
        self.playhead_secs = secs.max(0.0);
    }

    /// Set or clear a selected time range.
    pub fn set_selection(&mut self, selection: Option<(f32, f32)>) {
        self.selection = selection;
    }

    /// Current visible window.
    #[must_use]
    pub fn visible(&self) -> (f32, f32) {
        (
            self.scroll_secs,
            (self.scroll_secs + self.visible_secs).min(self.peaks.duration_secs()),
        )
    }

    /// Scroll by a delta in seconds, clamped to the source.
    pub fn scroll_by(&mut self, secs: f32) {
        let max = (self.peaks.duration_secs() - self.visible_secs).max(0.0);
        self.scroll_secs = (self.scroll_secs + secs).clamp(0.0, max);
    }

    /// Re-tint the waveform (for example when the active workspace changes its
    /// neon accent, or when the theme mode flips).
    pub fn set_colors(&mut self, colors: ThemeColors) {
        self.colors = colors;
    }

    /// Zoom around the centre of the current view.
    pub fn zoom_by(&mut self, factor: f32) {
        let centre = self.scroll_secs + self.visible_secs / 2.0;
        self.visible_secs = (self.visible_secs * factor)
            .clamp(Self::MIN_VISIBLE_SECS, Self::MAX_VISIBLE_SECS)
            .min(self.peaks.duration_secs());
        self.scroll_secs = (centre - self.visible_secs / 2.0).clamp(
            0.0,
            (self.peaks.duration_secs() - self.visible_secs).max(0.0),
        );
    }

    /// Build a canvas element.
    #[must_use]
    pub fn view<Message: 'static>(&self) -> Element<'_, Message> {
        canvas(WaveformCanvas {
            peaks: self.peaks.clone(),
            scroll_secs: self.scroll_secs,
            visible_secs: self.visible_secs,
            playhead_secs: self.playhead_secs,
            selection: self.selection,
            colors: self.colors,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

struct WaveformCanvas {
    peaks: Arc<WaveformPeaks>,
    scroll_secs: f32,
    visible_secs: f32,
    playhead_secs: f32,
    selection: Option<(f32, f32)>,
    colors: ThemeColors,
}

impl<Message> canvas::Program<Message> for WaveformCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let width = bounds.width.max(1.0);
        let height = bounds.height.max(1.0);
        let mid = height / 2.0;
        let c = self.colors;

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), c.background);
        draw_grid(
            &mut frame,
            width,
            height,
            self.scroll_secs,
            self.visible_secs,
            c,
        );

        if let Some((start, end)) = self.selection {
            let x0 = ((start - self.scroll_secs) / self.visible_secs * width).clamp(0.0, width);
            let x1 = ((end - self.scroll_secs) / self.visible_secs * width).clamp(0.0, width);
            frame.fill_rectangle(
                Point::new(x0, 0.0),
                Size::new((x1 - x0).max(1.0), height),
                Color {
                    a: 0.16,
                    ..c.accent
                },
            );
        }

        let samples_per_column =
            f64::from(self.visible_secs) * f64::from(self.peaks.sample_rate) / f64::from(width);
        let level = self.peaks.level_for(samples_per_column);
        let columns = width as usize;
        let samples_per_pixel =
            self.visible_secs as f64 * f64::from(self.peaks.sample_rate) / width as f64;

        for column in 0..columns {
            let start = (f64::from(self.scroll_secs) * f64::from(self.peaks.sample_rate)
                + column as f64 * samples_per_pixel) as u64;
            let end = start + samples_per_pixel.max(1.0) as u64;
            if start >= self.peaks.frames {
                break;
            }
            let (min, max) = self.peaks.peak_between(level, start, end);
            let max = f32::from(max) / f32::from(i16::MAX);
            let min = f32::from(min) / f32::from(i16::MAX);
            let energy = (max - min).clamp(0.0, 1.0);
            let color = Color {
                r: c.wave.r + (c.wave_hot.r - c.wave.r) * energy,
                g: c.wave.g + (c.wave_hot.g - c.wave.g) * energy,
                b: c.wave.b + (c.wave_hot.b - c.wave.b) * energy,
                a: 0.92,
            };
            frame.fill_rectangle(
                Point::new(column as f32, mid - max * mid * 0.92),
                Size::new(1.0, ((max - min) * mid * 0.92).max(1.0)),
                color,
            );
        }

        frame.fill_rectangle(
            Point::new(0.0, mid),
            Size::new(width, 1.0),
            Color { a: 0.10, ..c.text },
        );

        let playhead_x =
            ((self.playhead_secs - self.scroll_secs) / self.visible_secs * width).clamp(0.0, width);
        if self.playhead_secs >= self.scroll_secs
            && self.playhead_secs <= self.scroll_secs + self.visible_secs
        {
            frame.fill_rectangle(
                Point::new(playhead_x, 0.0),
                Size::new(2.0, height),
                c.playhead,
            );
        }

        let (start, end) = (self.scroll_secs, self.scroll_secs + self.visible_secs);
        frame.fill_text(canvas::Text {
            content: format!("{:.1}s – {:.1}s", start, end),
            position: Point::new(8.0, height - 16.0),
            color: c.muted,
            size: 10.0.into(),
            font: fonts::regular(),
            ..canvas::Text::default()
        });

        vec![frame.into_geometry()]
    }
}

fn draw_grid(
    frame: &mut canvas::Frame,
    width: f32,
    height: f32,
    scroll_secs: f32,
    visible_secs: f32,
    colors: ThemeColors,
) {
    let tick = nice_tick(visible_secs, width);
    let first = (scroll_secs / tick).ceil() * tick;
    let mut at = first;
    while at < scroll_secs + visible_secs {
        let x = (at - scroll_secs) / visible_secs * width;
        frame.fill_rectangle(Point::new(x, 0.0), Size::new(1.0, height), colors.grid);
        frame.fill_text(canvas::Text {
            content: format_time(at),
            position: Point::new(x + 4.0, 5.0),
            color: colors.faint,
            size: 9.0.into(),
            font: fonts::regular(),
            ..canvas::Text::default()
        });
        at += tick;
    }
}

fn nice_tick(visible_secs: f32, width: f32) -> f32 {
    let target = (visible_secs / (width / 90.0).max(1.0)).max(0.1);
    for candidate in [
        0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0,
    ] {
        if candidate >= target {
            return candidate;
        }
    }
    900.0
}

fn format_time(secs: f32) -> String {
    let whole = secs.max(0.0) as u64;
    format!("{}:{:02}", whole / 60, whole % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_peaks() -> WaveformPeaks {
        let level0 = PeakLevel::new(2, vec![(-100, 100), (-200, 200), (-50, 50), (-300, 300)])
            .expect("valid");
        let level1 = PeakLevel::new(4, vec![(-200, 200), (-300, 300)]).expect("valid");
        WaveformPeaks::new(8, 16, vec![level0, level1]).expect("valid")
    }

    #[test]
    fn rejects_invalid_pyramids() {
        assert_eq!(
            WaveformPeaks::new(0, 10, vec![]),
            Err(WaveformError::ZeroSampleRate)
        );
        assert_eq!(
            WaveformPeaks::new(48_000, 0, vec![PeakLevel::new(2, vec![]).unwrap()]),
            Err(WaveformError::ZeroFrames)
        );
        assert_eq!(
            WaveformPeaks::new(48_000, 10, vec![]),
            Err(WaveformError::NoLevels)
        );
        assert!(matches!(
            PeakLevel::new(0, vec![]),
            Err(WaveformError::ZeroBucket)
        ));
        let coarse = PeakLevel::new(8, vec![]).unwrap();
        let fine = PeakLevel::new(2, vec![]).unwrap();
        assert_eq!(
            WaveformPeaks::new(48_000, 10, vec![coarse, fine]),
            Err(WaveformError::UnorderedLevels)
        );
    }

    #[test]
    fn level_selection_matches_pixel_scale() {
        let peaks = sample_peaks();
        assert_eq!(peaks.level_for(1.0).bucket_samples(), 2);
        assert_eq!(peaks.level_for(2.0).bucket_samples(), 2);
        assert_eq!(peaks.level_for(3.0).bucket_samples(), 2);
        assert_eq!(peaks.level_for(4.0).bucket_samples(), 4);
    }

    #[test]
    fn peak_between_aggregates_buckets() {
        let peaks = sample_peaks();
        let level = peaks.level_for(2.0);
        assert_eq!(peaks.peak_between(level, 0, 2), (-100, 100));
        assert_eq!(peaks.peak_between(level, 0, 8), (-300, 300));
        assert_eq!(peaks.peak_between(level, 15, 99), (-300, 300));
    }

    #[test]
    fn scroll_and_zoom_clamp_to_source() {
        let mut widget = WaveformWidget::new(Arc::new(sample_peaks()), ThemeColors::DARK);
        widget.scroll_by(100.0);
        assert_eq!(
            widget.visible().0,
            widget.peaks.duration_secs() - widget.visible_secs
        );
        widget.scroll_by(-100.0);
        assert_eq!(widget.visible().0, 0.0);
        widget.zoom_by(1000.0);
        assert!(widget.visible_secs >= WaveformWidget::MIN_VISIBLE_SECS);
        widget.zoom_by(0.0001);
        assert!(widget.visible_secs <= WaveformWidget::MAX_VISIBLE_SECS);
    }

    #[test]
    fn duration_is_frame_accurate() {
        assert!((sample_peaks().duration_secs() - 2.0).abs() < 1e-6);
    }
}
