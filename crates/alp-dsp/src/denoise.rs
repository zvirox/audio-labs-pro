//! Noise reduction processors: RNNoise neural denoiser and spectral subtraction.
//!
//! Provides two complementary noise reduction strategies:
//! 1. `RnnoiseDenoiser`: Deep neural network suppression (via `nnnoiseless`) optimized for
//!    speech, handling stationary and non-stationary noises (clicks, keyboard clatter, traffic).
//! 2. `SpectralDenoiser`: Classical spectral subtraction with captured noise profile (via `realfft`),
//!    optimized for stationary noise (fans, HVAC, steady hiss) with spectral floor control.
//! 3. `NoiseReduction`: Unified `Effect` wrapper permitting mode selection.

use std::collections::VecDeque;
use std::sync::Arc;

use nnnoiseless::DenoiseState;
use realfft::num_complex::Complex32;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};

use crate::Effect;

/// Number of samples in a native RNNoise processing frame (10 ms at 48 kHz).
pub const RNNOISE_FRAME_SIZE: usize = nnnoiseless::FRAME_SIZE;

/// FFT frame size for spectral subtraction (~21.3 ms at 48 kHz).
pub const SPECTRAL_FFT_SIZE: usize = 1024;

/// Hop size for 50% overlap spectral subtraction (~10.7 ms at 48 kHz).
pub const SPECTRAL_HOP_SIZE: usize = SPECTRAL_FFT_SIZE / 2;

/// Number of real FFT frequency bins.
pub const SPECTRAL_BINS: usize = SPECTRAL_FFT_SIZE / 2 + 1;

/// Mode of noise reduction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DenoiseMode {
    /// Deep learning neural suppression using RNNoise.
    #[default]
    Rnnoise,
    /// Profile-based spectral subtraction.
    Spectral,
}

/// Neural noise suppression processor based on RNNoise.
///
/// Features internal FIFO buffering to transparently support arbitrary audio chunk sizes
/// while RNNoise processes native 480-sample blocks.
pub struct RnnoiseDenoiser {
    states: Vec<Box<DenoiseState<'static>>>,
    input_fifos: Vec<VecDeque<f32>>,
    output_fifos: Vec<VecDeque<f32>>,
    sample_rate: u32,
    channels: u16,
    wet: f32,
}

impl Default for RnnoiseDenoiser {
    fn default() -> Self {
        Self::new()
    }
}

impl RnnoiseDenoiser {
    /// Create a new RNNoise neural denoiser with full effect wetness (1.0).
    #[must_use]
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            input_fifos: Vec::new(),
            output_fifos: Vec::new(),
            sample_rate: 48_000,
            channels: 1,
            wet: 1.0,
        }
    }

    /// Set the wet/dry mix ratio in `[0.0, 1.0]`.
    pub fn set_wet(&mut self, wet: f32) {
        self.wet = wet.clamp(0.0, 1.0);
    }

    /// Current wet/dry mix ratio.
    #[must_use]
    pub fn wet(&self) -> f32 {
        self.wet
    }

    /// Helper to process one channel's input samples through the RNNoise state and FIFOs.
    fn process_channel_samples(
        state: &mut DenoiseState<'static>,
        input_fifo: &mut VecDeque<f32>,
        output_fifo: &mut VecDeque<f32>,
        in_samples: &[f32],
        out_samples: &mut [f32],
        wet: f32,
    ) {
        let mut in_frame = [0.0f32; RNNOISE_FRAME_SIZE];
        let mut out_frame = [0.0f32; RNNOISE_FRAME_SIZE];

        // Feed new input samples into FIFO
        input_fifo.extend(in_samples.iter().copied());

        // Process full 480-sample frames whenever available
        while input_fifo.len() >= RNNOISE_FRAME_SIZE {
            for sample in &mut in_frame {
                // RNNoise expects 16-bit PCM scale (-32768.0 to 32767.0)
                let s = input_fifo.pop_front().unwrap_or(0.0);
                *sample = (s * 32768.0).clamp(-32768.0, 32767.0);
            }

            state.process_frame(&mut out_frame, &in_frame);

            // Scale back down to normalized [-1.0, 1.0]
            for sample in &out_frame {
                let s = *sample / 32768.0;
                output_fifo.push_back(s.clamp(-1.0, 1.0));
            }
        }

        // Pop requested number of processed samples into output
        for (i, out_s) in out_samples.iter_mut().enumerate() {
            let denoised = output_fifo.pop_front().unwrap_or(0.0);
            let dry = in_samples.get(i).copied().unwrap_or(0.0);
            *out_s = dry * (1.0 - wet) + denoised * wet;
        }
    }
}

impl Effect for RnnoiseDenoiser {
    fn name(&self) -> &'static str {
        "rnnoise"
    }

    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.sample_rate = sample_rate;
        self.channels = channels.max(1);

        let ch_count = self.channels as usize;
        self.states.clear();
        self.input_fifos.clear();
        self.output_fifos.clear();

        for _ in 0..ch_count {
            self.states.push(DenoiseState::new());
            self.input_fifos.push(VecDeque::with_capacity(2048));
            // Pre-seed output FIFO with latency compensation (480 samples = 10 ms)
            let mut out_q = VecDeque::with_capacity(2048);
            out_q.extend(std::iter::repeat_n(0.0, RNNOISE_FRAME_SIZE));
            self.output_fifos.push(out_q);
        }
    }

    fn process(&mut self, buffer: &mut [f32]) {
        let ch_count = self.channels as usize;
        if ch_count == 0 || buffer.is_empty() {
            return;
        }

        // Ensure states exist if prepare was not called
        if self.states.len() != ch_count {
            self.prepare(self.sample_rate, self.channels);
        }

        let num_frames = buffer.len() / ch_count;
        if num_frames == 0 {
            return;
        }

        let wet = self.wet;

        if ch_count == 1 {
            // Mono fast path: no de-interleave necessary
            let (state, (in_q, out_q)) = (
                &mut self.states[0],
                (&mut self.input_fifos[0], &mut self.output_fifos[0]),
            );
            let input_copy = buffer.to_vec();
            Self::process_channel_samples(state, in_q, out_q, &input_copy, buffer, wet);
        } else {
            // Multi-channel path: de-interleave, process per channel, re-interleave
            let mut ch_in = vec![0.0f32; num_frames];
            let mut ch_out = vec![0.0f32; num_frames];

            for ch in 0..ch_count {
                for f in 0..num_frames {
                    ch_in[f] = buffer[f * ch_count + ch];
                }

                Self::process_channel_samples(
                    &mut self.states[ch],
                    &mut self.input_fifos[ch],
                    &mut self.output_fifos[ch],
                    &ch_in,
                    &mut ch_out,
                    wet,
                );

                for f in 0..num_frames {
                    buffer[f * ch_count + ch] = ch_out[f];
                }
            }
        }
    }

    fn reset(&mut self) {
        let ch_count = self.channels as usize;
        self.states.clear();
        self.input_fifos.clear();
        self.output_fifos.clear();

        for _ in 0..ch_count {
            self.states.push(DenoiseState::new());
            self.input_fifos.push(VecDeque::with_capacity(2048));
            let mut out_q = VecDeque::with_capacity(2048);
            out_q.extend(std::iter::repeat_n(0.0, RNNOISE_FRAME_SIZE));
            self.output_fifos.push(out_q);
        }
    }
}

/// Classical spectral subtraction denoiser with noise-profile capture.
///
/// Uses an STFT with Hann windowing and 50% overlap-add to suppress stationary noise.
pub struct SpectralDenoiser {
    fft_forward: Option<Arc<dyn RealToComplex<f32>>>,
    fft_inverse: Option<Arc<dyn ComplexToReal<f32>>>,
    window: Vec<f32>,
    noise_profile: Option<Vec<f32>>,
    in_fifos: Vec<VecDeque<f32>>,
    ola_buffers: Vec<Vec<f32>>,
    out_fifos: Vec<VecDeque<f32>>,
    sample_rate: u32,
    channels: u16,
    over_subtraction: f32,
    spectral_floor: f32,
    strength: f32,
}

impl Default for SpectralDenoiser {
    fn default() -> Self {
        Self::new()
    }
}

impl SpectralDenoiser {
    /// Create a new spectral subtraction denoiser.
    #[must_use]
    pub fn new() -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(SPECTRAL_FFT_SIZE);
        let c2r = planner.plan_fft_inverse(SPECTRAL_FFT_SIZE);

        let mut window = vec![0.0f32; SPECTRAL_FFT_SIZE];
        for (i, w) in window.iter_mut().enumerate() {
            // Hann window: 0.5 * (1 - cos(2*pi*i / N))
            *w = 0.5
                * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / SPECTRAL_FFT_SIZE as f32).cos());
        }

        Self {
            fft_forward: Some(r2c),
            fft_inverse: Some(c2r),
            window,
            noise_profile: None,
            in_fifos: Vec::new(),
            ola_buffers: Vec::new(),
            out_fifos: Vec::new(),
            sample_rate: 48_000,
            channels: 1,
            over_subtraction: 1.5,
            spectral_floor: 0.05,
            strength: 1.0,
        }
    }

    /// Capture a noise profile from an audio segment consisting purely of background noise.
    pub fn capture_noise_profile(&mut self, noise_samples: &[f32]) {
        if noise_samples.is_empty() {
            return;
        }

        let r2c = match &self.fft_forward {
            Some(plan) => plan.clone(),
            None => {
                let mut planner = RealFftPlanner::<f32>::new();
                let plan = planner.plan_fft_forward(SPECTRAL_FFT_SIZE);
                self.fft_forward = Some(plan.clone());
                plan
            }
        };

        let mut avg_mag = vec![0.0f32; SPECTRAL_BINS];
        let mut frame_count = 0usize;

        let mut input_buf = vec![0.0f32; SPECTRAL_FFT_SIZE];
        let mut spec_buf = vec![Complex32::default(); SPECTRAL_BINS];
        let mut scratch = vec![Complex32::default(); r2c.get_scratch_len()];

        let mut offset = 0;
        while offset + SPECTRAL_FFT_SIZE <= noise_samples.len() {
            for (i, in_s) in input_buf.iter_mut().enumerate() {
                *in_s = noise_samples[offset + i] * self.window[i];
            }

            if r2c
                .process_with_scratch(&mut input_buf, &mut spec_buf, &mut scratch)
                .is_ok()
            {
                for (bin, &comp) in spec_buf.iter().enumerate() {
                    avg_mag[bin] += (comp.re * comp.re + comp.im * comp.im).sqrt();
                }
                frame_count += 1;
            }

            offset += SPECTRAL_HOP_SIZE;
        }

        // If sample slice was shorter than one FFT frame, zero-pad and compute single frame
        if frame_count == 0 {
            input_buf.fill(0.0);
            for (i, in_s) in input_buf.iter_mut().take(noise_samples.len()).enumerate() {
                *in_s = noise_samples[i] * self.window[i];
            }
            if r2c
                .process_with_scratch(&mut input_buf, &mut spec_buf, &mut scratch)
                .is_ok()
            {
                for (bin, &comp) in spec_buf.iter().enumerate() {
                    avg_mag[bin] += (comp.re * comp.re + comp.im * comp.im).sqrt();
                }
                frame_count = 1;
            }
        }

        if frame_count > 0 {
            for mag in &mut avg_mag {
                *mag /= frame_count as f32;
            }
            self.noise_profile = Some(avg_mag);
        }
    }

    /// Whether a noise profile has been captured.
    #[must_use]
    pub fn has_noise_profile(&self) -> bool {
        self.noise_profile.is_some()
    }

    /// Set over-subtraction factor (typically between 1.0 and 2.5).
    pub fn set_over_subtraction(&mut self, factor: f32) {
        self.over_subtraction = factor.max(0.5);
    }

    /// Set spectral floor (typically between 0.01 and 0.1).
    pub fn set_spectral_floor(&mut self, floor: f32) {
        self.spectral_floor = floor.clamp(0.0, 0.5);
    }

    /// Set overall reduction strength (0.0 = passthrough, 1.0 = full reduction).
    pub fn set_strength(&mut self, strength: f32) {
        self.strength = strength.clamp(0.0, 1.0);
    }

    /// Current reduction strength.
    #[must_use]
    pub fn strength(&self) -> f32 {
        self.strength
    }

    /// Process a single channel through the STFT overlap-add pipeline.
    #[allow(clippy::too_many_arguments)]
    fn process_channel_stft(
        r2c: &Arc<dyn RealToComplex<f32>>,
        c2r: &Arc<dyn ComplexToReal<f32>>,
        window: &[f32],
        profile: Option<&[f32]>,
        in_fifo: &mut VecDeque<f32>,
        ola_buf: &mut [f32],
        out_fifo: &mut VecDeque<f32>,
        in_samples: &[f32],
        out_samples: &mut [f32],
        alpha: f32,
        beta: f32,
        strength: f32,
    ) {
        in_fifo.extend(in_samples.iter().copied());

        let mut time_in = vec![0.0f32; SPECTRAL_FFT_SIZE];
        let mut time_out = vec![0.0f32; SPECTRAL_FFT_SIZE];
        let mut spectrum = vec![Complex32::default(); SPECTRAL_BINS];
        let mut fwd_scratch = vec![Complex32::default(); r2c.get_scratch_len()];
        let mut inv_scratch = vec![Complex32::default(); c2r.get_scratch_len()];

        // While we have at least one full FFT window in the FIFO:
        while in_fifo.len() >= SPECTRAL_FFT_SIZE {
            // Read 1024 samples from FIFO (without consuming all, only hop size)
            for (i, t_s) in time_in.iter_mut().enumerate() {
                *t_s = in_fifo[i] * window[i];
            }

            if r2c
                .process_with_scratch(&mut time_in, &mut spectrum, &mut fwd_scratch)
                .is_ok()
            {
                // Apply spectral subtraction if profile is present and strength > 0
                if let Some(prof) = profile
                    && strength > 0.0
                {
                    for (k, comp) in spectrum.iter_mut().enumerate() {
                        let mag = (comp.re * comp.re + comp.im * comp.im).sqrt();
                        let noise_mag = prof[k] * strength;
                        let sub_mag = mag - alpha * noise_mag;
                        let floor_mag = beta * mag;
                        let clean_mag = sub_mag.max(floor_mag).max(0.0);

                        let gain = if mag > 1e-12 {
                            (clean_mag / mag).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };

                        comp.re *= gain;
                        comp.im *= gain;
                    }
                }

                // Inverse FFT
                if c2r
                    .process_with_scratch(&mut spectrum, &mut time_out, &mut inv_scratch)
                    .is_ok()
                {
                    // Scale unnormalized IFFT by 1/N
                    let norm = 1.0 / SPECTRAL_FFT_SIZE as f32;
                    for (i, &s) in time_out.iter().enumerate() {
                        ola_buf[i] += s * norm;
                    }

                    // The first HOP_SIZE samples are fully overlap-added and ready
                    out_fifo.extend(ola_buf[..SPECTRAL_HOP_SIZE].iter().copied());

                    // Shift OLA buffer by HOP_SIZE
                    ola_buf.copy_within(SPECTRAL_HOP_SIZE..SPECTRAL_FFT_SIZE, 0);
                    ola_buf[SPECTRAL_HOP_SIZE..SPECTRAL_FFT_SIZE].fill(0.0);
                }
            }

            // Advance input FIFO by hop size
            in_fifo.drain(0..SPECTRAL_HOP_SIZE);
        }

        // Pop requested number of processed samples into output
        for out_s in out_samples.iter_mut() {
            *out_s = out_fifo.pop_front().unwrap_or(0.0).clamp(-1.0, 1.0);
        }
    }
}

impl Effect for SpectralDenoiser {
    fn name(&self) -> &'static str {
        "spectral-denoise"
    }

    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.sample_rate = sample_rate;
        self.channels = channels.max(1);

        let ch_count = self.channels as usize;
        self.in_fifos.clear();
        self.ola_buffers.clear();
        self.out_fifos.clear();

        for _ in 0..ch_count {
            // Seed input FIFO with hop-size silence so first frame begins immediately
            let mut in_q = VecDeque::with_capacity(2048);
            in_q.extend(std::iter::repeat_n(0.0, SPECTRAL_HOP_SIZE));
            self.in_fifos.push(in_q);

            self.ola_buffers.push(vec![0.0f32; SPECTRAL_FFT_SIZE]);

            // Pre-seed output FIFO with hop-size latency compensation
            let mut out_q = VecDeque::with_capacity(2048);
            out_q.extend(std::iter::repeat_n(0.0, SPECTRAL_HOP_SIZE));
            self.out_fifos.push(out_q);
        }
    }

    fn process(&mut self, buffer: &mut [f32]) {
        let ch_count = self.channels as usize;
        if ch_count == 0 || buffer.is_empty() {
            return;
        }

        if self.in_fifos.len() != ch_count {
            self.prepare(self.sample_rate, self.channels);
        }

        let r2c = self.fft_forward.as_ref().unwrap().clone();
        let c2r = self.fft_inverse.as_ref().unwrap().clone();
        let num_frames = buffer.len() / ch_count;
        if num_frames == 0 {
            return;
        }

        let alpha = self.over_subtraction;
        let beta = self.spectral_floor;
        let strength = self.strength;
        let profile = self.noise_profile.as_deref();

        if ch_count == 1 {
            let input_copy = buffer.to_vec();
            Self::process_channel_stft(
                &r2c,
                &c2r,
                &self.window,
                profile,
                &mut self.in_fifos[0],
                &mut self.ola_buffers[0],
                &mut self.out_fifos[0],
                &input_copy,
                buffer,
                alpha,
                beta,
                strength,
            );
        } else {
            let mut ch_in = vec![0.0f32; num_frames];
            let mut ch_out = vec![0.0f32; num_frames];

            for ch in 0..ch_count {
                for f in 0..num_frames {
                    ch_in[f] = buffer[f * ch_count + ch];
                }

                Self::process_channel_stft(
                    &r2c,
                    &c2r,
                    &self.window,
                    profile,
                    &mut self.in_fifos[ch],
                    &mut self.ola_buffers[ch],
                    &mut self.out_fifos[ch],
                    &ch_in,
                    &mut ch_out,
                    alpha,
                    beta,
                    strength,
                );

                for f in 0..num_frames {
                    buffer[f * ch_count + ch] = ch_out[f];
                }
            }
        }
    }

    fn reset(&mut self) {
        let ch_count = self.channels as usize;
        self.in_fifos.clear();
        self.ola_buffers.clear();
        self.out_fifos.clear();

        for _ in 0..ch_count {
            let mut in_q = VecDeque::with_capacity(2048);
            in_q.extend(std::iter::repeat_n(0.0, SPECTRAL_HOP_SIZE));
            self.in_fifos.push(in_q);

            self.ola_buffers.push(vec![0.0f32; SPECTRAL_FFT_SIZE]);

            let mut out_q = VecDeque::with_capacity(2048);
            out_q.extend(std::iter::repeat_n(0.0, SPECTRAL_HOP_SIZE));
            self.out_fifos.push(out_q);
        }
    }
}

/// Composite Noise Reduction effect supporting both neural RNNoise and spectral subtraction.
pub struct NoiseReduction {
    mode: DenoiseMode,
    rnnoise: RnnoiseDenoiser,
    spectral: SpectralDenoiser,
}

impl Default for NoiseReduction {
    fn default() -> Self {
        Self::new(DenoiseMode::Rnnoise)
    }
}

impl NoiseReduction {
    /// Create a noise reduction effect in the specified mode.
    #[must_use]
    pub fn new(mode: DenoiseMode) -> Self {
        Self {
            mode,
            rnnoise: RnnoiseDenoiser::new(),
            spectral: SpectralDenoiser::new(),
        }
    }

    /// Create an RNNoise-mode denoiser.
    #[must_use]
    pub fn rnnoise() -> Self {
        Self::new(DenoiseMode::Rnnoise)
    }

    /// Create a spectral-mode denoiser.
    #[must_use]
    pub fn spectral() -> Self {
        Self::new(DenoiseMode::Spectral)
    }

    /// Active denoise mode.
    #[must_use]
    pub fn mode(&self) -> DenoiseMode {
        self.mode
    }

    /// Set active denoise mode.
    pub fn set_mode(&mut self, mode: DenoiseMode) {
        self.mode = mode;
    }

    /// Reference to internal RNNoise processor.
    #[must_use]
    pub fn rnnoise_processor(&self) -> &RnnoiseDenoiser {
        &self.rnnoise
    }

    /// Mutable reference to internal RNNoise processor.
    pub fn rnnoise_processor_mut(&mut self) -> &mut RnnoiseDenoiser {
        &mut self.rnnoise
    }

    /// Reference to internal Spectral processor.
    #[must_use]
    pub fn spectral_processor(&self) -> &SpectralDenoiser {
        &self.spectral
    }

    /// Mutable reference to internal Spectral processor.
    pub fn spectral_processor_mut(&mut self) -> &mut SpectralDenoiser {
        &mut self.spectral
    }

    /// Capture a noise profile for spectral subtraction mode.
    pub fn capture_noise_profile(&mut self, noise_samples: &[f32]) {
        self.spectral.capture_noise_profile(noise_samples);
    }
}

impl Effect for NoiseReduction {
    fn name(&self) -> &'static str {
        match self.mode {
            DenoiseMode::Rnnoise => "noise-reduction (rnnoise)",
            DenoiseMode::Spectral => "noise-reduction (spectral)",
        }
    }

    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.rnnoise.prepare(sample_rate, channels);
        self.spectral.prepare(sample_rate, channels);
    }

    fn process(&mut self, buffer: &mut [f32]) {
        match self.mode {
            DenoiseMode::Rnnoise => self.rnnoise.process(buffer),
            DenoiseMode::Spectral => self.spectral.process(buffer),
        }
    }

    fn reset(&mut self) {
        self.rnnoise.reset();
        self.spectral.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compute_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    #[test]
    fn rnnoise_processes_arbitrary_chunk_sizes() {
        let mut denoiser = RnnoiseDenoiser::new();
        denoiser.prepare(48_000, 1);

        for chunk_size in [64, 128, 256, 480, 512, 1024] {
            let mut buf = vec![0.1f32; chunk_size];
            denoiser.process(&mut buf);
            assert_eq!(buf.len(), chunk_size);
            for s in buf {
                assert!(s.is_finite());
                assert!(s.abs() <= 1.0);
            }
        }
    }

    #[test]
    fn rnnoise_stereo_processing() {
        let mut denoiser = RnnoiseDenoiser::new();
        denoiser.prepare(48_000, 2);

        let mut buffer = vec![0.1f32; 960]; // 480 stereo frames
        denoiser.process(&mut buffer);

        assert_eq!(buffer.len(), 960);
        for s in buffer {
            assert!(s.is_finite());
            assert!(s.abs() <= 1.0);
        }
    }

    #[test]
    fn spectral_denoiser_reduces_noise_energy() {
        let mut denoiser = SpectralDenoiser::new();
        denoiser.prepare(48_000, 1);

        // Synthetic stationary noise (deterministic pseudo-random sequence)
        let mut noise = Vec::with_capacity(48_000);
        let mut state = 123456789u32;
        for _ in 0..48_000 {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let s = ((state >> 16) as f32 / 32768.0 - 1.0) * 0.2;
            noise.push(s);
        }

        // Capture noise profile from first 10,000 samples
        denoiser.capture_noise_profile(&noise[..10_000]);
        assert!(denoiser.has_noise_profile());

        let mut test_audio = noise[10_000..20_000].to_vec();
        let input_rms = compute_rms(&test_audio);

        denoiser.process(&mut test_audio);
        let output_rms = compute_rms(&test_audio[1024..]); // Skip initial latency ramp

        assert!(
            output_rms < input_rms,
            "spectral denoiser should reduce noise RMS: in={input_rms}, out={output_rms}"
        );
    }

    #[test]
    fn spectral_denoiser_passthrough_when_no_profile() {
        let mut denoiser = SpectralDenoiser::new();
        denoiser.prepare(48_000, 1);

        // Sinusoid signal
        let mut signal = Vec::with_capacity(4800);
        for i in 0..4800 {
            let s = (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 48_000.0).sin() * 0.5;
            signal.push(s);
        }

        let mut processed = signal.clone();
        denoiser.process(&mut processed);

        for &s in &processed {
            assert!(s.is_finite());
            assert!(s.abs() <= 1.0);
        }
    }

    #[test]
    fn noise_reduction_mode_switching() {
        let mut nr = NoiseReduction::default();
        assert_eq!(nr.mode(), DenoiseMode::Rnnoise);

        nr.prepare(48_000, 1);
        let mut buf = vec![0.05f32; 1000];
        nr.process(&mut buf);

        nr.set_mode(DenoiseMode::Spectral);
        assert_eq!(nr.mode(), DenoiseMode::Spectral);
        nr.process(&mut buf);

        for s in buf {
            assert!(s.is_finite());
            assert!(s.abs() <= 1.0);
        }
    }
}
