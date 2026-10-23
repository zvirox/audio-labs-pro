//! cpal output devices, playback and device discovery.
//!
//! The output callback only calls [`Mixer::render_into`] and updates atomics. It
//! never allocates, locks or logs. Device probing happens on the caller's thread.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use crate::TrackMeterBank;
use crate::error::EngineError;
use crate::mixer::Mixer;
use crate::scene::MixScene;

/// Direction of an audio device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceDirection {
    /// Playback device.
    Output,
    /// Capture device.
    Input,
}

/// A usable device format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Human-readable device name.
    pub name: String,
    /// Whether this is the default device for its direction.
    pub is_default: bool,
    /// Playback or capture.
    pub direction: DeviceDirection,
    /// Channels in the default configuration.
    pub channels: u16,
    /// Sample rate in the default configuration.
    pub sample_rate: u32,
    /// Sample format name (`f32`, `i16`, ...).
    pub sample_format: String,
}

/// Result of one playback run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaybackReport {
    /// Device callbacks observed.
    pub callbacks: u64,
    /// Frames written to the device.
    pub frames: u64,
    /// Wall-clock duration.
    pub elapsed_secs: f64,
    /// Whether the scene reached its end.
    pub finished: bool,
    /// Whether the backend reported a stream error.
    pub stream_error: bool,
}

/// Enumerate output and input devices with their default formats.
pub fn list_devices() -> Result<Vec<DeviceInfo>, EngineError> {
    let host = cpal::default_host();
    let default_output = host
        .default_output_device()
        .and_then(|device| device.description().ok())
        .map(|description| description.name().to_owned());
    let default_input = host
        .default_input_device()
        .and_then(|device| device.description().ok())
        .map(|description| description.name().to_owned());

    let mut devices = Vec::new();
    if let Ok(outputs) = host.output_devices() {
        for device in outputs {
            let Ok(description) = device.description() else {
                continue;
            };
            let name = description.name().to_owned();
            let Ok(config) = device.default_output_config() else {
                continue;
            };
            devices.push(DeviceInfo {
                is_default: default_output.as_deref() == Some(name.as_str()),
                name,
                direction: DeviceDirection::Output,
                channels: config.channels(),
                sample_rate: config.sample_rate(),
                sample_format: format!("{:?}", config.sample_format()),
            });
        }
    }
    if let Ok(inputs) = host.input_devices() {
        for device in inputs {
            let Ok(description) = device.description() else {
                continue;
            };
            let name = description.name().to_owned();
            let Ok(config) = device.default_input_config() else {
                continue;
            };
            devices.push(DeviceInfo {
                is_default: default_input.as_deref() == Some(name.as_str()),
                name,
                direction: DeviceDirection::Input,
                channels: config.channels(),
                sample_rate: config.sample_rate(),
                sample_format: format!("{:?}", config.sample_format()),
            });
        }
    }
    Ok(devices)
}

#[derive(Debug, Default)]
struct PlaybackStats {
    callbacks: AtomicU64,
    frames: AtomicU64,
    finished: AtomicBool,
    stream_error: AtomicBool,
}

/// Shared live playback state read by the UI.
#[derive(Clone, Default)]
pub struct PlaybackMeter {
    pub callbacks: std::sync::Arc<std::sync::atomic::AtomicU64>,
    pub frames: std::sync::Arc<std::sync::atomic::AtomicU64>,
    pub finished: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub stream_error: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub master_peak: std::sync::Arc<std::sync::atomic::AtomicU32>,
    pub master_rms: std::sync::Arc<std::sync::atomic::AtomicU32>,
    pub track_meter_bank: std::sync::Arc<std::sync::Mutex<Option<Arc<TrackMeterBank>>>>,
}

/// Handle to a background playback started by the UI.
pub struct PlaybackHandle {
    state: std::sync::Arc<PlaybackMeter>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<Result<PlaybackReport, EngineError>>>,
}

impl PlaybackHandle {
    #[must_use]
    pub fn position_frames(&self) -> u64 {
        self.state.frames.load(std::sync::atomic::Ordering::Relaxed)
    }
    #[must_use]
    pub fn master_peak(&self) -> f32 {
        f32::from_bits(
            self.state
                .master_peak
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }
    #[must_use]
    pub fn master_rms(&self) -> f32 {
        f32::from_bits(
            self.state
                .master_rms
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }
    #[must_use]
    pub fn finished(&self) -> bool {
        self.state
            .finished
            .load(std::sync::atomic::Ordering::Relaxed)
            || self
                .state
                .stream_error
                .load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn stop(&self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
    #[must_use]
    pub fn track_meters(&self) -> Option<Arc<crate::mixer::TrackMeterBank>> {
        self.state.track_meter_bank.lock().ok()?.clone()
    }
}
impl Drop for PlaybackHandle {
    fn drop(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Play a prepared scene on the default output device.
///
/// `max_seconds` caps wall-clock time; `None` plays the whole scene. The scene is
/// resampled to the device rate and reinterpreted at the device channel count
/// before the stream starts, never inside the callback.
pub fn play_scene(
    scene: &MixScene,
    max_seconds: Option<f64>,
) -> Result<PlaybackReport, EngineError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| EngineError::NoDevice("no default output device".into()))?;
    let supported = device
        .default_output_config()
        .map_err(|error| EngineError::Stream(format!("default output config: {error}")))?;
    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.config();
    let device_rate = config.sample_rate;
    let device_channels = config.channels;

    let prepared = if scene.sample_rate() == device_rate {
        scene.clone()
    } else {
        scene.resampled(device_rate)?
    }
    .with_channels(device_channels);

    let stats = Arc::new(PlaybackStats::default());
    let stats_cb = stats.clone();
    let mut mixer = Mixer::new(prepared);
    let error_flag = stats.clone();

    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream::<f32, _, _>(
            config,
            move |data, _| {
                let frames = mixer.render_into(data);
                stats_cb.callbacks.fetch_add(1, Ordering::Relaxed);
                stats_cb.frames.fetch_add(frames as u64, Ordering::Relaxed);
                if mixer.finished() {
                    stats_cb.finished.store(true, Ordering::Relaxed);
                }
            },
            move |_error| {
                error_flag.stream_error.store(true, Ordering::Relaxed);
            },
            None,
        ),
        SampleFormat::I16 => {
            let scratch_len = 16_384 * usize::from(device_channels);
            let mut scratch = vec![0.0f32; scratch_len];
            device.build_output_stream::<i16, _, _>(
                config,
                move |data, _| {
                    let mut offset = 0;
                    let mut frames = 0;
                    while offset < data.len() {
                        let take = (data.len() - offset).min(scratch.len());
                        let block = &mut scratch[..take];
                        frames += mixer.render_into(block);
                        for (destination, source) in
                            data[offset..offset + take].iter_mut().zip(block.iter())
                        {
                            *destination = float_to_i16(*source);
                        }
                        offset += take;
                    }
                    stats_cb.callbacks.fetch_add(1, Ordering::Relaxed);
                    stats_cb.frames.fetch_add(frames as u64, Ordering::Relaxed);
                    if mixer.finished() {
                        stats_cb.finished.store(true, Ordering::Relaxed);
                    }
                },
                move |_error| {
                    error_flag.stream_error.store(true, Ordering::Relaxed);
                },
                None,
            )
        }
        other => {
            return Err(EngineError::UnsupportedFormat(format!(
                "output sample format {other:?}"
            )));
        }
    }
    .map_err(|error| EngineError::Stream(format!("build output stream: {error}")))?;

    stream
        .play()
        .map_err(|error| EngineError::Stream(format!("start playback: {error}")))?;

    let limit = max_seconds.map_or_else(
        || Duration::from_secs_f64(scene.duration_secs() + 0.25),
        |seconds| Duration::from_secs_f64(seconds.max(0.001)),
    );
    let started = Instant::now();
    while started.elapsed() < limit && !stats.finished.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(stream);

    Ok(PlaybackReport {
        callbacks: stats.callbacks.load(Ordering::Relaxed),
        frames: stats.frames.load(Ordering::Relaxed),
        elapsed_secs: started.elapsed().as_secs_f64(),
        finished: stats.finished.load(Ordering::Relaxed),
        stream_error: stats.stream_error.load(Ordering::Relaxed),
    })
}

pub fn play_scene_live(
    scene: &MixScene,
    max_seconds: Option<f64>,
) -> Result<PlaybackHandle, EngineError> {
    let reporter = Arc::new(PlaybackMeter::default());
    let thread_reporter = reporter.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_thread = cancel.clone();
    let scene = scene.clone();
    let thread = std::thread::spawn(move || {
        play_scene_with_reporter(
            &scene,
            max_seconds,
            Some(thread_reporter),
            Some(cancel_thread),
        )
    });
    Ok(PlaybackHandle {
        state: reporter,
        cancel,
        thread: Some(thread),
    })
}

pub fn play_scene_with_reporter(
    scene: &MixScene,
    max_seconds: Option<f64>,
    reporter: Option<Arc<PlaybackMeter>>,
    cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
) -> Result<PlaybackReport, EngineError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| EngineError::NoDevice("no default output device".into()))?;
    let supported = device
        .default_output_config()
        .map_err(|error| EngineError::Stream(format!("default output config: {error}")))?;
    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.config();
    let device_rate = config.sample_rate;
    let device_channels = config.channels;

    let prepared = if scene.sample_rate() == device_rate {
        scene.clone()
    } else {
        scene.resampled(device_rate)?
    }
    .with_channels(device_channels);

    let stats = reporter.clone().unwrap_or_default();
    let stats_cb = stats.clone();
    let mut mixer = Mixer::new(prepared);
    if let Some(reporter) = &reporter {
        *reporter.track_meter_bank.lock().expect("track meter bank") = Some(mixer.track_meters());
    }
    let error_flag = stats.clone();

    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream::<f32, _, _>(
            config,
            move |data, _| {
                let frames = mixer.render_into(data);
                stats_cb.callbacks.fetch_add(1, Ordering::Relaxed);
                stats_cb.frames.fetch_add(frames as u64, Ordering::Relaxed);
                if mixer.finished() {
                    stats_cb.finished.store(true, Ordering::Relaxed);
                }
                let mut peak = 0.0f32;
                let mut sum = 0.0f64;
                for sample in &mut *data {
                    peak = peak.max(sample.abs());
                    sum += f64::from(*sample) * f64::from(*sample);
                }
                stats_cb
                    .master_peak
                    .store(peak.to_bits(), Ordering::Relaxed);
                stats_cb.master_rms.store(
                    (if data.is_empty() {
                        0.0
                    } else {
                        (sum / data.len() as f64).sqrt()
                    } as f32)
                        .to_bits(),
                    Ordering::Relaxed,
                );
            },
            move |_error| {
                error_flag.stream_error.store(true, Ordering::Relaxed);
            },
            None,
        ),
        SampleFormat::I16 => {
            let scratch_len = 16_384 * usize::from(device_channels);
            let mut scratch = vec![0.0f32; scratch_len];
            device.build_output_stream::<i16, _, _>(
                config,
                move |data, _| {
                    let mut offset = 0;
                    let mut frames = 0;
                    while offset < data.len() {
                        let take = (data.len() - offset).min(scratch.len());
                        let block = &mut scratch[..take];
                        frames += mixer.render_into(block);
                        let mut peak = 0.0f32;
                        let mut sum = 0.0f64;
                        for source in &block[..take] {
                            peak = peak.max(source.abs());
                            sum += f64::from(*source) * f64::from(*source);
                        }
                        stats_cb
                            .master_peak
                            .store(peak.to_bits(), Ordering::Relaxed);
                        stats_cb.master_rms.store(
                            (if take == 0 {
                                0.0
                            } else {
                                (sum / take as f64).sqrt()
                            } as f32)
                                .to_bits(),
                            Ordering::Relaxed,
                        );
                        for (destination, source) in
                            data[offset..offset + take].iter_mut().zip(block.iter())
                        {
                            *destination = float_to_i16(*source);
                        }
                        offset += take;
                    }
                    stats_cb.callbacks.fetch_add(1, Ordering::Relaxed);
                    stats_cb.frames.fetch_add(frames as u64, Ordering::Relaxed);
                    if mixer.finished() {
                        stats_cb.finished.store(true, Ordering::Relaxed);
                    }
                },
                move |_error| {
                    error_flag.stream_error.store(true, Ordering::Relaxed);
                },
                None,
            )
        }
        other => {
            return Err(EngineError::UnsupportedFormat(format!(
                "output sample format {other:?}"
            )));
        }
    }
    .map_err(|error| EngineError::Stream(format!("build output stream: {error}")))?;

    stream
        .play()
        .map_err(|error| EngineError::Stream(format!("start playback: {error}")))?;

    let started = Instant::now();
    let limit = max_seconds.map_or_else(
        || Duration::from_secs_f64(scene.duration_secs() + 0.25),
        |seconds| Duration::from_secs_f64(seconds.max(0.001)),
    );
    while !cancel
        .as_ref()
        .is_some_and(|flag| flag.load(Ordering::Relaxed))
        && started.elapsed() < limit
        && !stats.finished.load(Ordering::Relaxed)
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(stream);

    Ok(PlaybackReport {
        callbacks: stats.callbacks.load(Ordering::Relaxed),
        frames: stats.frames.load(Ordering::Relaxed),
        elapsed_secs: started.elapsed().as_secs_f64(),
        finished: stats.finished.load(Ordering::Relaxed),
        stream_error: stats.stream_error.load(Ordering::Relaxed),
    })
}

fn float_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16
}
