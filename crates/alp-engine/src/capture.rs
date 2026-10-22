//! Input capture to WAV through a lock-free ring buffer.
//!
//! The input callback only copies converted samples into an SPSC ring and bumps
//! atomics. A writer thread performs all WAV encoding and disk I/O.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use alp_core::{commit_temp, create_temp_sibling};
use alp_media::MediaError;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use rtrb::RingBuffer;

use crate::error::EngineError;

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
}

#[derive(Debug, Default)]
struct CaptureStats {
    callbacks: AtomicU64,
    dropped: AtomicU64,
    stream_error: AtomicBool,
}

/// Record the default input device to a WAV file for `seconds`.
pub fn record_to_wav(path: impl AsRef<Path>, seconds: f64) -> Result<CaptureReport, EngineError> {
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(EngineError::Stream(
            "record duration must be positive".into(),
        ));
    }
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| EngineError::NoDevice("no default input device".into()))?;
    let supported = device
        .default_input_config()
        .map_err(|error| EngineError::Stream(format!("default input config: {error}")))?;
    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.config();
    let sample_rate = config.sample_rate;
    let channels = config.channels;

    let capacity = (sample_rate as usize * usize::from(channels) * 2).max(65_536);
    let (mut producer, mut consumer) = RingBuffer::<f32>::new(capacity);
    let stats = Arc::new(CaptureStats::default());
    let stats_cb = stats.clone();
    let error_flag = stats.clone();
    let done = Arc::new(AtomicBool::new(false));
    let done_cb = done.clone();

    let path = path.as_ref();
    let (temp, file) = create_temp_sibling(path)?;
    let writer_done = done.clone();
    let writer = std::thread::spawn(move || -> Result<u64, EngineError> {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::new(file, spec)
            .map_err(|error| EngineError::Media(MediaError::Encode(error.to_string())))?;
        let mut written = 0u64;
        loop {
            while let Ok(sample) = consumer.pop() {
                writer
                    .write_sample(sample)
                    .map_err(|error| EngineError::Media(MediaError::Encode(error.to_string())))?;
                written += 1;
            }
            if writer_done.load(Ordering::Acquire) && consumer.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        writer
            .finalize()
            .map_err(|error| EngineError::Media(MediaError::Encode(error.to_string())))?;
        Ok(written)
    });

    let stream_result = match sample_format {
        SampleFormat::F32 => device.build_input_stream::<f32, _, _>(
            config,
            move |data, _| {
                stats_cb.callbacks.fetch_add(1, Ordering::Relaxed);
                for sample in data {
                    if producer.push(*sample).is_err() {
                        stats_cb.dropped.fetch_add(1, Ordering::Relaxed);
                    }
                }
            },
            move |_error| {
                error_flag.stream_error.store(true, Ordering::Relaxed);
            },
            None,
        ),
        SampleFormat::I16 => device.build_input_stream::<i16, _, _>(
            config,
            move |data, _| {
                stats_cb.callbacks.fetch_add(1, Ordering::Relaxed);
                for sample in data {
                    let value = f32::from(*sample) / 32_768.0;
                    if producer.push(value).is_err() {
                        stats_cb.dropped.fetch_add(1, Ordering::Relaxed);
                    }
                }
            },
            move |_error| {
                error_flag.stream_error.store(true, Ordering::Relaxed);
            },
            None,
        ),
        other => {
            done_cb.store(true, Ordering::Release);
            let _ = writer.join();
            let _ = std::fs::remove_file(&temp);
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
            let _ = std::fs::remove_file(&temp);
            return Err(EngineError::Stream(format!("build input stream: {error}")));
        }
    };
    if let Err(error) = stream.play() {
        done_cb.store(true, Ordering::Release);
        let _ = writer.join();
        let _ = std::fs::remove_file(&temp);
        return Err(EngineError::Stream(format!("start capture: {error}")));
    }

    let started = Instant::now();
    std::thread::sleep(Duration::from_secs_f64(seconds));
    drop(stream);
    done_cb.store(true, Ordering::Release);
    let written = match writer.join() {
        Ok(Ok(written)) => written,
        Ok(Err(error)) => {
            let _ = std::fs::remove_file(&temp);
            return Err(error);
        }
        Err(_) => {
            let _ = std::fs::remove_file(&temp);
            return Err(EngineError::Stream("capture writer thread panicked".into()));
        }
    };

    if let Err(error) = commit_temp(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(EngineError::Io(error));
    }

    Ok(CaptureReport {
        frames: written / u64::from(channels),
        sample_rate,
        channels,
        dropped: stats.dropped.load(Ordering::Relaxed),
        callbacks: stats.callbacks.load(Ordering::Relaxed),
        elapsed_secs: started.elapsed().as_secs_f64(),
        stream_error: stats.stream_error.load(Ordering::Relaxed),
    })
}
