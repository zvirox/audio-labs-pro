//! cpal device probing plus playback/capture stress measurements.
//!
//! The audio callback touches only preallocated memory and the lock-free ring
//! buffer: no locks, no allocation, no logging.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, StreamConfig};

const TIMESTAMP_SLOTS: usize = 1 << 16;

pub fn list() -> Result<(), String> {
    let host = cpal::default_host();
    println!("host: {:?}", host.id());

    let output = host
        .default_output_device()
        .map(|d| d.to_string())
        .unwrap_or_else(|| "<none>".into());
    let input = host
        .default_input_device()
        .map(|d| d.to_string())
        .unwrap_or_else(|| "<none>".into());
    println!("default output: {output}");
    println!("default input:  {input}");

    if let Ok(devices) = host.output_devices() {
        for device in devices {
            let name = device.to_string();
            match device.default_output_config() {
                Ok(config) => println!(
                    "  out: {name} · {} Hz · {} ch · {:?}",
                    config.sample_rate(),
                    config.channels(),
                    config.sample_format()
                ),
                Err(error) => println!("  out: {name} · unsupported ({error})"),
            }
        }
    }

    if let Ok(devices) = host.input_devices() {
        for device in devices {
            let name = device.to_string();
            match device.default_input_config() {
                Ok(config) => println!(
                    "  in:  {name} · {} Hz · {} ch · {:?}",
                    config.sample_rate(),
                    config.channels(),
                    config.sample_format()
                ),
                Err(error) => println!("  in:  {name} · unsupported ({error})"),
            }
        }
    }

    Ok(())
}

/// Stats gathered by the stream callback.
struct CallbackStats {
    timestamps_ns: Vec<AtomicU64>,
    count: AtomicUsize,
    frames: AtomicUsize,
    shortfalls: AtomicUsize,
}

impl CallbackStats {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            timestamps_ns: (0..TIMESTAMP_SLOTS).map(|_| AtomicU64::new(0)).collect(),
            count: AtomicUsize::new(0),
            frames: AtomicUsize::new(0),
            shortfalls: AtomicUsize::new(0),
        })
    }

    fn record(&self, frames: usize, shortfall: usize) {
        let index = self.count.fetch_add(1, Ordering::Relaxed);
        if index < TIMESTAMP_SLOTS {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            self.timestamps_ns[index].store(now, Ordering::Relaxed);
        }
        self.frames.fetch_add(frames, Ordering::Relaxed);
        self.shortfalls.fetch_add(shortfall, Ordering::Relaxed);
    }

    fn report(&self, label: &str, expected_callbacks: Option<f64>) {
        let count = self.count.load(Ordering::Relaxed).min(TIMESTAMP_SLOTS);
        let mut intervals: Vec<f64> = Vec::with_capacity(count);
        for index in 1..count {
            let a = self.timestamps_ns[index - 1].load(Ordering::Relaxed);
            let b = self.timestamps_ns[index].load(Ordering::Relaxed);
            if b > a {
                intervals.push((b - a) as f64 / 1_000_000.0);
            }
        }

        if intervals.is_empty() {
            println!("{label}: no callbacks recorded");
            return;
        }

        let sum: f64 = intervals.iter().sum();
        let mean = sum / intervals.len() as f64;
        let min = intervals.iter().cloned().fold(f64::MAX, f64::min);
        let max = intervals.iter().cloned().fold(f64::MIN, f64::max);
        let spikes = intervals
            .iter()
            .filter(|interval| **interval > mean * 1.5)
            .count();

        println!(
            "{label}: callbacks={} frames={} mean={mean:.2} ms min={min:.2} max={max:.2} \
             gaps(>1.5x mean)={spikes} shortfalls={}",
            self.count.load(Ordering::Relaxed),
            self.frames.load(Ordering::Relaxed),
            self.shortfalls.load(Ordering::Relaxed),
        );

        if let Some(expected) = expected_callbacks {
            println!(
                "  expected callbacks ≈ {expected:.0}, delta = {:.1}%",
                (self.count.load(Ordering::Relaxed) as f64 - expected) / expected * 100.0
            );
        }
    }
}

pub fn play(seconds: f32) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("no default output device")?;
    let supported = device
        .default_output_config()
        .map_err(|e| format!("default output config: {e}"))?;

    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let channels = config.channels as usize;
    let sample_rate = config.sample_rate;
    let frames = (seconds * sample_rate as f32) as usize;

    println!(
        "play: {} Hz · {} ch · {:?} · {seconds}s",
        sample_rate, channels, sample_format
    );

    let capacity = sample_rate as usize * channels * 2;
    let (mut producer, mut consumer) = rtrb::RingBuffer::<f32>::new(capacity);
    let stats = CallbackStats::new();
    let stats_cb = stats.clone();

    // Prefill ~100 ms so the first callbacks do not underrun while the
    // producer thread spins up.
    let prefill_frames = sample_rate as usize / 10;
    let mut phase = 0.0f32;
    let step = std::f32::consts::TAU * 440.0 / sample_rate as f32;
    for _ in 0..prefill_frames {
        for _ in 0..channels {
            let value = phase.sin() * 0.05;
            phase = (phase + step) % std::f32::consts::TAU;
            let _ = producer.push(value);
        }
    }

    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream::<f32, _, _>(
            config,
            move |data, _| {
                let mut shortfall = 0;
                for sample in data.iter_mut() {
                    match consumer.pop() {
                        Ok(value) => *sample = value,
                        Err(_) => {
                            *sample = 0.0;
                            shortfall += 1;
                        }
                    }
                }
                stats_cb.record(data.len() / channels, shortfall);
            },
            |error| eprintln!("stream error: {error}"),
            None,
        ),
        SampleFormat::I16 => device.build_output_stream::<i16, _, _>(
            config,
            move |data, _| {
                let mut shortfall = 0;
                for sample in data.iter_mut() {
                    match consumer.pop() {
                        Ok(value) => *sample = i16::from_sample(value),
                        Err(_) => {
                            *sample = 0;
                            shortfall += 1;
                        }
                    }
                }
                stats_cb.record(data.len() / channels, shortfall);
            },
            |error| eprintln!("stream error: {error}"),
            None,
        ),
        other => return Err(format!("unsupported output sample format {other:?}")),
    }
    .map_err(|e| format!("build output stream: {e}"))?;

    stream.play().map_err(|e| format!("play: {e}"))?;

    let producer_handle = std::thread::spawn(move || {
        let mut producer = producer;
        let mut phase = 0.0f32;
        let step = std::f32::consts::TAU * 440.0 / sample_rate as f32;
        // Pace in 1024-frame blocks so the ring does not starve or overfill.
        let block_frames = 1024usize;
        let blocks = frames / block_frames;
        let started = Instant::now();

        for block in 0..blocks {
            for _ in 0..block_frames {
                for _ in 0..channels {
                    let value = phase.sin() * 0.05;
                    phase = (phase + step) % std::f32::consts::TAU;
                    if producer.push(value).is_err() {
                        // Dropped: consumer is behind.
                        std::thread::yield_now();
                    }
                }
            }

            let deadline = started
                + Duration::from_secs_f64(
                    (block + 1) as f64 * block_frames as f64 / sample_rate as f64,
                );
            let now = Instant::now();
            if deadline > now {
                std::thread::sleep(deadline - now);
            }
        }

        let elapsed = started.elapsed();
        println!(
            "producer: wrote {} frames in {:?}",
            blocks * block_frames,
            elapsed
        );
    });

    std::thread::sleep(Duration::from_secs_f32(seconds + 0.2));
    drop(stream);
    producer_handle.join().ok();

    let expected_callbacks = seconds as f64 * sample_rate as f64 / (sample_rate as f64 * 0.01);
    stats.report("playback", Some(expected_callbacks));
    Ok(())
}

pub fn record(path: &str, seconds: f32) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("no default input device")?;
    let supported = device
        .default_input_config()
        .map_err(|e| format!("default input config: {e}"))?;

    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let channels = config.channels as usize;
    let sample_rate = config.sample_rate;
    let capacity = (seconds * sample_rate as f32) as usize * channels + 4096;

    println!(
        "record: {} Hz · {} ch · {:?} · {seconds}s",
        sample_rate, channels, sample_format
    );

    let buffer: Arc<Vec<AtomicU64>> = Arc::new((0..capacity).map(|_| AtomicU64::new(0)).collect());
    let write_index = Arc::new(AtomicUsize::new(0));
    let stats = CallbackStats::new();

    let buffer_cb = buffer.clone();
    let index_cb = write_index.clone();
    let stats_cb = stats.clone();

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream::<f32, _, _>(
            config,
            move |data, _| {
                let start = index_cb.fetch_add(data.len(), Ordering::Relaxed);
                for (offset, value) in data.iter().enumerate() {
                    if let Some(slot) = buffer_cb.get(start + offset) {
                        slot.store(value.to_bits() as u64, Ordering::Relaxed);
                    }
                }
                stats_cb.record(data.len() / channels, 0);
            },
            |error| eprintln!("stream error: {error}"),
            None,
        ),
        SampleFormat::I16 => device.build_input_stream::<i16, _, _>(
            config,
            move |data, _| {
                let start = index_cb.fetch_add(data.len(), Ordering::Relaxed);
                for (offset, value) in data.iter().enumerate() {
                    if let Some(slot) = buffer_cb.get(start + offset) {
                        slot.store(f32::from_sample(*value).to_bits() as u64, Ordering::Relaxed);
                    }
                }
                stats_cb.record(data.len() / channels, 0);
            },
            |error| eprintln!("stream error: {error}"),
            None,
        ),
        other => return Err(format!("unsupported input sample format {other:?}")),
    }
    .map_err(|e| format!("build input stream: {e}"))?;

    stream.play().map_err(|e| format!("capture: {e}"))?;
    std::thread::sleep(Duration::from_secs_f32(seconds));
    drop(stream);

    let written = write_index.load(Ordering::Relaxed).min(capacity);
    let frames = written / channels;
    let spec = hound::WavSpec {
        channels: channels as u16,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer =
        hound::WavWriter::create(path, spec).map_err(|e| format!("create wav: {e}"))?;
    for index in 0..written {
        let bits = buffer[index].load(Ordering::Relaxed) as u32;
        writer
            .write_sample(f32::from_bits(bits))
            .map_err(|e| format!("write wav: {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("finalize wav: {e}"))?;

    println!("wrote {frames} frames to {path}");
    let callbacks = stats.count.load(Ordering::Relaxed).max(1) as f64;
    let frames_per_callback = stats.frames.load(Ordering::Relaxed) as f64 / callbacks;
    let expected_callbacks = seconds as f64 * sample_rate as f64 / frames_per_callback;
    stats.report("capture", Some(expected_callbacks));
    Ok(())
}
