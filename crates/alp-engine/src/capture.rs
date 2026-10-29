//! Input capture with device selection, metering, monitoring and crash-safe
//! chunking.
//!
//! The input callback converts samples, pushes them into an SPSC ring, updates
//! peak/RMS atomics and (optionally) feeds a second ring for monitoring. A
//! writer thread owns all disk I/O. It spools raw little-endian f32 samples and
//! syncs a JSON manifest at a fixed chunk boundary, so a forced kill leaves a
//! recoverable recording behind.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use alp_core::{commit_temp, create_temp_sibling, now_ms, write_atomic};
use alp_media::MediaError;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use rtrb::{Consumer, Producer, RingBuffer};
use serde::{Deserialize, Serialize};

use crate::error::EngineError;

const MANIFEST_VERSION: u32 = 1;
const DEFAULT_CHUNK_FRAMES: u64 = 480_000;
const WRITER_POLL: Duration = Duration::from_millis(1);

/// Result of one capture run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureReport {
    /// Frames written to the WAV file.
    pub frames: u64,
    /// Capture sample rate.
    pub sample_rate: u32,
    /// Capture channel count.
    pub channels: u16,
    /// Samples dropped because the ring was full.
    pub dropped: u64,
    /// Device callbacks observed.
    pub callbacks: u64,
    /// Wall-clock duration.
    pub elapsed_secs: f64,
    /// Whether the backend reported a stream error.
    pub stream_error: bool,
    /// Whether this report came from recovering an interrupted capture.
    pub recovered: bool,
}

/// Capture configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureConfig {
    /// Exact input device name, or the host default when `None`.
    pub device: Option<String>,
    /// Frames between manifest syncs. Clamped to at least one second.
    pub chunk_frames: u64,
    /// Copy input to the default output while recording.
    pub monitor: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            device: None,
            chunk_frames: DEFAULT_CHUNK_FRAMES,
            monitor: false,
        }
    }
}

/// Lock-free peak/RMS meter sampled by the UI thread.
#[derive(Debug, Clone)]
pub struct InputMeter {
    peak: Arc<AtomicU32>,
    rms: Arc<AtomicU32>,
    frames: Arc<AtomicU64>,
}

impl InputMeter {
    fn new() -> Self {
        Self {
            peak: Arc::new(AtomicU32::new(0)),
            rms: Arc::new(AtomicU32::new(0)),
            frames: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Latest callback peak in `0.0..=1.0`.
    #[must_use]
    pub fn peak(&self) -> f32 {
        f32::from_bits(self.peak.load(Ordering::Relaxed))
    }

    /// Latest callback RMS in `0.0..=1.0`.
    #[must_use]
    pub fn rms(&self) -> f32 {
        f32::from_bits(self.rms.load(Ordering::Relaxed))
    }

    /// Frames delivered by the input callback.
    #[must_use]
    pub fn frames(&self) -> u64 {
        self.frames.load(Ordering::Relaxed)
    }

    fn update(&self, peak: f32, rms: f32, frames: u64) {
        self.peak.store(peak.to_bits(), Ordering::Relaxed);
        self.rms.store(rms.to_bits(), Ordering::Relaxed);
        self.frames.fetch_add(frames, Ordering::Relaxed);
    }
}

#[derive(Debug, Default)]
struct CaptureStats {
    callbacks: AtomicU64,
    dropped: AtomicU64,
    stream_error: AtomicBool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CaptureManifest {
    version: u32,
    sample_rate: u32,
    channels: u16,
    chunk_frames: u64,
    frames: u64,
    complete: bool,
    started_at_ms: u64,
}

/// A live capture. Call [`Recording::stop`] to finalize the WAV.
pub struct Recording {
    input_stream: Option<cpal::Stream>,
    monitor_stream: Option<cpal::Stream>,
    done: Arc<AtomicBool>,
    stats: Arc<CaptureStats>,
    meter: InputMeter,
    writer: Option<std::thread::JoinHandle<Result<u64, EngineError>>>,
    spool: PathBuf,
    manifest: PathBuf,
    output: PathBuf,
    sample_rate: u32,
    channels: u16,
    started: Instant,
}

impl Recording {
    /// Live input meter.
    #[must_use]
    pub fn meter(&self) -> &InputMeter {
        &self.meter
    }

    /// Capture sample rate.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Capture channel count.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Wall-clock elapsed time.
    #[must_use]
    pub fn elapsed_secs(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    /// Frames handed to the capture callback.
    #[must_use]
    pub fn input_frames(&self) -> u64 {
        self.meter.frames()
    }

    /// Stop capture, finalize the WAV and remove the recovery spool.
    pub fn stop(mut self) -> Result<CaptureReport, EngineError> {
        drop(self.input_stream.take());
        drop(self.monitor_stream.take());
        self.done.store(true, Ordering::Release);
        let written = self
            .writer
            .take()
            .ok_or_else(|| EngineError::Stream("capture writer is missing".into()))?
            .join()
            .map_err(|_| EngineError::Stream("capture writer thread panicked".into()))??;
        let frames = written / u64::from(self.channels);
        finalize_spool(
            &self.output,
            &self.spool,
            self.sample_rate,
            self.channels,
            frames,
        )?;
        let _ = std::fs::remove_file(&self.manifest);
        Ok(CaptureReport {
            frames,
            sample_rate: self.sample_rate,
            channels: self.channels,
            dropped: self.stats.dropped.load(Ordering::Relaxed),
            callbacks: self.stats.callbacks.load(Ordering::Relaxed),
            elapsed_secs: self.started.elapsed().as_secs_f64(),
            stream_error: self.stats.stream_error.load(Ordering::Relaxed),
            recovered: false,
        })
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Release);
        drop(self.input_stream.take());
        drop(self.monitor_stream.take());
    }
}

/// Start recording a selected input device.
pub fn start_recording(
    path: impl AsRef<Path>,
    config: &CaptureConfig,
) -> Result<Recording, EngineError> {
    let host = cpal::default_host();
    let device = input_device(&host, config.device.as_deref())?;
    let supported = device
        .default_input_config()
        .map_err(|error| EngineError::Stream(format!("default input config: {error}")))?;
    let sample_format = supported.sample_format();
    let stream_config: StreamConfig = supported.config();
    let sample_rate = stream_config.sample_rate;
    let channels = stream_config.channels;
    let chunk_frames = config.chunk_frames.max(u64::from(sample_rate)).max(1);

    let output = path.as_ref().to_path_buf();
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let spool = spool_path(&output);
    let manifest_path = manifest_path(&output);
    if spool.exists() || manifest_path.exists() {
        return Err(EngineError::Stream(format!(
            "unfinished capture exists at {}; recover it before recording again",
            spool.display()
        )));
    }

    let manifest = CaptureManifest {
        version: MANIFEST_VERSION,
        sample_rate,
        channels,
        chunk_frames,
        frames: 0,
        complete: false,
        started_at_ms: now_ms(),
    };
    write_manifest(&manifest_path, &manifest)?;
    let spool_file = File::create(&spool)?;

    // 10 seconds of samples so a long fsync on the chunk boundary cannot
    // starve the ring buffer and force callback drops.
    let capacity = (sample_rate as usize * usize::from(channels) * 2 * 10).max(65_536);
    let (producer, consumer) = RingBuffer::<f32>::new(capacity);
    let (monitor_producer, monitor_consumer) = if config.monitor {
        let (producer, consumer) = RingBuffer::<f32>::new(capacity);
        (Some(producer), Some(consumer))
    } else {
        (None, None)
    };
    let monitor_stream = if let Some(consumer) = monitor_consumer {
        match build_monitor(consumer, channels, sample_rate, &manifest) {
            Ok(stream) => Some(stream),
            Err(error) => {
                let _ = std::fs::remove_file(&spool);
                let _ = std::fs::remove_file(&manifest_path);
                return Err(error);
            }
        }
    } else {
        None
    };

    let stats = Arc::new(CaptureStats::default());
    let meter = InputMeter::new();
    let done = Arc::new(AtomicBool::new(false));
    let writer_done = done.clone();
    let writer_manifest = manifest_path.clone();
    let writer_stats = stats.clone();
    let writer = std::thread::spawn(move || {
        let result = write_spool(
            spool_file,
            consumer,
            writer_done,
            &writer_manifest,
            &manifest,
            &writer_stats,
        );
        if result.is_err() {
            let _ = std::fs::remove_file(&writer_manifest);
        }
        result
    });

    let stats_cb = stats.clone();
    let meter_cb = meter.clone();
    let done_cb = done.clone();
    let error_flag = stats.clone();
    let mut producer = producer;
    let mut monitor_producer = monitor_producer;
    let stream_result = match sample_format {
        SampleFormat::F32 => device.build_input_stream::<f32, _, _>(
            stream_config,
            move |data, _| {
                stats_cb.callbacks.fetch_add(1, Ordering::Relaxed);
                push_input(
                    data,
                    &mut producer,
                    &mut monitor_producer,
                    &stats_cb,
                    &meter_cb,
                    channels,
                );
            },
            move |_error| {
                error_flag.stream_error.store(true, Ordering::Relaxed);
            },
            None,
        ),
        SampleFormat::I16 => device.build_input_stream::<i16, _, _>(
            stream_config,
            move |data, _| {
                stats_cb.callbacks.fetch_add(1, Ordering::Relaxed);
                push_input(
                    data,
                    &mut producer,
                    &mut monitor_producer,
                    &stats_cb,
                    &meter_cb,
                    channels,
                );
            },
            move |_error| {
                error_flag.stream_error.store(true, Ordering::Relaxed);
            },
            None,
        ),
        other => {
            done_cb.store(true, Ordering::Release);
            let _ = writer.join();
            let _ = std::fs::remove_file(&spool);
            let _ = std::fs::remove_file(&manifest_path);
            return Err(EngineError::UnsupportedFormat(format!(
                "input sample format {other:?}"
            )));
        }
    };

    let stream = match stream_result {
        Ok(stream) => stream,
        Err(error) => {
            done_cb.store(true, Ordering::Release);
            let _ = writer.join();
            let _ = std::fs::remove_file(&spool);
            let _ = std::fs::remove_file(&manifest_path);
            return Err(EngineError::Stream(format!("build input stream: {error}")));
        }
    };
    if let Err(error) = stream.play() {
        done_cb.store(true, Ordering::Release);
        drop(stream);
        let _ = writer.join();
        let _ = std::fs::remove_file(&spool);
        let _ = std::fs::remove_file(&manifest_path);
        return Err(EngineError::Stream(format!("start capture: {error}")));
    }
    if let Some(monitor) = &monitor_stream
        && let Err(error) = monitor.play()
    {
        done_cb.store(true, Ordering::Release);
        drop(monitor_stream);
        drop(stream);
        let _ = writer.join();
        let _ = std::fs::remove_file(&spool);
        let _ = std::fs::remove_file(&manifest_path);
        return Err(EngineError::Stream(format!("start monitoring: {error}")));
    }

    Ok(Recording {
        input_stream: Some(stream),
        monitor_stream,
        done,
        stats,
        meter,
        writer: Some(writer),
        spool,
        manifest: manifest_path,
        output,
        sample_rate,
        channels,
        started: Instant::now(),
    })
}

/// Record the default input device to a WAV file for `seconds`.
pub fn record_to_wav(path: impl AsRef<Path>, seconds: f64) -> Result<CaptureReport, EngineError> {
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(EngineError::Stream(
            "record duration must be positive".into(),
        ));
    }
    let recording = start_recording(path, &CaptureConfig::default())?;
    std::thread::sleep(Duration::from_secs_f64(seconds));
    recording.stop()
}

/// Finalize a spool left by a killed process.
pub fn recover_capture(path: impl AsRef<Path>) -> Result<CaptureReport, EngineError> {
    let output = path.as_ref();
    let spool = spool_path(output);
    let manifest_path = manifest_path(output);
    let text = std::fs::read_to_string(&manifest_path).map_err(|error| {
        EngineError::Stream(format!(
            "read capture manifest {}: {error}",
            manifest_path.display()
        ))
    })?;
    let manifest: CaptureManifest = serde_json::from_str(&text)
        .map_err(|error| EngineError::Stream(format!("parse capture manifest: {error}")))?;
    if manifest.version != MANIFEST_VERSION {
        return Err(EngineError::Stream(format!(
            "unsupported capture manifest version {}",
            manifest.version
        )));
    }
    let bytes = std::fs::metadata(&spool)
        .map_err(|error| EngineError::Stream(format!("stat capture spool: {error}")))?
        .len();
    let frame_bytes = u64::from(manifest.channels) * 4;
    if frame_bytes == 0 {
        return Err(EngineError::Stream("capture has zero channels".into()));
    }
    let spool_frames = bytes / frame_bytes;
    if manifest.frames > spool_frames {
        return Err(EngineError::Stream(format!(
            "capture manifest commits {} frames but the spool only holds {}",
            manifest.frames, spool_frames
        )));
    }
    // The writer fsyncs the spool before each manifest write, so
    // `manifest.frames` is the authoritative committed frame count. Spool
    // blocks beyond the last sync were never acknowledged and may be zeros
    // or stale garbage after a crash: never finalize them into the WAV.
    let frames = manifest.frames;
    if frames == 0 {
        return Err(EngineError::Stream(
            "capture spool has no complete frames".into(),
        ));
    }
    finalize_spool(
        output,
        &spool,
        manifest.sample_rate,
        manifest.channels,
        frames,
    )?;
    let _ = std::fs::remove_file(&spool);
    let _ = std::fs::remove_file(&manifest_path);
    Ok(CaptureReport {
        frames,
        sample_rate: manifest.sample_rate,
        channels: manifest.channels,
        dropped: 0,
        callbacks: 0,
        elapsed_secs: frames as f64 / f64::from(manifest.sample_rate),
        stream_error: false,
        recovered: true,
    })
}

/// Path of the raw spool for an output WAV.
#[must_use]
pub fn spool_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".alp-capture");
    PathBuf::from(name)
}

/// Path of the recovery manifest for an output WAV.
#[must_use]
pub fn manifest_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".alp-capture.json");
    PathBuf::from(name)
}

fn input_device(host: &cpal::Host, requested: Option<&str>) -> Result<cpal::Device, EngineError> {
    if let Some(requested) = requested {
        let mut devices = host
            .input_devices()
            .map_err(|error| EngineError::Stream(format!("enumerate inputs: {error}")))?;
        return devices
            .find(|device| {
                device
                    .description()
                    .is_ok_and(|description| description.name() == requested)
            })
            .ok_or_else(|| EngineError::NoDevice(format!("input device {requested:?} not found")));
    }
    host.default_input_device()
        .ok_or_else(|| EngineError::NoDevice("no default input device".into()))
}

fn push_input<T: Copy + Into<f32>>(
    data: &[T],
    producer: &mut Producer<f32>,
    monitor: &mut Option<Producer<f32>>,
    stats: &CaptureStats,
    meter: &InputMeter,
    channels: u16,
) {
    let channels = channels.max(1) as usize;
    // Meter the full callback first: meter coverage must not depend on ring
    // space.
    let mut peak = 0.0f32;
    let mut sum = 0.0f32;
    for raw in data {
        let value: f32 = (*raw).into();
        peak = peak.max(value.abs());
        sum += value * value;
    }
    let count = data.len().max(1) as f32;
    meter.update(
        peak,
        (sum / count).sqrt(),
        u64::try_from(data.len() / channels).unwrap_or(0),
    );

    // Hand off whole frames only. If a sample-level push ever landed a
    // partial frame during a ring overflow, every following frame in the
    // take would play with swapped channels. When the ring cannot fit the
    // next whole frame, the remaining audio of this callback is dropped
    // frame-atomically instead.
    for (offset, frame) in data.chunks_exact(channels).enumerate() {
        let Ok(mut chunk) = producer.write_chunk(channels) else {
            let dropped_samples = u64::try_from(data.len() - offset * channels).unwrap_or(0);
            stats.dropped.fetch_add(dropped_samples, Ordering::Relaxed);
            return;
        };
        let (first, second) = chunk.as_mut_slices();
        let split = first.len();
        for (slot, raw) in first.iter_mut().zip(&frame[..split]) {
            *slot = (*raw).into();
        }
        for (slot, raw) in second.iter_mut().zip(&frame[split..]) {
            *slot = (*raw).into();
        }
        chunk.commit_all();
        if let Some(monitor) = monitor.as_mut() {
            for raw in frame {
                let _ = monitor.push((*raw).into());
            }
        }
    }
    // A trailing partial sample group (never produced by cpal's frame-aligned
    // buffers) is ignored so interleave alignment can never drift.
}

fn write_spool(
    file: File,
    mut consumer: Consumer<f32>,
    done: Arc<AtomicBool>,
    manifest_path: &Path,
    manifest: &CaptureManifest,
    stats: &CaptureStats,
) -> Result<u64, EngineError> {
    let mut writer = BufWriter::new(file);
    let mut bytes = Vec::with_capacity(64 * 1024);
    let mut written = 0u64;
    let mut chunk_samples = 0u64;
    let chunk_samples_limit = manifest
        .chunk_frames
        .saturating_mul(u64::from(manifest.channels))
        .max(1);
    let mut manifest = CaptureManifest {
        version: manifest.version,
        sample_rate: manifest.sample_rate,
        channels: manifest.channels,
        chunk_frames: manifest.chunk_frames,
        frames: 0,
        complete: false,
        started_at_ms: manifest.started_at_ms,
    };

    loop {
        while let Ok(sample) = consumer.pop() {
            bytes.extend_from_slice(&sample.to_le_bytes());
            written += 1;
            chunk_samples += 1;
        }
        if !bytes.is_empty() {
            writer.write_all(&bytes).map_err(EngineError::Io)?;
            bytes.clear();
        }
        if chunk_samples >= chunk_samples_limit {
            writer.flush().map_err(EngineError::Io)?;
            writer.get_ref().sync_all().map_err(EngineError::Io)?;
            manifest.frames = written / u64::from(manifest.channels);
            write_manifest(manifest_path, &manifest)?;
            chunk_samples = 0;
        }
        if done.load(Ordering::Acquire) && consumer.is_empty() {
            break;
        }
        std::thread::sleep(WRITER_POLL);
    }
    writer.flush().map_err(EngineError::Io)?;
    writer.get_ref().sync_all().map_err(EngineError::Io)?;
    manifest.frames = written / u64::from(manifest.channels);
    write_manifest(manifest_path, &manifest)?;
    let _ = stats;
    Ok(written)
}

fn write_manifest(path: &Path, manifest: &CaptureManifest) -> Result<(), EngineError> {
    let json = serde_json::to_vec(manifest)
        .map_err(|error| EngineError::Stream(format!("serialize capture manifest: {error}")))?;
    write_atomic(path, &json)?;
    Ok(())
}

fn build_monitor(
    consumer: Consumer<f32>,
    source_channels: u16,
    source_rate: u32,
    manifest: &CaptureManifest,
) -> Result<cpal::Stream, EngineError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| EngineError::NoDevice("no default output device for monitoring".into()))?;
    let supported = device
        .default_output_config()
        .map_err(|error| EngineError::Stream(format!("default output config: {error}")))?;
    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.config();
    let target_channels = config.channels;
    let target_rate = config.sample_rate;
    let step = f64::from(source_rate) / f64::from(target_rate.max(1));
    let mut state = MonitorState::new(consumer, source_channels, target_channels, step);
    let _ = manifest;
    match sample_format {
        SampleFormat::F32 => device
            .build_output_stream::<f32, _, _>(
                config,
                move |data, _| state.render_f32(data),
                |_error| {},
                None,
            )
            .map_err(|error| EngineError::Stream(format!("build monitor stream: {error}"))),
        SampleFormat::I16 => device
            .build_output_stream::<i16, _, _>(
                config,
                move |data, _| state.render_i16(data),
                |_error| {},
                None,
            )
            .map_err(|error| EngineError::Stream(format!("build monitor stream: {error}"))),
        other => Err(EngineError::UnsupportedFormat(format!(
            "monitor sample format {other:?}"
        ))),
    }
}

struct MonitorState {
    consumer: Consumer<f32>,
    source_channels: usize,
    target_channels: usize,
    step: f64,
    phase: f64,
    previous: Vec<f32>,
    underflow: bool,
}

impl MonitorState {
    fn new(consumer: Consumer<f32>, source_channels: u16, target_channels: u16, step: f64) -> Self {
        let source_channels = usize::from(source_channels.max(1));
        Self {
            consumer,
            source_channels,
            target_channels: usize::from(target_channels.max(1)),
            step: step.max(0.000_001),
            phase: 0.0,
            previous: vec![0.0; source_channels],
            underflow: false,
        }
    }

    fn read_frame(&mut self) -> bool {
        if self.consumer.slots() < self.source_channels {
            return false;
        }
        for sample in &mut self.previous {
            let Ok(value) = self.consumer.pop() else {
                return false;
            };
            *sample = value;
        }
        true
    }

    fn advance(&mut self) {
        self.phase += self.step;
        self.underflow = false;
        while self.phase >= 1.0 {
            if !self.read_frame() {
                self.underflow = true;
                self.phase = 0.0;
                break;
            }
            self.phase -= 1.0;
        }
    }

    fn sample(&self, channel: usize) -> f32 {
        if self.underflow {
            0.0
        } else {
            self.previous[channel % self.source_channels]
        }
    }

    fn render_f32(&mut self, data: &mut [f32]) {
        for frame in data.chunks_mut(self.target_channels) {
            self.advance();
            for (channel, sample) in frame.iter_mut().enumerate() {
                *sample = self.sample(channel);
            }
        }
    }

    fn render_i16(&mut self, data: &mut [i16]) {
        for frame in data.chunks_mut(self.target_channels) {
            self.advance();
            for (channel, sample) in frame.iter_mut().enumerate() {
                *sample =
                    (self.sample(channel).clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
            }
        }
    }
}

fn finalize_spool(
    output: &Path,
    spool: &Path,
    sample_rate: u32,
    channels: u16,
    frames: u64,
) -> Result<(), EngineError> {
    let (temp, file) = create_temp_sibling(output)?;
    let result = (|| -> Result<(), EngineError> {
        let reader = File::open(spool)?;
        let mut reader = BufReader::new(reader);
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::new(BufWriter::new(file), spec)
            .map_err(|error| EngineError::Media(MediaError::Encode(error.to_string())))?;
        let total_samples = frames.saturating_mul(u64::from(channels));
        let mut remaining = total_samples;
        let mut buffer = vec![0u8; 64 * 1024];
        while remaining > 0 {
            let samples = remaining.min((buffer.len() / 4) as u64);
            let bytes = usize::try_from(samples * 4).unwrap_or(buffer.len());
            reader.read_exact(&mut buffer[..bytes])?;
            for chunk in buffer[..bytes].as_chunks::<4>().0 {
                let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                writer
                    .write_sample(sample)
                    .map_err(|error| EngineError::Media(MediaError::Encode(error.to_string())))?;
            }
            remaining -= samples;
        }
        writer
            .finalize()
            .map_err(|error| EngineError::Media(MediaError::Encode(error.to_string())))?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            commit_temp(&temp, output)?;
            Ok(())
        }
        Err(error) => {
            let _ = std::fs::remove_file(&temp);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_spool(dir: &Path, frames: u64, channels: u16) -> (PathBuf, PathBuf) {
        let output = dir.join("take.wav");
        let spool = spool_path(&output);
        let manifest_path = manifest_path(&output);
        let mut file = File::create(&spool).expect("spool");
        for index in 0..frames * u64::from(channels) {
            let sample = if index.is_multiple_of(2) {
                0.5f32
            } else {
                -0.25
            };
            file.write_all(&sample.to_le_bytes()).expect("write");
        }
        file.sync_all().expect("sync");
        let manifest = CaptureManifest {
            version: MANIFEST_VERSION,
            sample_rate: 48_000,
            channels,
            chunk_frames: 48_000,
            // The spool content is the last fsynced chunk state: what it
            // holds is what the manifest committed.
            frames,
            complete: false,
            started_at_ms: now_ms(),
        };
        write_manifest(&manifest_path, &manifest).expect("manifest");
        (spool, manifest_path)
    }

    #[test]
    fn recovery_finalizes_an_interrupted_spool() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (spool, manifest) = create_spool(dir.path(), 100, 2);
        let report = recover_capture(dir.path().join("take.wav")).expect("recover");
        assert!(report.recovered);
        assert_eq!(report.frames, 100);
        assert_eq!(report.sample_rate, 48_000);
        assert!(!spool.exists());
        assert!(!manifest.exists());
        let mut reader = hound::WavReader::open(dir.path().join("take.wav")).expect("wav");
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
        assert_eq!(reader.samples::<f32>().count(), 200);
    }

    #[test]
    fn recovery_rejects_a_missing_manifest() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(recover_capture(dir.path().join("missing.wav")).is_err());
    }

    #[test]
    fn meter_reads_are_relaxed_and_zero_initially() {
        let meter = InputMeter::new();
        assert_eq!(meter.peak(), 0.0);
        assert_eq!(meter.rms(), 0.0);
        assert_eq!(meter.frames(), 0);
        meter.update(0.75, 0.5, 10);
        assert!((meter.peak() - 0.75).abs() < f32::EPSILON);
        assert!((meter.rms() - 0.5).abs() < f32::EPSILON);
        assert_eq!(meter.frames(), 10);
    }

    #[test]
    fn push_input_never_leaves_a_partial_frame_in_the_ring() {
        let (mut producer, mut consumer) = RingBuffer::<f32>::new(5);
        let stats = CaptureStats::default();
        let meter = InputMeter::new();
        let mut monitor = None;

        // Stereo batches: right = left + 1, so a mispaired drain is
        // instantly recognizable. The ring holds 5 slots; three frames need 6.
        let batch_a = [10.0f32, 11.0, 20.0, 21.0, 30.0, 31.0];
        let batch_b = [40.0f32, 41.0, 50.0, 51.0, 60.0, 61.0];
        push_input(&batch_a, &mut producer, &mut monitor, &stats, &meter, 2);
        push_input(&batch_b, &mut producer, &mut monitor, &stats, &meter, 2);

        let drained: Vec<f32> = std::iter::from_fn(|| consumer.pop().ok()).collect();
        assert_eq!(
            drained.len() % 2,
            0,
            "a stereo take must never hold a dangling half frame, got {drained:?}"
        );
        for [left, right] in drained.as_chunks::<2>().0 {
            assert_eq!(
                *right,
                *left + 1.0,
                "left/right samples must drain as whole frames, got ({left}, {right})"
            );
        }
        assert_eq!(drained, vec![10.0, 11.0, 20.0, 21.0]);
        assert_eq!(
            stats.dropped.load(Ordering::Relaxed),
            8,
            "overflow must drop whole frames: 2 samples from batch A, 6 from batch B"
        );
    }

    #[test]
    fn recovery_truncates_spool_tail_beyond_committed_frames() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_spool, manifest_path) = create_spool(dir.path(), 12, 1);
        // The final chunk sync committed 9 frames; the spool's trailing
        // frames were never acknowledged and are stale blocks after a crash.
        let manifest = CaptureManifest {
            version: MANIFEST_VERSION,
            sample_rate: 48_000,
            channels: 1,
            chunk_frames: 48_000,
            frames: 9,
            complete: false,
            started_at_ms: now_ms(),
        };
        write_manifest(&manifest_path, &manifest).expect("rewrite manifest");

        let report = recover_capture(dir.path().join("take.wav")).expect("recover");
        assert_eq!(
            report.frames, 9,
            "recovery must truncate to the manifest's committed frame count"
        );
    }

    #[test]
    fn recovery_rejects_manifest_committing_more_frames_than_spool_holds() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_spool, manifest_path) = create_spool(dir.path(), 2, 1);
        let manifest = CaptureManifest {
            version: MANIFEST_VERSION,
            sample_rate: 48_000,
            channels: 1,
            chunk_frames: 48_000,
            frames: 50,
            complete: false,
            started_at_ms: now_ms(),
        };
        write_manifest(&manifest_path, &manifest).expect("rewrite manifest");

        let result = recover_capture(dir.path().join("take.wav"));
        assert!(
            result.is_err(),
            "a manifest committing more frames than the spool holds is torn and must not be finalized"
        );
    }
}
