//! CLI commands for scanning, listing, and inspecting audio plugins (CLAP & LV2).

use alp_plugin::{PluginHost, PluginScanner};
use anyhow::{Context, Result};
use std::path::Path;

/// Scan for installed CLAP and LV2 plugins and update cache.
pub fn scan(custom_path: Option<&Path>, json: bool) -> Result<()> {
    let mut scanner = PluginScanner::new();
    if let Some(path) = custom_path {
        scanner.add_search_path(path);
    }
    let plugins = scanner.scan();

    if json {
        let serialized = serde_json::to_string_pretty(&plugins)
            .context("failed to serialize plugin scan results to JSON")?;
        println!("{serialized}");
    } else {
        println!("Discovered {} audio plugin(s):", plugins.len());
        for p in &plugins {
            println!(
                "  [{:?}] {:<24} v{:<8} ({}) -> {}",
                p.format,
                p.name,
                p.version,
                p.vendor,
                p.path.display()
            );
        }
    }
    Ok(())
}

/// List all cached and virtual audio plugins.
pub fn list(json: bool) -> Result<()> {
    let host = PluginHost::new();
    let plugins = host.list_plugins();

    if json {
        let serialized = serde_json::to_string_pretty(&plugins)
            .context("failed to serialize plugin list to JSON")?;
        println!("{serialized}");
    } else {
        println!("Available Audio Plugins ({} total):", plugins.len());
        println!(
            "{:<6} {:<28} {:<10} {:<16} PATH",
            "FORMAT", "NAME", "CATEGORY", "ID"
        );
        println!("{}", "-".repeat(80));
        for p in &plugins {
            let cat = format!("{:?}", p.category);
            let fmt = format!("{:?}", p.format);
            println!(
                "{:<6} {:<28} {:<10} {:<16} {}",
                fmt,
                p.name,
                cat,
                p.id,
                p.path.display()
            );
        }
    }
    Ok(())
}

/// Inspect a specific audio plugin's parameters and properties.
pub fn inspect(plugin_id: &str, json: bool) -> Result<()> {
    let host = PluginHost::new();
    let plugins = host.list_plugins();
    let desc = plugins
        .iter()
        .find(|p| p.id == plugin_id || p.name.eq_ignore_ascii_case(plugin_id))
        .ok_or_else(|| anyhow::anyhow!("Plugin '{plugin_id}' not found"))?;

    let instance = host
        .instantiate(&desc.id, 48000)
        .context("failed to instantiate plugin for inspection")?;

    if json {
        let serialized = serde_json::to_string_pretty(&instance.parameters())
            .context("failed to serialize plugin parameters to JSON")?;
        println!("{serialized}");
    } else {
        println!("Plugin: {} (v{})", desc.name, desc.version);
        println!("Vendor: {}", desc.vendor);
        println!("Format: {:?}", desc.format);
        println!("Category: {:?}", desc.category);
        println!("Path: {}", desc.path.display());
        println!("Parameters ({}):", instance.parameters().len());
        for param in instance.parameters() {
            println!(
                "  - {} [id: {}]: default={:.2}, range=[{:.2} .. {:.2}]",
                param.name, param.id, param.default_value, param.min_value, param.max_value
            );
        }
    }
    Ok(())
}
