//! Min/max peak pyramids for waveform drawing.
//!
//! The format and sizing follow ADR 0009: three levels with a base bucket of
//! 256 frames, each level four times coarser, stored as i16 min/max pairs.

use std::path::Path;

use crate::decode;
use crate::error::MediaError;
use crate::pcm::PcmBuffer;

/// Samples per bucket at level 0.
pub const BASE_BUCKET: u32 = 256;
/// Number of pyramid levels.
pub const LEVEL_COUNT: usize = 3;
/// Ratio between consecutive levels.
pub const LEVEL_RATIO: u32 = 4;
/// Cache file magic.
pub const CACHE_MAGIC: &[u8; 8] = b"ALPPEAKS";
/// Cache format version.
pub const CACHE_VERSION: u32 = 1;

/// One resolution level of a peak pyramid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeakLevel {
    pub(crate) bucket_samples: u32,
    pub(crate) pairs: Vec<(i16, i16)>,
}

impl PeakLevel {
    /// Frames represented by each bucket.
    #[must_use]
    pub fn bucket_samples(&self) -> u32 {
        self.bucket_samples
    }

    /// Interleaved min/max pairs, one per bucket.
    #[must_use]
    pub fn pairs(&self) -> &[(i16, i16)] {
        &self.pairs
    }

    /// Number of buckets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether the level contains no buckets.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

/// A complete min/max pyramid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeakPyramid {
    sample_rate: u32,
    frames: u64,
    levels: Vec<PeakLevel>,
}

impl PeakPyramid {
    /// Assemble a pyramid from cached parts, validating its shape.
    pub fn from_parts(
        sample_rate: u32,
        frames: u64,
        levels: Vec<PeakLevel>,
    ) -> Result<Self, MediaError> {
        if levels.is_empty() {
            return Err(MediaError::InvalidCache("pyramid has no levels".into()));
        }
        if levels[0].bucket_samples != BASE_BUCKET {
            return Err(MediaError::InvalidCache(format!(
                "base bucket is {}, expected {BASE_BUCKET}",
                levels[0].bucket_samples
            )));
        }
        for level in &levels {
            if level.pairs.is_empty() {
                return Err(MediaError::InvalidCache("level has no buckets".into()));
            }
        }
        Ok(Self {
            sample_rate,
            frames,
            levels,
        })
    }

    /// Build a pyramid from an in-memory buffer.
    pub fn build_from_buffer(buffer: &PcmBuffer) -> Result<Self, MediaError> {
        let mut builder = PeakBuilder::default();
        builder.push_chunk(buffer.samples(), buffer.channels())?;
        builder.finish(buffer.sample_rate())
    }

    /// Build a pyramid by streaming a file, without holding all samples in RAM.
    pub fn build_from_path(path: impl AsRef<Path>) -> Result<Self, MediaError> {
        let mut builder = PeakBuilder::default();
        let info =
            decode::decode_path(path, |chunk, channels| builder.push_chunk(chunk, channels))?;
        builder.finish(info.sample_rate)
    }

    /// Source sample rate.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Source length in frames.
    #[must_use]
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Pyramid levels from finest to coarsest.
    #[must_use]
    pub fn levels(&self) -> &[PeakLevel] {
        &self.levels
    }

    /// Level best suited to a zoom of `samples_per_pixel`.
    #[must_use]
    pub fn level_for(&self, samples_per_pixel: f64) -> &PeakLevel {
        let mut chosen = &self.levels[0];
        for level in &self.levels {
            if f64::from(level.bucket_samples) <= samples_per_pixel {
                chosen = level;
            } else {
                break;
            }
        }
        chosen
    }

    /// Merge a bucket range into one min/max pair for a canvas span.
    #[must_use]
    pub fn peak_between(&self, level_index: usize, start_frame: u64, end_frame: u64) -> (i16, i16) {
        let Some(level) = self.levels.get(level_index) else {
            return (0, 0);
        };
        let bucket = u64::from(level.bucket_samples);
        let first = (start_frame / bucket) as usize;
        let last = (end_frame.saturating_sub(1) / bucket) as usize;
        let mut min = i16::MAX;
        let mut max = i16::MIN;
        // `start > end` (an inverted range) must not underflow the span len:
        // yield an empty window instead of wrapping into a huge take.
        let span = last.saturating_add(1).saturating_sub(first);
        for (low, high) in level.pairs.iter().skip(first).take(span) {
            min = min.min(*low);
            max = max.max(*high);
        }
        if min > max { (0, 0) } else { (min, max) }
    }
}

/// Streaming pyramid builder.
#[derive(Debug, Default)]
pub struct PeakBuilder {
    channels: u16,
    base: Vec<(i16, i16)>,
    min: i16,
    max: i16,
    frame_in_bucket: u32,
    frames: u64,
}

impl PeakBuilder {
    /// Feed an interleaved chunk. Channel count must stay constant.
    pub fn push_chunk(&mut self, chunk: &[f32], channels: u16) -> Result<(), MediaError> {
        if channels == 0 {
            return Err(MediaError::InvalidBuffer("zero channels".into()));
        }
        if !chunk.len().is_multiple_of(usize::from(channels)) {
            return Err(MediaError::InvalidBuffer(
                "chunk is not frame-aligned".into(),
            ));
        }
        if self.frames == 0 {
            self.channels = channels;
            self.min = i16::MAX;
            self.max = i16::MIN;
        } else if self.channels != channels {
            return Err(MediaError::InvalidBuffer(
                "channel count changed mid-stream".into(),
            ));
        }

        let channels = usize::from(channels);
        for frame in chunk.chunks_exact(channels) {
            let mono = if channels == 1 {
                frame[0]
            } else {
                frame.iter().copied().sum::<f32>() / channels as f32
            };
            let sample = (mono.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
            self.min = self.min.min(sample);
            self.max = self.max.max(sample);
            self.frame_in_bucket += 1;
            self.frames += 1;
            if self.frame_in_bucket == BASE_BUCKET {
                self.base.push((self.min, self.max));
                self.min = i16::MAX;
                self.max = i16::MIN;
                self.frame_in_bucket = 0;
            }
        }
        Ok(())
    }

    /// Finish the pyramid at a known sample rate.
    pub fn finish(mut self, sample_rate: u32) -> Result<PeakPyramid, MediaError> {
        if self.frames == 0 {
            return Err(MediaError::Decode("decoded no audio frames".into()));
        }
        if self.frame_in_bucket > 0 {
            self.base.push((self.min, self.max));
        }
        let mut levels = vec![PeakLevel {
            bucket_samples: BASE_BUCKET,
            pairs: self.base,
        }];
        for _ in 1..LEVEL_COUNT {
            let previous = &levels.last().expect("at least one level").pairs;
            let mut next = Vec::with_capacity(previous.len().div_ceil(LEVEL_RATIO as usize));
            for group in previous.chunks(LEVEL_RATIO as usize) {
                let low = group.iter().map(|pair| pair.0).min().unwrap_or(0);
                let high = group.iter().map(|pair| pair.1).max().unwrap_or(0);
                next.push((low, high));
            }
            let bucket_samples = levels
                .last()
                .expect("at least one level")
                .bucket_samples
                .saturating_mul(LEVEL_RATIO);
            levels.push(PeakLevel {
                bucket_samples,
                pairs: next,
            });
        }
        PeakPyramid::from_parts(sample_rate, self.frames, levels)
    }

    /// Frames pushed so far.
    #[must_use]
    pub fn frames(&self) -> u64 {
        self.frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_buffer_produces_one_bucket_per_level() {
        let buffer = PcmBuffer::new(48_000, 1, vec![0.5, -0.5, 0.25, -0.25]).expect("buffer");
        let pyramid = PeakPyramid::build_from_buffer(&buffer).expect("pyramid");
        assert_eq!(pyramid.frames(), 4);
        assert_eq!(pyramid.levels().len(), 3);
        assert_eq!(pyramid.levels()[0].len(), 1);
        assert_eq!(pyramid.levels()[2].bucket_samples, 4096);
        let (min, max) = pyramid.levels()[0].pairs()[0];
        assert!(min < 0 && max > 0);
    }

    #[test]
    fn level_selection_prefers_finer_levels_when_zoomed_in() {
        let buffer = PcmBuffer::new(48_000, 1, vec![0.0; 20_000]).expect("buffer");
        let pyramid = PeakPyramid::build_from_buffer(&buffer).expect("pyramid");
        assert_eq!(pyramid.level_for(1.0).bucket_samples(), BASE_BUCKET);
        assert_eq!(pyramid.level_for(2_000.0).bucket_samples(), 1024);
    }

    #[test]
    fn peak_between_handles_inverted_range_without_panic() {
        let buffer = PcmBuffer::new(48_000, 1, vec![0.5; 10_000]).expect("buffer");
        let pyramid = PeakPyramid::build_from_buffer(&buffer).expect("pyramid");
        let (min, max) = pyramid.peak_between(0, 5_000, 1_000);
        assert_eq!(
            (min, max),
            (0, 0),
            "an inverted frame range must yield an empty window, not an underflow"
        );
    }
}
