//! Integer quantization and optional TPDF dither.
//!
//! Dither is deterministic within one export so repeated renders of the same
//! project are reproducible. The noise is triangular (the sum of two uniform
//! values) at one least-significant-bit peak amplitude, the standard choice for
//! word-length reduction.

use std::str::FromStr;

use crate::error::MediaError;

/// Dither policy for integer exports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DitherMode {
    /// Truncate to the nearest integer after rounding.
    #[default]
    None,
    /// Triangular probability-density dither at one LSB.
    Triangular,
}

impl DitherMode {
    /// Short label used by the CLI and UI.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Triangular => "triangular",
        }
    }
}

impl FromStr for DitherMode {
    type Err = MediaError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "none" | "off" | "0" => Ok(Self::None),
            "triangular" | "tpdf" | "on" | "1" => Ok(Self::Triangular),
            other => Err(MediaError::Encode(format!(
                "unknown dither mode {other:?} (use none or triangular)"
            ))),
        }
    }
}

/// Round a normalized float to an integer range with optional TPDF dither.
#[derive(Debug)]
pub(crate) struct Quantizer {
    max: i32,
    min: i32,
    scale: f32,
    dither: DitherMode,
    state: u64,
}

impl Quantizer {
    pub(crate) fn new(max: i32, dither: DitherMode) -> Self {
        Self {
            max,
            min: -max - 1,
            scale: (max as f32) + 1.0,
            dither,
            state: 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub(crate) fn quantize(&mut self, sample: f32) -> i32 {
        let sample = sample.clamp(-1.0, 1.0);
        let noise = match self.dither {
            DitherMode::None => 0.0,
            DitherMode::Triangular => {
                let first = self.uniform();
                let second = self.uniform();
                first - second
            }
        };
        let value = (sample * self.scale + noise).round() as i32;
        value.clamp(self.min, self.max)
    }

    fn uniform(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        let bits = (self.state >> 40) as u32;
        bits as f32 / ((1u32 << 24) - 1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantization_clamps_and_rounds() {
        let mut quantizer = Quantizer::new(32_767, DitherMode::None);
        assert_eq!(quantizer.quantize(0.0), 0);
        assert_eq!(quantizer.quantize(1.0), 32_767);
        assert_eq!(quantizer.quantize(-1.0), -32_768);
        assert_eq!(quantizer.quantize(2.0), 32_767);
    }

    #[test]
    fn dither_is_deterministic_and_non_silent() {
        let mut first = Quantizer::new(32_767, DitherMode::Triangular);
        let mut second = Quantizer::new(32_767, DitherMode::Triangular);
        let a: Vec<i32> = (0..64).map(|_| first.quantize(0.0)).collect();
        let b: Vec<i32> = (0..64).map(|_| second.quantize(0.0)).collect();
        assert_eq!(a, b);
        assert!(a.iter().any(|sample| *sample != 0));
        assert!(a.iter().all(|sample| sample.abs() <= 1));
    }

    #[test]
    fn dither_mode_parses() {
        assert_eq!(
            "tpdf".parse::<DitherMode>().expect("parse"),
            DitherMode::Triangular
        );
        assert!("bad".parse::<DitherMode>().is_err());
    }
}
