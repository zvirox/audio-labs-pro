//! Generate a long test WAV so decode/peak timings do not depend on external
//! media.

use std::time::Instant;

pub fn generate(path: &str, seconds: f32) -> Result<(), String> {
    let sample_rate = 48_000u32;
    let total = (seconds * sample_rate as f32) as u64;

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let started = Instant::now();
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| format!("create: {e}"))?;
    let mut writer = writer.get_i16_writer(total as u32);

    let mut phase = 0.0f32;
    let mut noise_state = 0x1234_5678u64;
    for index in 0..total {
        let t = index as f32 / sample_rate as f32;
        let phrase = (t * 0.21).sin().max(0.0);
        let carrier = (std::f32::consts::TAU * 180.0 * t).sin() * 0.6
            + (std::f32::consts::TAU * 360.0 * t).sin() * 0.3;
        noise_state = noise_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let noise = ((noise_state >> 40) as f32 / 8_388_608.0 - 0.5) * 0.05;
        let value = (carrier * phrase + noise).clamp(-1.0, 1.0) * 0.4;
        phase = (phase + value) % 1.0;
        writer.write_sample((value * i16::MAX as f32) as i16);
    }
    // `flush` on the typed writer also finalizes the parent WAV header.
    writer.flush().map_err(|e| format!("flush: {e}"))?;

    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    println!(
        "generated {path}: {seconds:.0}s · {size} bytes ({:.1} MB) in {:?}",
        size as f64 / 1_048_576.0,
        started.elapsed()
    );
    Ok(())
}
