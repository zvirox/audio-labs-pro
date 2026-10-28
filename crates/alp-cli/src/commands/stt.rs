//! STT transcription and VAD segmentation CLI commands.

use alp_media::decode;
use alp_voice::client::InferClient;
use alp_voice::{BuiltinSttBackend, SttBackend, Transcript, VadConfig, segment_speech};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Transcribe speech audio into text, SRT, or WebVTT.
pub fn transcribe(input: &Path, output: Option<&Path>, format: &str, json: bool) -> Result<()> {
    let started = Instant::now();
    let buffer = decode(input).with_context(|| format!("decode {}", input.display()))?;

    // Try communicating with alp-infer over Unix socket first, fall back to in-process STT
    let client = InferClient::new();
    let segments = if client.is_alive() {
        client
            .transcribe(&buffer)
            .map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        let backend = BuiltinSttBackend::new();
        backend
            .transcribe(&buffer)
            .map_err(|e| anyhow::anyhow!("{e}"))?
    };

    let transcript = Transcript { segments };
    let elapsed = started.elapsed().as_secs_f64();

    if json {
        let json_str = serde_json::to_string_pretty(&transcript)?;
        if let Some(out_path) = output {
            fs::write(out_path, &json_str)
                .with_context(|| format!("write to {}", out_path.display()))?;
            eprintln!("Wrote JSON transcript to {}", out_path.display());
        } else {
            println!("{json_str}");
        }
        return Ok(());
    }

    let rendered_text = match format.to_ascii_lowercase().as_str() {
        "srt" => transcript.to_srt(buffer.sample_rate()),
        "vtt" => transcript.to_vtt(buffer.sample_rate()),
        _ => transcript.to_plain_text(),
    };

    if let Some(out_path) = output {
        fs::write(out_path, &rendered_text)
            .with_context(|| format!("write to {}", out_path.display()))?;
        println!("Transcribed:   {}", input.display());
        println!("Output:        {}", out_path.display());
        println!("Format:        {}", format.to_ascii_lowercase());
        println!("Segments:      {}", transcript.segments.len());
        println!(
            "Audio duration:{:.2}s (RTF: {:.2})",
            buffer.duration_secs(),
            elapsed / buffer.duration_secs().max(0.01)
        );
    } else {
        print!("{rendered_text}");
    }

    Ok(())
}

/// Run Voice Activity Detection on an audio file and output speech intervals.
pub fn vad(
    input: &Path,
    threshold_db: f32,
    hangover_ms: u32,
    min_speech_ms: u32,
    json: bool,
) -> Result<()> {
    let buffer = decode(input).with_context(|| format!("decode {}", input.display()))?;

    let config = VadConfig {
        threshold_db,
        hangover_ms,
        min_speech_ms,
        frame_size_ms: 20,
    };

    let segments = segment_speech(&buffer, &config);
    let sample_rate = buffer.sample_rate() as f64;

    if json {
        let intervals: Vec<_> = segments
            .iter()
            .map(|range| {
                let start_secs = range.start as f64 / sample_rate;
                let end_secs = range.end as f64 / sample_rate;
                serde_json::json!({
                    "start_frame": range.start,
                    "end_frame": range.end,
                    "start_secs": start_secs,
                    "end_secs": end_secs,
                    "duration_secs": end_secs - start_secs,
                })
            })
            .collect();

        let obj = serde_json::json!({
            "path": input.display().to_string(),
            "sample_rate": buffer.sample_rate(),
            "total_frames": buffer.frames(),
            "duration_secs": buffer.duration_secs(),
            "speech_segments": intervals,
        });

        println!("{}", serde_json::to_string_pretty(&obj)?);
        return Ok(());
    }

    println!("Voice Activity Detection: {}", input.display());
    println!(
        "Config: threshold = {:.1} dBFS, hangover = {} ms, min_speech = {} ms",
        threshold_db, hangover_ms, min_speech_ms
    );
    println!("Detected speech intervals: {}", segments.len());

    let mut total_speech_frames = 0u64;
    for (i, range) in segments.iter().enumerate() {
        let start_secs = range.start as f64 / sample_rate;
        let end_secs = range.end as f64 / sample_rate;
        let dur_secs = end_secs - start_secs;
        total_speech_frames += range.end.saturating_sub(range.start);

        println!(
            "  [{i:02}] {:02}:{:05.2} -> {:02}:{:05.2} (duration: {:.2}s, frames: {}..{})",
            (start_secs / 60.0) as u32,
            start_secs % 60.0,
            (end_secs / 60.0) as u32,
            end_secs % 60.0,
            dur_secs,
            range.start,
            range.end
        );
    }

    let speech_secs = total_speech_frames as f64 / sample_rate;
    let pct = (speech_secs / buffer.duration_secs().max(0.001)) * 100.0;
    println!(
        "Total speech: {:.2}s / {:.2}s ({:.1}%)",
        speech_secs,
        buffer.duration_secs(),
        pct
    );

    Ok(())
}
