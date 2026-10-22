//! Media-oriented commands: generate, decode and peaks.

use std::path::{Path, PathBuf};
use std::time::Instant;

use alp_media::{CacheStatus, PeakCache, compare, decode, probe, resample, write_test_tone};
use anyhow::{Context, Result, bail};

/// Generate a deterministic WAV fixture.
pub fn generate(
    output: &Path,
    seconds: f64,
    sample_rate: u32,
    channels: u16,
    frequency: f32,
    amplitude: f32,
) -> Result<()> {
    let started = Instant::now();
    write_test_tone(output, seconds, sample_rate, channels, frequency, amplitude)
        .with_context(|| format!("generate {}", output.display()))?;
    let bytes = std::fs::metadata(output)
        .with_context(|| format!("stat {}", output.display()))?
        .len();
    println!(
        "generated {} · {:.2}s · {} Hz · {} ch · {:.1} MiB · {:.2}s",
        output.display(),
        seconds,
        sample_rate,
        channels,
        bytes as f64 / (1024.0 * 1024.0),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

/// Decode a file and print information and levels.
pub fn decode_file(input: &Path, json: bool) -> Result<()> {
    let started = Instant::now();
    let info = probe(input).with_context(|| format!("probe {}", input.display()))?;
    let buffer = decode(input).with_context(|| format!("decode {}", input.display()))?;
    let elapsed = started.elapsed().as_secs_f64();

    if json {
        let value = serde_json::json!({
            "path": input.display().to_string(),
            "sample_rate": buffer.sample_rate(),
            "channels": buffer.channels(),
            "frames": buffer.frames(),
            "duration_secs": buffer.duration_secs(),
            "codec": info.codec,
            "peak": buffer.peak(),
            "rms": buffer.rms(),
            "decode_secs": elapsed,
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!("file:         {}", input.display());
    println!("codec:        {}", info.codec);
    println!("sample rate:  {} Hz", buffer.sample_rate());
    println!("channels:     {}", buffer.channels());
    println!("frames:       {}", buffer.frames());
    println!("duration:     {:.3}s", buffer.duration_secs());
    println!("peak:         {:.4}", buffer.peak());
    println!("rms:          {:.4}", buffer.rms());
    println!("decoded in:   {elapsed:.3}s");
    Ok(())
}

/// Build or load a peak cache for a file.
pub fn peaks(input: &Path, cache_dir: Option<PathBuf>, json: bool) -> Result<()> {
    let directory = cache_dir.unwrap_or_else(|| default_cache_dir(input));
    let cache = PeakCache::new(&directory);
    let started = Instant::now();
    let (pyramid, status, cache_path) = cache
        .load_or_build(input)
        .with_context(|| format!("peaks {}", input.display()))?;
    let elapsed = started.elapsed().as_secs_f64();
    let status_label = match status {
        CacheStatus::Hit => "hit",
        CacheStatus::Miss => "miss",
    };

    if json {
        let levels: Vec<_> = pyramid
            .levels()
            .iter()
            .map(|level| {
                serde_json::json!({
                    "bucket_samples": level.bucket_samples(),
                    "buckets": level.len(),
                })
            })
            .collect();
        let value = serde_json::json!({
            "path": input.display().to_string(),
            "cache_path": cache_path.display().to_string(),
            "status": status_label,
            "sample_rate": pyramid.sample_rate(),
            "frames": pyramid.frames(),
            "duration_secs": pyramid.frames() as f64 / f64::from(pyramid.sample_rate()),
            "levels": levels,
            "elapsed_secs": elapsed,
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!("file:         {}", input.display());
    println!("cache:        {} ({status_label})", cache_path.display());
    println!("sample rate:  {} Hz", pyramid.sample_rate());
    println!("frames:       {}", pyramid.frames());
    println!(
        "duration:     {:.3}s",
        pyramid.frames() as f64 / f64::from(pyramid.sample_rate())
    );
    for (index, level) in pyramid.levels().iter().enumerate() {
        println!(
            "level {index}:      {} frames/bucket · {} buckets",
            level.bucket_samples(),
            level.len()
        );
    }
    println!("elapsed:      {elapsed:.3}s");
    Ok(())
}

/// Compare an export with a reference render.
pub fn null_test(
    reference_path: &Path,
    rendered_path: &Path,
    tolerance: f32,
    json: bool,
) -> Result<()> {
    let reference =
        decode(reference_path).with_context(|| format!("decode {}", reference_path.display()))?;
    let mut rendered =
        decode(rendered_path).with_context(|| format!("decode {}", rendered_path.display()))?;
    if rendered.sample_rate() != reference.sample_rate() {
        rendered = resample(&rendered, reference.sample_rate()).with_context(|| {
            format!(
                "resample {} to {} Hz",
                rendered_path.display(),
                reference.sample_rate()
            )
        })?;
    }
    let report = compare(&reference, &rendered).context("compare buffers")?;
    let passed = report.passed(tolerance);
    if json {
        let value = serde_json::json!({
            "reference": reference_path.display().to_string(),
            "rendered": rendered_path.display().to_string(),
            "reference_frames": report.reference_frames,
            "rendered_frames": report.rendered_frames,
            "compared_frames": report.compared_frames,
            "max_abs_diff": report.max_abs_diff,
            "max_abs_diff_db": if report.max_abs_diff > 0.0 {
                f64::from(report.max_abs_diff).log10() * 20.0
            } else {
                f64::NEG_INFINITY
            },
            "rms_diff": report.rms_diff,
            "tolerance": tolerance,
            "passed": passed,
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("reference:    {}", reference_path.display());
        println!("rendered:     {}", rendered_path.display());
        println!(
            "frames:       {} / {}",
            report.compared_frames, report.reference_frames
        );
        println!("max diff:     {:.8}", report.max_abs_diff);
        println!("rms diff:     {:.8}", report.rms_diff);
        println!("tolerance:    {tolerance:.8}");
        println!("result:       {}", if passed { "PASS" } else { "FAIL" });
    }
    if !passed {
        bail!("null test failed: max difference exceeds tolerance or lengths differ");
    }
    Ok(())
}

fn default_cache_dir(input: &Path) -> PathBuf {
    input
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("peaks")
}
