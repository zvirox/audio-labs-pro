//! Device commands: enumerate and record.

use std::path::Path;

use alp_engine::{list_devices, record_to_wav};
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

/// Record the default input device.
pub fn record(output: &Path, seconds: f64) -> Result<()> {
    let report = record_to_wav(output, seconds).context("record audio")?;
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
