//! Device commands: enumerate, record and recover.

use std::path::Path;
use std::time::Duration;

use alp_engine::{CaptureConfig, list_devices, recover_capture, start_recording};
use anyhow::{Context, Result, bail};

/// List output and input devices.
pub fn devices(json: bool) -> Result<()> {
    let devices = list_devices().context("enumerate audio devices")?;
    if json {
        let entries: Vec<_> = devices
            .iter()
            .map(|device| {
                serde_json::json!({
                    "name": device.name,
                    "direction": match device.direction {
                        alp_engine::DeviceDirection::Output => "output",
                        alp_engine::DeviceDirection::Input => "input",
                    },
                    "default": device.is_default,
                    "channels": device.channels,
                    "sample_rate": device.sample_rate,
                    "sample_format": device.sample_format,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if devices.is_empty() {
        println!("no audio devices found");
        return Ok(());
    }
    for device in devices {
        let direction = match device.direction {
            alp_engine::DeviceDirection::Output => "out",
            alp_engine::DeviceDirection::Input => "in",
        };
        let default = if device.is_default { " (default)" } else { "" };
        println!(
            "{direction:<3} {}{default} · {} ch · {} Hz · {}",
            device.name, device.channels, device.sample_rate, device.sample_format
        );
    }
    Ok(())
}

/// Record an input device with crash-safe chunking.
pub fn record(
    output: &Path,
    seconds: f64,
    device: Option<&str>,
    monitor: bool,
    chunk_seconds: f64,
) -> Result<()> {
    if !seconds.is_finite() || seconds <= 0.0 {
        bail!("record duration must be positive");
    }
    let chunk_frames = if chunk_seconds.is_finite() && chunk_seconds > 0.0 {
        (chunk_seconds * 48_000.0).round().max(1.0) as u64
    } else {
        480_000
    };
    let config = CaptureConfig {
        device: device.map(str::to_owned),
        chunk_frames,
        monitor,
    };
    let recording = start_recording(output, &config).context("start recording")?;
    println!(
        "recording {} · {} Hz · {} ch · device {}",
        output.display(),
        recording.sample_rate(),
        recording.channels(),
        device.unwrap_or("default")
    );
    std::thread::sleep(Duration::from_secs_f64(seconds));
    let report = recording.stop().context("stop recording")?;
    println!("recorded {}", output.display());
    println!("frames:      {}", report.frames);
    println!("sample rate: {} Hz", report.sample_rate);
    println!("channels:    {}", report.channels);
    println!("callbacks:   {}", report.callbacks);
    println!("dropped:     {} samples", report.dropped);
    println!("elapsed:     {:.3}s", report.elapsed_secs);
    if report.stream_error {
        bail!("the audio backend reported a stream error");
    }
    Ok(())
}

/// Finish a capture left behind by a forced kill.
pub fn recover(output: &Path, json: bool) -> Result<()> {
    let report =
        recover_capture(output).with_context(|| format!("recover {}", output.display()))?;
    if json {
        let value = serde_json::json!({
            "path": output.display().to_string(),
            "frames": report.frames,
            "sample_rate": report.sample_rate,
            "channels": report.channels,
            "duration_secs": report.frames as f64 / f64::from(report.sample_rate),
            "recovered": report.recovered,
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }
    println!("recovered {}", output.display());
    println!("frames:      {}", report.frames);
    println!("sample rate: {} Hz", report.sample_rate);
    println!("channels:    {}", report.channels);
    println!(
        "duration:    {:.3}s",
        report.frames as f64 / f64::from(report.sample_rate)
    );
    Ok(())
}
