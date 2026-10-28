//! DSP commands: voice cleanup, loudness measurement and dynamics processing.

use std::path::Path;
use std::time::Instant;

use alp_dsp::{Effect, VoiceCleanup, calculate_normalization_gain, measure_lufs};
use alp_media::{ExportSettings, WavBitDepth, decode, export};
use anyhow::{Context, Result};

/// Measure ITU-R BS.1770 integrated loudness (LUFS) of an audio file.
pub fn measure(input: &Path, json: bool) -> Result<()> {
    let started = Instant::now();
    let buffer = decode(input).with_context(|| format!("decode {}", input.display()))?;
    let lufs = measure_lufs(buffer.samples(), buffer.sample_rate(), buffer.channels());
    let peak = buffer.peak();
    let peak_db = if peak > 0.0 {
        20.0 * peak.log10()
    } else {
        -100.0
    };

    let podcast_gain = calculate_normalization_gain(lufs, -16.0, peak, -1.0);
    let stream_gain = calculate_normalization_gain(lufs, -14.0, peak, -1.0);
    let broadcast_gain = calculate_normalization_gain(lufs, -23.0, peak, -1.0);

    let elapsed = started.elapsed().as_secs_f64();

    if json {
        let value = serde_json::json!({
            "path": input.display().to_string(),
            "sample_rate": buffer.sample_rate(),
            "channels": buffer.channels(),
            "frames": buffer.frames(),
            "duration_secs": buffer.duration_secs(),
            "integrated_lufs": lufs,
            "peak_linear": peak,
            "peak_db": peak_db,
            "normalization": {
                "podcast_target_lufs": -16.0,
                "podcast_gain_multiplier": podcast_gain,
                "stream_target_lufs": -14.0,
                "stream_gain_multiplier": stream_gain,
                "broadcast_target_lufs": -23.0,
                "broadcast_gain_multiplier": broadcast_gain,
            },
            "elapsed_secs": elapsed,
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!("file:            {}", input.display());
    println!(
        "format:          {} Hz · {} ch",
        buffer.sample_rate(),
        buffer.channels()
    );
    println!(
        "duration:        {:.3}s ({} frames)",
        buffer.duration_secs(),
        buffer.frames()
    );
    println!("integrated:      {:.2} LUFS", lufs);
    println!("true peak:       {:.2} dBFS ({:.4} linear)", peak_db, peak);
    println!(
        "podcast (-16):   {:.2}x gain ({:+.2} dB)",
        podcast_gain,
        20.0 * podcast_gain.log10()
    );
    println!(
        "stream (-14):    {:.2}x gain ({:+.2} dB)",
        stream_gain,
        20.0 * stream_gain.log10()
    );
    println!(
        "broadcast (-23): {:.2}x gain ({:+.2} dB)",
        broadcast_gain,
        20.0 * broadcast_gain.log10()
    );
    println!("elapsed:         {:.3}s", elapsed);

    Ok(())
}

/// Apply the One-Click Voice Cleanup chain.
pub fn cleanup(input: &Path, output: &Path, mains_freq: f32) -> Result<()> {
    let started = Instant::now();
    let mut buffer = decode(input).with_context(|| format!("decode {}", input.display()))?;
    let initial_peak = buffer.peak();
    let initial_lufs = measure_lufs(buffer.samples(), buffer.sample_rate(), buffer.channels());

    let mut chain = VoiceCleanup::build(mains_freq);
    chain.prepare(buffer.sample_rate(), buffer.channels());
    chain.process(buffer.samples_mut());

    let settings = ExportSettings::wav(WavBitDepth::Int24);
    let report = export(output, &buffer, &settings)
        .with_context(|| format!("export {}", output.display()))?;

    let final_lufs = measure_lufs(buffer.samples(), buffer.sample_rate(), buffer.channels());

    println!("Voice Cleanup:  {} → {}", input.display(), output.display());
    println!(
        "stages:         HighPass(80Hz) → DeHum({mains_freq:.0}Hz) → DeClick → Gate → EQ → DeEsser → Compressor → Limiter"
    );
    println!(
        "before:         {:.2} LUFS · peak {:.4}",
        initial_lufs, initial_peak
    );
    println!(
        "after:          {:.2} LUFS · peak {:.4} · rms {:.4}",
        final_lufs, report.peak, report.rms
    );
    println!("elapsed:        {:.3}s", started.elapsed().as_secs_f64());

    Ok(())
}

/// Apply single DSP effect to a file.
pub fn process_effect(
    input: &Path,
    output: &Path,
    effect_name: &str,
    mut effect: Box<dyn Effect>,
) -> Result<()> {
    let started = Instant::now();
    let mut buffer = decode(input).with_context(|| format!("decode {}", input.display()))?;

    effect.prepare(buffer.sample_rate(), buffer.channels());
    effect.process(buffer.samples_mut());

    let settings = ExportSettings::wav(WavBitDepth::Int24);
    let report = export(output, &buffer, &settings)
        .with_context(|| format!("export {}", output.display()))?;

    println!("processed:   {} → {}", input.display(), output.display());
    println!("effect:      {}", effect_name);
    println!("peak:        {:.4}", report.peak);
    println!("rms:         {:.4}", report.rms);
    println!("elapsed:     {:.3}s", started.elapsed().as_secs_f64());

    Ok(())
}
