//! Voice profile management, reference audio auditing, and voice design CLI commands.

use alp_media::decode;
use alp_voice::client::InferClient;
use alp_voice::{
    HardwareProfile, ProfileStore, ReferenceQualityReport, VoiceDesignRequest,
    analyze_reference_quality, compare_audio, design_voice,
};
use anyhow::{Context, Result, bail};
use std::path::Path;

/// List all installed voice profiles.
pub fn list(json: bool) -> Result<()> {
    let store = ProfileStore::default_store().context("failed to open profile store")?;
    let profiles = store.list_profiles();

    if json {
        println!("{}", serde_json::to_string_pretty(&profiles)?);
        return Ok(());
    }

    println!("Voice Profiles ({} installed):", profiles.len());
    println!(
        "{:<16} {:<12} {:<10} {:<24} TAGS",
        "ID", "NAME", "KIND", "BACKEND"
    );
    println!("{}", "-".repeat(70));

    for p in &profiles {
        let kind_str = match p.kind {
            alp_voice::VoiceKind::Builtin => "Builtin",
            alp_voice::VoiceKind::Cloned => "Cloned",
            alp_voice::VoiceKind::Designed => "Designed",
        };
        let tags_str = p.tags.join(", ");
        println!(
            "{:<16} {:<12} {:<10} {:<24} {}",
            p.id, p.name, kind_str, p.backend, tags_str
        );
    }
    Ok(())
}

/// Print detailed metadata for a single voice profile.
pub fn info(id: &str, json: bool) -> Result<()> {
    let store = ProfileStore::default_store().context("failed to open profile store")?;
    let profile = store
        .get_profile(id)
        .ok_or_else(|| anyhow::anyhow!("voice profile '{id}' not found"))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&profile)?);
        return Ok(());
    }

    println!("Voice Profile: {}", profile.name);
    println!("ID:            {}", profile.id);
    println!("Kind:          {:?}", profile.kind);
    println!("Backend:       {}", profile.backend);
    println!("Tags:          {}", profile.tags.join(", "));
    println!(
        "Parameters:    speed={:.2}x, pitch={:+.1} semitones, energy={:.2}, emotion={}",
        profile.default_params.speed,
        profile.default_params.pitch,
        profile.default_params.energy,
        profile.default_params.emotion
    );

    if let Some(consent) = &profile.consent {
        println!("Consent Record:");
        println!("  Speaker:     {}", consent.speaker_name);
        println!("  Attested By: {}", consent.attested_by);
        println!("  Timestamp:   {}", consent.timestamp);
        println!("  Statement:   {}", consent.statement);
    } else if profile.kind == alp_voice::VoiceKind::Cloned {
        println!("Consent:       WARNING: Missing consent attestation");
    } else {
        println!("Consent:       N/A (Built-in or Designed)");
    }

    Ok(())
}

/// Audit reference recording audio against cloning quality gates.
pub fn check_reference(input: &Path, json: bool) -> Result<()> {
    let buffer = decode(input).with_context(|| format!("decode {}", input.display()))?;
    let report = analyze_reference_quality(&buffer);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("Reference Audio Quality Audit: {}", input.display());
    println!(
        "Duration:      {:.2}s (minimum: {:.1}s)",
        report.duration_secs,
        ReferenceQualityReport::MIN_DURATION_SECS
    );
    println!(
        "Sample Rate:   {} Hz · {} channels",
        report.sample_rate, report.channels
    );
    println!("Peak Level:    {:.1} dBFS", report.peak_dbfs);
    println!("RMS Energy:    {:.1} dBFS", report.rms_dbfs);
    println!(
        "Noise Floor:   {:.1} dBFS (ceiling: {:.1} dBFS)",
        report.noise_floor_dbfs,
        ReferenceQualityReport::MAX_NOISE_FLOOR_DBFS
    );
    println!(
        "Silence Ratio: {:.1}% (ceiling: {:.0}%)",
        report.silence_ratio * 100.0,
        ReferenceQualityReport::MAX_SILENCE_RATIO * 100.0
    );
    println!(
        "Clipping:      {}",
        if report.clipping_detected {
            "FAILED (digital full-scale spikes detected)"
        } else {
            "Passed (no clipping)"
        }
    );
    println!(
        "Status:        {}",
        if report.passed {
            "PASSED (Ready for cloning)"
        } else {
            "FAILED"
        }
    );

    if !report.advice.is_empty() {
        println!("\nActionable Recommendations:");
        for adv in &report.advice {
            println!("  • {adv}");
        }
    }

    if !report.passed {
        bail!("Reference audio did not meet quality requirements");
    }

    Ok(())
}

/// Design a new voice profile from a natural language brief.
pub fn design(name: &str, prompt: &str, tags: Vec<String>, json: bool) -> Result<()> {
    let request = VoiceDesignRequest {
        name: name.to_string(),
        prompt: prompt.to_string(),
        tags,
    };

    // Try alp-infer daemon first, fall back to in-process design
    let client = InferClient::new();
    let profile = if client.is_alive() {
        client
            .design_voice(&request)
            .map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        design_voice(&request)
    };

    let store = ProfileStore::default_store().context("failed to open profile store")?;
    store.save_profile(&profile)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&profile)?);
        return Ok(());
    }

    println!("Designed Voice Profile: {}", profile.name);
    println!("ID:          {}", profile.id);
    println!(
        "Parameters:  speed={:.2}x, pitch={:+.1} st, energy={:.2}, emotion={}",
        profile.default_params.speed,
        profile.default_params.pitch,
        profile.default_params.energy,
        profile.default_params.emotion
    );
    println!("Tags:        {}", profile.tags.join(", "));
    println!(
        "Stored at:   {}",
        store.root_dir().join(&profile.id).display()
    );
    Ok(())
}

/// Export a voice profile into a `.alpvoice` package.
pub fn export_profile(id: &str, output: &Path) -> Result<()> {
    let store = ProfileStore::default_store().context("failed to open profile store")?;
    store
        .export_bundle(id, output)
        .with_context(|| format!("export profile '{id}' to {}", output.display()))?;

    println!("Exported profile '{id}' to {}", output.display());
    Ok(())
}

/// Import a voice profile from a `.alpvoice` package.
pub fn import_profile(bundle_path: &Path, json: bool) -> Result<()> {
    let store = ProfileStore::default_store().context("failed to open profile store")?;
    let profile = store
        .import_bundle(bundle_path)
        .with_context(|| format!("import bundle from {}", bundle_path.display()))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&profile)?);
        return Ok(());
    }

    println!("Successfully imported voice profile: {}", profile.name);
    println!("ID:      {}", profile.id);
    println!("Kind:    {:?}", profile.kind);
    println!("Backend: {}", profile.backend);
    Ok(())
}

/// Compare reference audio against generated speech audio.
pub fn compare(reference: &Path, generated: &Path, json: bool) -> Result<()> {
    let ref_buf = decode(reference).with_context(|| format!("decode {}", reference.display()))?;
    let gen_buf = decode(generated).with_context(|| format!("decode {}", generated.display()))?;

    let comp = compare_audio(&ref_buf, &gen_buf);

    if json {
        println!("{}", serde_json::to_string_pretty(&comp)?);
        return Ok(());
    }

    println!("Voice A/B Acoustic Comparison:");
    println!(
        "Reference:   {} ({:.2}s, {:.1} dBFS)",
        reference.display(),
        comp.ref_duration_secs,
        comp.ref_rms_dbfs
    );
    println!(
        "Generated:   {} ({:.2}s, {:.1} dBFS)",
        generated.display(),
        comp.gen_duration_secs,
        comp.gen_rms_dbfs
    );
    println!("Duration Δ:  {:+.2}s", comp.duration_diff_secs);
    println!("Level Δ:     {:+.1} dB", comp.level_diff_db);
    println!("ZCR (Ref/Gen): {:.4} / {:.4}", comp.ref_zcr, comp.gen_zcr);
    println!("Similarity:  {:.1}%", comp.similarity_score * 100.0);
    Ok(())
}

/// Delete a voice profile from the local library.
pub fn delete(id: &str) -> Result<()> {
    let store = ProfileStore::default_store().context("failed to open profile store")?;
    if store.delete_profile(id)? {
        println!("Deleted voice profile '{id}'");
    } else {
        println!("Voice profile '{id}' does not exist");
    }
    Ok(())
}

/// Probe and display system hardware capabilities and model tier recommendations.
pub fn hardware_probe(json: bool) -> Result<()> {
    let profile = HardwareProfile::probe();

    if json {
        println!("{}", serde_json::to_string_pretty(&profile)?);
        return Ok(());
    }

    println!("Hardware Capability Assessment:");
    println!("Total RAM:       {:.2} GB", profile.total_ram_gb());
    println!("Available RAM:   {:.2} GB", profile.available_ram_gb());
    println!("CPU Cores:       {}", profile.cpu_cores);
    println!(
        "GPU Detected:    {}",
        if profile.gpu_detected {
            profile.gpu_description.as_deref().unwrap_or("Yes")
        } else {
            "No dedicated accelerator detected"
        }
    );
    println!("Recommended:     {}", profile.recommended_tier.as_str());

    Ok(())
}
