//! System hardware detection for CPU, RAM, and GPU capability assessment.
//!
//! Recommends appropriate model tiers (Fast vs Best) and issues clear warnings
//! before downloading large neural weights.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Recommended model complexity tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelTier {
    /// Fast lightweight models suitable for lower-spec hardware or rapid iteration.
    Fast,
    /// High-fidelity neural models requiring more memory and compute.
    Best,
}

impl ModelTier {
    /// Friendly tier name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "Fast (Lightweight)",
            Self::Best => "Best (Neural Quality)",
        }
    }
}

/// System hardware capabilities and memory profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareProfile {
    /// Total system RAM in bytes.
    pub total_ram_bytes: u64,
    /// Available system RAM in bytes.
    pub available_ram_bytes: u64,
    /// Number of logical CPU cores.
    pub cpu_cores: usize,
    /// Whether an acceleration GPU / DRM render node was detected.
    pub gpu_detected: bool,
    /// Descriptive string for detected GPU / accelerator.
    pub gpu_description: Option<String>,
    /// Recommended model tier based on available compute and memory.
    pub recommended_tier: ModelTier,
}

impl HardwareProfile {
    /// Probe the current system hardware environment (Linux-specific with fallback).
    #[must_use]
    pub fn probe() -> Self {
        let (total_ram, avail_ram) = probe_linux_memory();
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let (gpu_detected, gpu_description) = probe_gpu();

        // 16 GB+ RAM and at least 8 cores or GPU qualifies for "Best"
        let recommended_tier =
            if total_ram >= 16 * 1024 * 1024 * 1024 && (cpu_cores >= 8 || gpu_detected) {
                ModelTier::Best
            } else {
                ModelTier::Fast
            };

        Self {
            total_ram_bytes: total_ram,
            available_ram_bytes: avail_ram,
            cpu_cores,
            gpu_detected,
            gpu_description,
            recommended_tier,
        }
    }

    /// Total system RAM formatted in gigabytes.
    #[must_use]
    pub fn total_ram_gb(&self) -> f64 {
        self.total_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Available system RAM formatted in gigabytes.
    #[must_use]
    pub fn available_ram_gb(&self) -> f64 {
        self.available_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Check whether a model can be safely run without memory exhaustion warnings.
    #[must_use]
    pub fn evaluate_model_fit(&self, total_size_bytes: u64) -> Option<String> {
        let size_gb = total_size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let avail_gb = self.available_ram_gb();

        // Model weights usually expand ~1.5x in working RAM during runtime
        let estimated_working_ram_gb = size_gb * 1.5;

        if estimated_working_ram_gb > avail_gb {
            Some(format!(
                "Warning: Model requires ~{estimated_working_ram_gb:.1} GB working RAM, but only {avail_gb:.1} GB is available. System may experience paging."
            ))
        } else if self.recommended_tier == ModelTier::Fast && size_gb > 0.5 {
            Some(format!(
                "Notice: Heavy model ({size_gb:.1} GB) on a machine recommended for Fast tier. Synthesis may be slower than real-time on CPU."
            ))
        } else {
            None
        }
    }
}

/// Read total and available memory from `/proc/meminfo`.
fn probe_linux_memory() -> (u64, u64) {
    let Ok(content) = fs::read_to_string("/proc/meminfo") else {
        // Fallback default (8 GB)
        let default_mem = 8 * 1024 * 1024 * 1024;
        return (default_mem, default_mem / 2);
    };

    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total_kb = parse_meminfo_kb(rest);
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            avail_kb = parse_meminfo_kb(rest);
        }
    }

    if total_kb == 0 {
        total_kb = 8 * 1024 * 1024;
    }
    if avail_kb == 0 {
        avail_kb = total_kb / 2;
    }

    (total_kb * 1024, avail_kb * 1024)
}

fn parse_meminfo_kb(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|val| val.parse::<u64>().ok())
        .unwrap_or(0)
}

/// Probe GPU or acceleration presence via Linux DRM or NVIDIA driver nodes.
fn probe_gpu() -> (bool, Option<String>) {
    // Check NVIDIA proc entry
    if Path::new("/proc/driver/nvidia/version").exists() {
        return (true, Some("NVIDIA Proprietary Driver".into()));
    }

    // Check DRM render nodes
    if let Ok(entries) = fs::read_dir("/dev/dri") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("renderD") {
                return (true, Some(format!("DRM Accelerator ({name_str})")));
            }
        }
    }

    (false, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_valid_hardware_profile() {
        let profile = HardwareProfile::probe();
        assert!(profile.total_ram_bytes > 0);
        assert!(profile.available_ram_bytes > 0);
        assert!(profile.cpu_cores > 0);
    }

    #[test]
    fn evaluates_model_warnings() {
        let profile = HardwareProfile {
            total_ram_bytes: 8 * 1024 * 1024 * 1024,
            available_ram_bytes: 2 * 1024 * 1024 * 1024, // 2 GB available
            cpu_cores: 4,
            gpu_detected: false,
            gpu_description: None,
            recommended_tier: ModelTier::Fast,
        };

        // 3 GB model requires ~4.5 GB working RAM -> Warning
        let warning = profile.evaluate_model_fit(3 * 1024 * 1024 * 1024);
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("Warning: Model requires"));

        // 50 MB model fits easily
        let no_warn = profile.evaluate_model_fit(50 * 1024 * 1024);
        assert!(no_warn.is_none());
    }
}
