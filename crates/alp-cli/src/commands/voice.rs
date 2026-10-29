//! Voice, model manager, and script rendering CLI commands.

use alp_media::{ExportSettings, WavBitDepth, export};
use alp_voice::client::{BuiltinAcousticBackend, InferClient};
use alp_voice::model::ModelId;
use alp_voice::{ModelManager, Script, TtsBackend, TtsRequest};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Synthesize speech from text and write the output WAV.
pub fn tts(text: &str, output: &Path, voice: &str, speed: f32) -> Result<()> {
    let started = Instant::now();
    let request = TtsRequest {
        text: text.to_string(),
        voice_id: voice.to_string(),
        speed,
    };

    // Try communicating with alp-infer over Unix socket first, fall back to in-process acoustic
    let client = InferClient::new();
    let buffer = if client.is_alive() {
        println!("synthesizing via alp-infer worker...");
        client
            .synthesize(&request)
            .map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        let backend = BuiltinAcousticBackend::new();
        backend
            .synthesize(&request)
            .map_err(|e| anyhow::anyhow!("{e}"))?
    };

    let settings = ExportSettings::wav(WavBitDepth::Int24);
    let report = export(output, &buffer, &settings)
        .with_context(|| format!("failed to export {}", output.display()))?;

    let elapsed = started.elapsed().as_secs_f64();
    println!("TTS synthesis:    {}", output.display());
    println!("voice:            {voice}");
    println!("speed:            {speed:.2}x");
    println!(
        "format:           {} Hz · {} ch",
        buffer.sample_rate(),
        buffer.channels()
    );
    println!(
        "duration:         {:.3}s ({} frames)",
        buffer.duration_secs(),
        buffer.frames()
    );
    println!("peak / rms:       {:.4} / {:.4}", report.peak, report.rms);
    println!(
        "elapsed:          {:.3}s (RTF: {:.2})",
        elapsed,
        elapsed / buffer.duration_secs().max(0.01)
    );

    Ok(())
}

/// List all models in the catalogue.
pub fn model_list(json: bool) -> Result<()> {
    let manager = ModelManager::new();
    let catalogue = manager.catalogue();

    if json {
        let json_str = serde_json::to_string_pretty(catalogue)?;
        println!("{json_str}");
        return Ok(());
    }

    println!(
        "{:<20} {:<12} {:<12} {:<10} {:<10}",
        "ID", "KIND", "SIZE", "LICENSE", "STATUS"
    );
    println!("{:-<70}", "");
    for m in catalogue {
        let status = manager.status(&m.id);
        let status_str = match status {
            alp_voice::ModelStatus::NotInstalled => "available",
            alp_voice::ModelStatus::Downloading { .. } => "downloading",
            alp_voice::ModelStatus::Installed { .. } => "installed",
            alp_voice::ModelStatus::Verified { .. } => "verified",
            alp_voice::ModelStatus::Corrupt { .. } => "corrupt",
        };
        let size_mb = m.total_size_bytes() as f64 / (1024.0 * 1024.0);
        let size_str = if m.is_builtin {
            "embedded".into()
        } else {
            format!("{size_mb:.1} MB")
        };
        let lic_accepted = if m.license.accepted {
            "accepted"
        } else {
            "pending"
        };
        println!(
            "{:<20} {:<12?} {:<12} {:<10} {:<10}",
            m.id.as_str(),
            m.kind,
            size_str,
            lic_accepted,
            status_str
        );
    }

    Ok(())
}

/// Verify SHA-256 checksums of an installed model.
pub fn model_verify(model_str: &str) -> Result<()> {
    let manager = ModelManager::new();
    let id = ModelId::new(model_str);

    let status = manager.verify(&id)?;
    match status {
        alp_voice::ModelStatus::Verified { path } => {
            println!("model {model_str}: VERIFIED ({})", path.display());
            Ok(())
        }
        alp_voice::ModelStatus::Corrupt { reason } => {
            bail!("model {model_str}: CORRUPT: {reason}");
        }
        alp_voice::ModelStatus::NotInstalled => {
            bail!("model {model_str} is not installed locally");
        }
        _ => bail!("unexpected model status for {model_str}"),
    }
}

/// Accept the license terms for a model.
pub fn model_accept_license(model_str: &str) -> Result<()> {
    let mut manager = ModelManager::new();
    let id = ModelId::new(model_str);
    manager.accept_license(&id)?;
    println!("model {model_str}: license terms accepted");
    Ok(())
}

/// Human-readable byte count (e.g. `12.3 MB`).
fn human_bytes(value: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const KIB: f64 = 1024.0;
    let v = value as f64;
    if v >= MIB {
        format!("{:.1} MB", v / MIB)
    } else if v >= KIB {
        format!("{:.1} KB", v / KIB)
    } else {
        format!("{value} B")
    }
}

/// Download model files from their manifest URLs (resumable, SHA-256 verified).
pub fn model_download(model_str: &str, offline_flag: bool) -> Result<()> {
    let mut manager = ModelManager::new();
    if offline_flag || std::env::var("ALP_OFFLINE").is_ok_and(|v| v == "1") {
        manager.set_offline(true);
    }
    let id = ModelId::new(model_str);

    let Some(manifest) = manager.get_manifest(&id) else {
        bail!("unknown model {model_str}; run `alp-cli model list` for the catalogue");
    };
    if manifest.is_builtin {
        println!("model {model_str}: built into the application; there is nothing to download");
        return Ok(());
    }

    let downloader = alp_voice::ModelDownloader::new();
    let outcome = downloader.download(&manager, &id, &mut |progress| {
        eprint!(
            "\rdownloading {} / {} {} ({} of {})",
            progress.file_index + 1,
            progress.file_count,
            progress.file_name,
            human_bytes(progress.overall_done),
            human_bytes(progress.overall_total),
        );
    });

    match outcome {
        Ok(report) => {
            eprintln!(); // finish the in-place progress line
            if report.bytes_downloaded == 0 {
                println!(
                    "model {model_str}: already complete ({} file(s) reused)",
                    report.files_skipped.len()
                );
            } else {
                println!(
                    "model {model_str}: fetched {} file(s) ({}), reused {} file(s)",
                    report.files_fetched.len(),
                    human_bytes(report.bytes_downloaded),
                    report.files_skipped.len()
                );
            }
        }
        Err(alp_voice::DownloadError::ShortStream {
            expected, received, ..
        }) => {
            bail!(
                "download of {model_str} ended early ({received} of {expected} bytes); \
                 partial data was kept - rerun the same command to resume"
            );
        }
        Err(alp_voice::DownloadError::LicenseNotAccepted { id: pending }) => {
            bail!(
                "the license for {pending} must be accepted first; \
                 run `alp-cli model accept-license {pending}`"
            );
        }
        Err(err) => bail!("download of {model_str} failed: {err}"),
    }

    // Verify in the same breath so no second command is needed to learn whether
    // the install is usable.
    match manager.verify(&id)? {
        alp_voice::ModelStatus::Verified { path } => {
            println!("model {model_str}: VERIFIED ({})", path.display());
            Ok(())
        }
        alp_voice::ModelStatus::Corrupt { reason } => {
            bail!("model {model_str}: CORRUPT after download: {reason}");
        }
        _ => bail!("model {model_str} did not reach a verified state after download"),
    }
}

/// Render a JSON script to audio (stitched single file or folder of stem files).
pub fn script_render(script_path: &Path, output: &Path, stitch: bool) -> Result<()> {
    let text = fs::read_to_string(script_path)
        .with_context(|| format!("cannot read script file {}", script_path.display()))?;
    let mut script: Script = serde_json::from_str(&text)
        .with_context(|| format!("invalid script JSON {}", script_path.display()))?;

    let backend = BuiltinAcousticBackend::new();

    println!(
        "rendering script: {} ({} lines)",
        script.title,
        script.lines.len()
    );
    for line in &mut script.lines {
        let req = TtsRequest {
            text: line.text.clone(),
            voice_id: line.voice_id.clone(),
            speed: line.speed,
        };
        let buf = backend
            .synthesize(&req)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        line.set_audio(buf);
    }

    if stitch {
        let combined = script.stitch(0.3)?;
        let settings = ExportSettings::wav(WavBitDepth::Int24);
        export(output, &combined, &settings)?;
        println!(
            "stitched output:  {} ({:.2}s)",
            output.display(),
            combined.duration_secs()
        );
    } else {
        let stems = script.export_stems(output)?;
        println!(
            "exported {} line stems to {}",
            stems.len(),
            output.display()
        );
    }

    Ok(())
}
