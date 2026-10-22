//! Peak-pyramid generation and a canvas waveform renderer.
//!
//! The spike proves the Phase 2 performance budget: a 1-hour source must scroll
//! at 60 fps while only touching the peaks that intersect the viewport.

use std::sync::Arc;

use iced::widget::canvas;
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, mouse};

const BASE_BUCKET: usize = 256;
const LEVELS: usize = 3;

#[derive(Debug)]
pub struct PeakLevel {
    /// Interleaved min/max pairs, -1.0..=1.0.
    pairs: Vec<(f32, f32)>,
    bucket_samples: usize,
}

impl PeakLevel {
    fn bucket_at(&self, sample: u64) -> Option<(f32, f32)> {
        let index = sample as usize / self.bucket_samples;
        self.pairs.get(index).copied()
    }
}

#[derive(Debug)]
pub struct PeakPyramid {
    levels: Vec<PeakLevel>,
    sample_rate: u32,
    samples: u64,
}

impl PeakPyramid {
    /// Build a pyramid from a synthetic 1-hour signal so the spike has no
    /// external media dependency.
    pub fn synthetic_hour() -> Self {
        let sample_rate = 48_000;
        let samples = 3_600u64 * sample_rate as u64;

        let mut pairs: Vec<(f32, f32)> = Vec::with_capacity((samples as usize) / BASE_BUCKET);
        let mut min = f32::MAX;
        let mut max = f32::MIN;

        // Sine LUT: avoids ~500M transcendental calls for the synthetic hour.
        let lut: Vec<f32> = (0..4096)
            .map(|i| (std::f32::consts::TAU * i as f32 / 4096.0).sin())
            .collect();
        let lut_index = |hz: f32, t: f32| -> f32 {
            let phase = (hz * t * 4096.0) as usize % 4096;
            lut[phase]
        };
        let phrase_lut: Vec<f32> = (0..4096)
            .map(|i| {
                let phase = std::f32::consts::TAU * i as f32 / 4096.0;
                phase.sin().max(0.0).powf(0.6)
            })
            .collect();
        let noise = |i: u64| -> f32 {
            let mut x = i
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            x ^= x >> 33;
            ((x & 0xFFFF) as f32 / 65535.0 - 0.5) * 0.12
        };

        for i in 0..samples {
            let t = i as f32 / sample_rate as f32;
            // Speech-like envelope: phrase bursts with pauses, plus tremolo.
            let phrase = phrase_lut[(t * 0.22 * 4096.0) as usize % 4096];
            let tremolo = 0.55 + 0.45 * lut_index(11.0, t);
            let carrier =
                lut_index(220.0, t) * 0.6 + lut_index(440.0, t) * 0.25 + lut_index(97.0, t) * 0.15;
            let sample = (carrier * phrase * tremolo + noise(i)).clamp(-1.0, 1.0);

            min = min.min(sample);
            max = max.max(sample);

            if (i as usize + 1).is_multiple_of(BASE_BUCKET) {
                pairs.push((min, max));
                min = f32::MAX;
                max = f32::MIN;
            }
        }

        let mut levels = vec![PeakLevel {
            pairs,
            bucket_samples: BASE_BUCKET,
        }];

        for _ in 1..LEVELS {
            let previous = &levels.last().unwrap().pairs;
            let mut next = Vec::with_capacity(previous.len() / 4 + 1);
            for chunk in previous.chunks(4) {
                let lo = chunk.iter().map(|p| p.0).fold(f32::MAX, f32::min);
                let hi = chunk.iter().map(|p| p.1).fold(f32::MIN, f32::max);
                next.push((lo, hi));
            }
            levels.push(PeakLevel {
                pairs: next,
                bucket_samples: levels.last().unwrap().bucket_samples * 4,
            });
        }

        for level in &levels {
            eprintln!(
                "  level bucket={} buckets={}",
                level.bucket_samples,
                level.pairs.len()
            );
        }

        Self {
            levels,
            sample_rate,
            samples,
        }
    }

    fn bytes(&self) -> usize {
        self.levels
            .iter()
            .map(|l| l.pairs.len() * std::mem::size_of::<(f32, f32)>())
            .sum()
    }

    fn buckets(&self) -> usize {
        self.levels.iter().map(|l| l.pairs.len()).sum()
    }

    /// Aggregate min/max over `[start_sample, end_sample)` at the coarsest level
    /// that still gives enough detail for one screen pixel.
    fn range(&self, start_sample: u64, end_sample: u64) -> (f32, f32) {
        let span = end_sample.saturating_sub(start_sample).max(1);
        let mut level = self.levels.last().unwrap();
        for candidate in self.levels.iter().rev() {
            if (span as usize / candidate.bucket_samples) >= 2 {
                level = candidate;
                break;
            }
        }

        let first = level.bucket_at(start_sample).unwrap_or((0.0, 0.0));
        let last = level
            .bucket_at(end_sample.saturating_sub(1))
            .unwrap_or(first);
        (first.0.min(last.0), first.1.max(last.1))
    }
}

/// View parameters for the scrolling waveform.
pub struct Waveform {
    peaks: Arc<PeakPyramid>,
    pub scroll_secs: f32,
    pub visible_secs: f32,
}

impl Waveform {
    pub fn synthetic_hour() -> Self {
        Self {
            peaks: Arc::new(PeakPyramid::synthetic_hour()),
            scroll_secs: 0.0,
            visible_secs: 120.0,
        }
    }

    pub fn scroll_by(&mut self, secs: f32) {
        let max = (self.total_secs() - self.visible_secs).max(0.0);
        self.scroll_secs = (self.scroll_secs + secs).min(max);
    }

    pub fn total_secs(&self) -> f32 {
        self.peaks.samples as f32 / self.peaks.sample_rate as f32
    }

    pub fn buckets(&self) -> usize {
        self.peaks.buckets()
    }

    pub fn bytes(&self) -> usize {
        self.peaks.bytes()
    }

    pub fn view(&self) -> Element<'_, crate::Message> {
        canvas(WaveformCanvas {
            peaks: self.peaks.clone(),
            scroll_secs: self.scroll_secs,
            visible_secs: self.visible_secs,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

struct WaveformCanvas {
    peaks: Arc<PeakPyramid>,
    scroll_secs: f32,
    visible_secs: f32,
}

impl canvas::Program<crate::Message> for WaveformCanvas {
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
        let width = bounds.width;
        let height = bounds.height;
        let mid = height / 2.0;

        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgba(0.02, 0.025, 0.035, 0.55),
        );

        // Time ruler: one tick per second of a 10 s grid.
        let tick_secs = 10.0;
        let first_tick = (self.scroll_secs / tick_secs).ceil() * tick_secs;
        let mut tick = first_tick;
        while tick < self.scroll_secs + self.visible_secs {
            let x = (tick - self.scroll_secs) / self.visible_secs * width;
            frame.fill_rectangle(
                Point::new(x, 0.0),
                iced::Size::new(1.0, height),
                Color::from_rgba(1.0, 1.0, 1.0, 0.05),
            );
            frame.fill_text(canvas::Text {
                content: format!("{}:{:02}", (tick as u64) / 60, (tick as u64) % 60),
                position: Point::new(x + 5.0, 6.0),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.35),
                size: 10.0.into(),
                ..canvas::Text::default()
            });
            tick += tick_secs;
        }

        let sr = self.peaks.sample_rate as f64;
        let columns = width.max(1.0) as usize;
        let samples_per_column = self.visible_secs as f64 * sr / columns as f64;

        for column in 0..columns {
            let start =
                ((self.scroll_secs as f64 * sr) + column as f64 * samples_per_column) as u64;
            let end = start + samples_per_column as u64 + 1;
            let (min, max) = self.peaks.range(start, end);
            let energy = (max - min).clamp(0.0, 1.0);
            let color = Color::from_rgb(
                0.35 + 0.30 * energy,
                0.72 - 0.18 * energy,
                0.95 - 0.25 * energy,
            );
            frame.fill_rectangle(
                Point::new(column as f32, mid - max * mid * 0.92),
                iced::Size::new(1.0, ((max - min) * mid * 0.92).max(1.0)),
                color,
            );
        }

        // Playhead
        frame.fill_rectangle(
            Point::new(0.0, 0.0),
            iced::Size::new(2.0, height),
            Color::from_rgb(1.0, 0.42, 0.42),
        );

        vec![frame.into_geometry()]
    }
}
