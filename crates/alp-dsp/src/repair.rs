//! Targeted spectral repair processors operating on a 2D time–frequency region.
//!
//! `SpectralNotchFilter` attenuates a narrow band around a centre frequency,
//! while `HarmonicSuppressor` attenuates a fundamental and its overtones. Both
//! share the `SpectralRepair` STFT overlap-add engine, whose buffers are
//! allocated in `prepare` so `process` performs no per-block allocation for
//! buffers up to the reserved capacity.

use std::collections::VecDeque;
use std::f32::consts::PI;
use std::sync::Arc;

use realfft::num_complex::Complex32;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};

use crate::Effect;
use crate::denoise::{SPECTRAL_BINS, SPECTRAL_FFT_SIZE, SPECTRAL_HOP_SIZE};

/// Which repair mask is applied to the selected region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepairMode {
    /// Attenuate one narrow band around `center_hz`.
    #[default]
    Notch,
    /// Attenuate `fundamental_hz` and its overtones.
    Harmonic,
}

/// Public parameters of a spectral repair processor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpectralRepairConfig {
    /// Repair mask shape.
    pub mode: RepairMode,
    /// Notch centre frequency in Hz.
    pub center_hz: f32,
    /// Bandwidth quality factor: `bandwidth_hz = center_hz / bandwidth_q`.
    pub bandwidth_q: f32,
    /// Suppressor fundamental in Hz.
    pub fundamental_hz: f32,
    /// Number of overtones to suppress, including the fundamental.
    pub harmonics: usize,
    /// Maximum attenuation in dB inside the mask (positive value).
    pub reduction_db: f32,
    /// Wet mix ratio in `[0.0, 1.0]`.
    pub wet: f32,
}

impl Default for SpectralRepairConfig {
    fn default() -> Self {
        Self {
            mode: RepairMode::Notch,
            center_hz: 1_000.0,
            bandwidth_q: 12.0,
            fundamental_hz: 220.0,
            harmonics: 4,
            reduction_db: 24.0,
            wet: 1.0,
        }
    }
}

/// Preallocated STFT scratch, shared by all channels (processing is serial).
struct RepairScratch {
    time_in: Vec<f32>,
    time_out: Vec<f32>,
    spectrum: Vec<Complex32>,
    fwd_scratch: Vec<Complex32>,
    inv_scratch: Vec<Complex32>,
}

impl RepairScratch {
    fn new(r2c: &dyn RealToComplex<f32>, c2r: &dyn ComplexToReal<f32>) -> Self {
        Self {
            time_in: vec![0.0; SPECTRAL_FFT_SIZE],
            time_out: vec![0.0; SPECTRAL_FFT_SIZE],
            spectrum: vec![Complex32::default(); SPECTRAL_BINS],
            fwd_scratch: vec![Complex32::default(); r2c.get_scratch_len()],
            inv_scratch: vec![Complex32::default(); c2r.get_scratch_len()],
        }
    }

    fn reset(&mut self) {
        self.time_in.fill(0.0);
        self.time_out.fill(0.0);
        self.spectrum.fill(Complex32::default());
    }
}

/// Gain applied at a bin frequency, in `[min_gain, 1.0]`.
fn bin_gain(config: &SpectralRepairConfig, sample_rate: u32, frequency_hz: f32) -> f32 {
    let min_gain = 10.0f32.powf(-config.reduction_db / 20.0);
    let nyquist = sample_rate as f32 * 0.5;
    if frequency_hz <= 0.0 || frequency_hz >= nyquist {
        return 1.0;
    }

    let mut centres = [0.0f32; 16];
    let count = match config.mode {
        RepairMode::Notch => {
            centres[0] = config.center_hz;
            1
        }
        RepairMode::Harmonic => {
            let count = config.harmonics.max(1).min(centres.len());
            for (index, centre) in centres.iter_mut().enumerate().take(count) {
                *centre = config.fundamental_hz * (index + 1) as f32;
            }
            count
        }
    };

    let mut gain = 1.0f32;
    for &centre in centres.iter().take(count) {
        if centre <= 0.0 || centre >= nyquist {
            continue;
        }
        let bandwidth = (centre / config.bandwidth_q.max(0.5)).max(1.0);
        let half = bandwidth * 0.5;
        let offset = (frequency_hz - centre).abs();
        if offset >= half {
            continue;
        }
        // Flat-top with a 20% raised-cosine edge taper: full reduction across
        // the selected band, smooth enough at the edges to avoid zipper noise.
        let taper_start = half * 0.8;
        let shape = if offset <= taper_start {
            1.0
        } else {
            0.5 * (1.0 + (PI * (offset - taper_start) / (half - taper_start)).cos())
        };
        gain = gain.min(1.0 - (1.0 - min_gain) * shape);
    }
    gain
}

/// Process one deinterleaved channel through the STFT mask.
#[allow(clippy::too_many_arguments)]
fn process_channel(
    r2c: &dyn RealToComplex<f32>,
    c2r: &dyn ComplexToReal<f32>,
    window: &[f32],
    config: &SpectralRepairConfig,
    sample_rate: u32,
    fifo: &mut VecDeque<f32>,
    ola: &mut [f32],
    out_fifo: &mut VecDeque<f32>,
    scratch: &mut RepairScratch,
    input: &[f32],
    output: &mut [f32],
) {
    fifo.extend(input.iter().copied());
    while fifo.len() >= SPECTRAL_FFT_SIZE {
        for (i, sample) in scratch.time_in.iter_mut().enumerate() {
            *sample = fifo[i] * window[i];
        }
        if r2c
            .process_with_scratch(
                &mut scratch.time_in,
                &mut scratch.spectrum,
                &mut scratch.fwd_scratch,
            )
            .is_ok()
        {
            for (bin, bin_hz) in (0..SPECTRAL_BINS)
                .map(|k| k as f32 * sample_rate as f32 / SPECTRAL_FFT_SIZE as f32)
                .enumerate()
            {
                let gain = bin_gain(config, sample_rate, bin_hz);
                scratch.spectrum[bin].re *= gain;
                scratch.spectrum[bin].im *= gain;
            }
            if c2r
                .process_with_scratch(
                    &mut scratch.spectrum,
                    &mut scratch.time_out,
                    &mut scratch.inv_scratch,
                )
                .is_ok()
            {
                let norm = 1.0 / SPECTRAL_FFT_SIZE as f32;
                for (i, value) in scratch.time_out.iter().enumerate() {
                    ola[i] += *value * norm;
                }
                out_fifo.extend(ola[..SPECTRAL_HOP_SIZE].iter().copied());
                ola.copy_within(SPECTRAL_HOP_SIZE..SPECTRAL_FFT_SIZE, 0);
                ola[SPECTRAL_HOP_SIZE..].fill(0.0);
            }
        }
        fifo.drain(..SPECTRAL_HOP_SIZE);
    }
    for sample in output.iter_mut() {
        *sample = out_fifo.pop_front().unwrap_or(0.0);
    }
}

/// Shared STFT engine for targeted spectral attenuation.
pub struct SpectralRepair {
    config: SpectralRepairConfig,
    enabled: bool,
    sample_rate: u32,
    channels: u16,
    r2c: Arc<dyn RealToComplex<f32>>,
    c2r: Arc<dyn ComplexToReal<f32>>,
    window: Vec<f32>,
    in_fifos: Vec<VecDeque<f32>>,
    ola_buffers: Vec<Vec<f32>>,
    out_fifos: Vec<VecDeque<f32>>,
    channel_in: Vec<Vec<f32>>,
    channel_out: Vec<Vec<f32>>,
    dry_scratch: Vec<f32>,
    scratch: RepairScratch,
}

impl Default for SpectralRepair {
    fn default() -> Self {
        Self::new(SpectralRepairConfig::default())
    }
}

impl SpectralRepair {
    /// Create a repair engine with the given configuration.
    #[must_use]
    pub fn new(config: SpectralRepairConfig) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(SPECTRAL_FFT_SIZE);
        let c2r = planner.plan_fft_inverse(SPECTRAL_FFT_SIZE);
        let window = (0..SPECTRAL_FFT_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / SPECTRAL_FFT_SIZE as f32).cos()))
            .collect();
        let scratch = RepairScratch::new(r2c.as_ref(), c2r.as_ref());

        Self {
            config,
            enabled: true,
            sample_rate: 48_000,
            channels: 1,
            r2c,
            c2r,
            window,
            in_fifos: Vec::new(),
            ola_buffers: Vec::new(),
            out_fifos: Vec::new(),
            channel_in: Vec::new(),
            channel_out: Vec::new(),
            dry_scratch: Vec::new(),
            scratch,
        }
    }

    /// Build a notch filter.
    #[must_use]
    pub fn notch(center_hz: f32, bandwidth_q: f32, reduction_db: f32) -> Self {
        let config = SpectralRepairConfig {
            mode: RepairMode::Notch,
            center_hz,
            bandwidth_q,
            reduction_db,
            ..SpectralRepairConfig::default()
        };
        Self::new(config)
    }

    /// Build a harmonic suppressor.
    #[must_use]
    pub fn harmonic(
        fundamental_hz: f32,
        harmonics: usize,
        bandwidth_q: f32,
        reduction_db: f32,
    ) -> Self {
        let config = SpectralRepairConfig {
            mode: RepairMode::Harmonic,
            fundamental_hz,
            harmonics,
            bandwidth_q,
            reduction_db,
            ..SpectralRepairConfig::default()
        };
        Self::new(config)
    }

    /// Current configuration.
    #[must_use]
    pub fn config(&self) -> SpectralRepairConfig {
        self.config
    }

    /// Whether the processor is active.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable the processor. Disabling bypasses the input exactly.
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.reset();
            self.enabled = enabled;
        }
    }

    /// Set the wet mix ratio in `[0.0, 1.0]`.
    pub fn set_wet(&mut self, wet: f32) {
        self.config.wet = wet.clamp(0.0, 1.0);
    }

    /// Set the notch centre frequency in Hz.
    pub fn set_center_hz(&mut self, center_hz: f32) {
        self.config.center_hz = center_hz.max(0.0);
    }

    /// Set the notch bandwidth quality factor.
    pub fn set_bandwidth_q(&mut self, q: f32) {
        self.config.bandwidth_q = q.max(0.5);
    }

    /// Set the maximum attenuation in dB.
    pub fn set_reduction_db(&mut self, reduction_db: f32) {
        self.config.reduction_db = reduction_db.max(0.0);
    }

    /// Set the suppressor fundamental in Hz.
    pub fn set_fundamental_hz(&mut self, fundamental_hz: f32) {
        self.config.fundamental_hz = fundamental_hz.max(0.0);
    }

    /// Set the number of suppressed overtones.
    pub fn set_harmonics(&mut self, harmonics: usize) {
        self.config.harmonics = harmonics.min(16);
    }
}

impl Effect for SpectralRepair {
    fn name(&self) -> &'static str {
        match self.config.mode {
            RepairMode::Notch => "spectral_notch",
            RepairMode::Harmonic => "harmonic_suppressor",
        }
    }

    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        let same_format = self.sample_rate == sample_rate && self.channels == channels;
        self.sample_rate = sample_rate.max(1);
        self.channels = channels.max(1);
        if same_format && !self.in_fifos.is_empty() {
            return;
        }
        self.reset();
        let channels = usize::from(self.channels);
        self.in_fifos = (0..channels)
            .map(|_| VecDeque::with_capacity(SPECTRAL_FFT_SIZE * 4))
            .collect();
        self.out_fifos = (0..channels)
            .map(|_| VecDeque::with_capacity(SPECTRAL_FFT_SIZE * 4))
            .collect();
        self.ola_buffers = vec![vec![0.0; SPECTRAL_FFT_SIZE]; channels];
        self.channel_in = vec![Vec::with_capacity(4096); channels];
        self.channel_out = vec![Vec::with_capacity(4096); channels];
        self.dry_scratch = Vec::with_capacity(8192);
    }

    fn process(&mut self, buffer: &mut [f32]) {
        let channels = usize::from(self.channels);
        if channels == 0 || buffer.is_empty() {
            return;
        }
        if !self.enabled || self.config.wet <= 0.0 {
            return;
        }
        let frames = buffer.len() / channels;
        self.dry_scratch.resize(buffer.len(), 0.0);
        self.dry_scratch.copy_from_slice(buffer);

        let Self {
            config,
            sample_rate,
            r2c,
            c2r,
            window,
            in_fifos,
            ola_buffers,
            out_fifos,
            channel_in,
            channel_out,
            dry_scratch,
            scratch,
            ..
        } = self;

        for channel in 0..channels {
            let input = &mut channel_in[channel];
            input.resize(frames, 0.0);
            for frame in 0..frames {
                input[frame] = buffer[frame * channels + channel];
            }
            let output = &mut channel_out[channel];
            output.resize(frames, 0.0);
            process_channel(
                r2c.as_ref(),
                c2r.as_ref(),
                window,
                config,
                *sample_rate,
                &mut in_fifos[channel],
                &mut ola_buffers[channel],
                &mut out_fifos[channel],
                scratch,
                input,
                output,
            );
        }

        let dry_mix = 1.0 - config.wet;
        for (frame, output) in buffer.chunks_exact_mut(channels).enumerate() {
            for (channel, sample) in output.iter_mut().enumerate() {
                let wet = channel_out[channel][frame];
                *sample = dry_scratch[frame * channels + channel] * dry_mix + wet * config.wet;
            }
        }
    }

    fn reset(&mut self) {
        for fifo in &mut self.in_fifos {
            fifo.clear();
        }
        for fifo in &mut self.out_fifos {
            fifo.clear();
        }
        for ola in &mut self.ola_buffers {
            ola.fill(0.0);
        }
        self.scratch.reset();
    }
}

/// Narrow-band notch implemented on top of [`SpectralRepair`].
pub struct SpectralNotchFilter(SpectralRepair);

impl SpectralNotchFilter {
    /// Create a notch at `center_hz` with the given Q and attenuation.
    #[must_use]
    pub fn new(center_hz: f32, bandwidth_q: f32, reduction_db: f32) -> Self {
        Self(SpectralRepair::notch(center_hz, bandwidth_q, reduction_db))
    }

    /// Move the notch centre while the processor is live.
    pub fn set_center_hz(&mut self, center_hz: f32) {
        self.0.set_center_hz(center_hz);
    }

    /// Change the notch bandwidth while the processor is live.
    pub fn set_bandwidth_q(&mut self, q: f32) {
        self.0.set_bandwidth_q(q);
    }

    /// Change the attenuation while the processor is live.
    pub fn set_reduction_db(&mut self, db: f32) {
        self.0.set_reduction_db(db);
    }

    /// Enable or disable the notch.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.0.set_enabled(enabled);
    }
}

impl Effect for SpectralNotchFilter {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.0.prepare(sample_rate, channels);
    }
    fn process(&mut self, buffer: &mut [f32]) {
        self.0.process(buffer);
    }
    fn reset(&mut self) {
        self.0.reset();
    }
}

/// Fundamental-plus-overtones suppressor implemented on [`SpectralRepair`].
pub struct HarmonicSuppressor(SpectralRepair);

impl HarmonicSuppressor {
    /// Create a suppressor for `fundamental_hz` and its first `harmonics` partials.
    #[must_use]
    pub fn new(fundamental_hz: f32, harmonics: usize, bandwidth_q: f32, reduction_db: f32) -> Self {
        Self(SpectralRepair::harmonic(
            fundamental_hz,
            harmonics,
            bandwidth_q,
            reduction_db,
        ))
    }

    /// Change the fundamental while the processor is live.
    pub fn set_fundamental_hz(&mut self, fundamental_hz: f32) {
        self.0.set_fundamental_hz(fundamental_hz);
    }

    /// Change the overtone count while the processor is live.
    pub fn set_harmonics(&mut self, harmonics: usize) {
        self.0.set_harmonics(harmonics);
    }

    /// Change the attenuation while the processor is live.
    pub fn set_reduction_db(&mut self, db: f32) {
        self.0.set_reduction_db(db);
    }

    /// Change the overtone bandwidth Q while the processor is live.
    pub fn set_bandwidth_q(&mut self, q: f32) {
        self.0.set_bandwidth_q(q);
    }

    /// Enable or disable the suppressor.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.0.set_enabled(enabled);
    }
}

impl Effect for HarmonicSuppressor {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.0.prepare(sample_rate, channels);
    }
    fn process(&mut self, buffer: &mut [f32]) {
        self.0.process(buffer);
    }
    fn reset(&mut self) {
        self.0.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn tone(frequency_hz: f32, seconds: f32) -> Vec<f32> {
        (0..(RATE as f32 * seconds) as usize)
            .map(|i| 0.25 * (2.0 * PI * frequency_hz * i as f32 / RATE as f32).sin())
            .collect()
    }

    /// Goertzel-style correlation energy at a known frequency.
    fn band_energy(samples: &[f32], frequency_hz: f32) -> f64 {
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for (i, sample) in samples.iter().enumerate() {
            let phase = 2.0 * PI * frequency_hz * i as f32 / RATE as f32;
            re += f64::from(*sample) * f64::from(phase.cos());
            im += f64::from(*sample) * f64::from(phase.sin());
        }
        (re * re + im * im) / samples.len() as f64
    }

    fn process(effect: &mut impl Effect, input: &[f32]) -> Vec<f32> {
        // Prime the overlap-add pipeline; the first FFT window is latency.
        let prime = vec![0.0; SPECTRAL_FFT_SIZE];
        let mut sink = prime.clone();
        effect.process(&mut sink);
        let mut output = input.to_vec();
        effect.process(&mut output);
        output
    }

    #[test]
    fn notch_attenuates_selected_frequency_and_preserves_other_band() {
        let input: Vec<f32> = tone(1_000.0, 0.2)
            .into_iter()
            .zip(tone(3_000.0, 0.2))
            .map(|(a, b)| a + b)
            .collect();
        let before_1k = band_energy(&input, 1_000.0);
        let before_3k = band_energy(&input, 3_000.0);

        let mut notch = SpectralNotchFilter::new(1_000.0, 4.0, 40.0);
        notch.prepare(RATE, 1);
        let output = process(&mut notch, &input);
        let after_1k = band_energy(&output, 1_000.0);
        let after_3k = band_energy(&output, 3_000.0);

        assert!(
            after_1k < before_1k * 0.1,
            "1 kHz band should drop >=20 dB, got {after_1k} vs {before_1k}"
        );
        assert!(
            after_3k > before_3k * 0.5,
            "3 kHz band should survive, got {after_3k} vs {before_3k}"
        );
    }

    #[test]
    fn harmonic_suppressor_attenuates_fundamental_and_overtones() {
        let input: Vec<f32> = tone(220.0, 0.2)
            .into_iter()
            .zip(tone(440.0, 0.2))
            .map(|(a, b)| a + b)
            .collect();
        let before_f0 = band_energy(&input, 220.0);
        let before_h2 = band_energy(&input, 440.0);

        let mut suppressor = HarmonicSuppressor::new(220.0, 4, 3.0, 40.0);
        suppressor.prepare(RATE, 1);
        let output = process(&mut suppressor, &input);

        assert!(band_energy(&output, 220.0) < before_f0 * 0.1);
        assert!(band_energy(&output, 440.0) < before_h2 * 0.1);
    }

    #[test]
    fn disabled_repair_is_exact_passthrough() {
        let input = tone(1_000.0, 0.1);
        let mut notch = SpectralNotchFilter::new(1_000.0, 4.0, 40.0);
        notch.prepare(RATE, 1);
        notch.set_enabled(false);
        let output = process(&mut notch, &input);
        assert_eq!(output, input);
    }

    #[test]
    fn live_parameter_updates_change_the_mask() {
        let input = tone(1_000.0, 0.15);
        let before = band_energy(&input, 1_000.0);
        let mut notch = SpectralNotchFilter::new(3_000.0, 4.0, 40.0);
        notch.prepare(RATE, 1);
        let untouched = process(&mut notch, &input);
        assert!(band_energy(&untouched, 1_000.0) > before * 0.5);

        notch.set_center_hz(1_000.0);
        let reduced = process(&mut notch, &input);
        assert!(band_energy(&reduced, 1_000.0) < before * 0.1);
    }
}
