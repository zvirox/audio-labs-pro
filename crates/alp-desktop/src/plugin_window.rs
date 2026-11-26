//! Safe X11 bridge for hosting native plugin editor windows.
//!
//! All window operations use safe `x11rb` calls. The host window id is
//! obtained through `raw-window-handle`, mirroring `glass::probe`, and no
//! `unsafe` block is needed in this crate.

use std::collections::HashMap;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::{Deserialize, Serialize};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt};

/// One hosted plugin editor window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginWindowInfo {
    /// Stable plugin id from the host.
    pub plugin_id: String,
    /// X11 window id of the plugin editor.
    pub xid: u32,
}

/// Tracks native editor windows parented under the main window.
#[derive(Debug, Default)]
pub struct PluginWindowRegistry {
    host_xid: Option<u32>,
    editors: HashMap<String, u32>,
}

impl PluginWindowRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Store the host window id captured from the Tauri window handle.
    pub fn set_host_xid(&mut self, xid: u32) {
        self.host_xid = Some(xid);
    }

    /// Host window id, when the windowing system is X11.
    #[must_use]
    pub fn host_xid(&self) -> Option<u32> {
        self.host_xid
    }

    /// Record a plugin editor as hosted.
    pub fn register(&mut self, plugin_id: impl Into<String>, xid: u32) {
        self.editors.insert(plugin_id.into(), xid);
    }

    /// Forget a plugin editor.
    pub fn unregister(&mut self, plugin_id: &str) -> bool {
        self.editors.remove(plugin_id).is_some()
    }

    /// Snapshot of currently hosted editors.
    #[must_use]
    pub fn list(&self) -> Vec<PluginWindowInfo> {
        let mut entries: Vec<PluginWindowInfo> = self
            .editors
            .iter()
            .map(|(plugin_id, xid)| PluginWindowInfo {
                plugin_id: plugin_id.clone(),
                xid: *xid,
            })
            .collect();
        entries.sort_by(|a, b| a.plugin_id.cmp(&b.plugin_id));
        entries
    }
}

/// Extract an X11 window id from a Tauri window handle.
pub fn host_window_id<W: HasWindowHandle>(window: &W) -> Result<u32, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("cannot obtain window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Xlib(xlib) => {
            u32::try_from(xlib.window).map_err(|_| "Xlib window id exceeds 32 bits".to_owned())
        }
        RawWindowHandle::Xcb(xcb) => Ok(xcb.window.get()),
        _ => Err("window handle is not X11 (Xlib or Xcb)".to_owned()),
    }
}

/// Reparent and map a plugin editor under the host window.
///
/// The plugin id is recorded in `registry` only after X11 accepts the request.
pub fn embed_editor(
    registry: &mut PluginWindowRegistry,
    plugin_id: impl Into<String>,
    plugin_xid: u32,
    x: i16,
    y: i16,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let host_xid = registry
        .host_xid()
        .ok_or_else(|| "no X11 host window id was captured at startup".to_owned())?;
    let (connection, _screen) =
        x11rb::connect(None).map_err(|error| format!("cannot connect to X11: {error}"))?;
    connection
        .reparent_window(plugin_xid, host_xid, x, y)
        .map_err(|error| format!("reparent failed: {error}"))?
        .check()
        .map_err(|error| format!("reparent rejected: {error}"))?;
    connection
        .configure_window(
            plugin_xid,
            &ConfigureWindowAux {
                x: Some(i32::from(x)),
                y: Some(i32::from(y)),
                width: Some(width),
                height: Some(height),
                ..ConfigureWindowAux::default()
            },
        )
        .map_err(|error| format!("configure failed: {error}"))?
        .check()
        .map_err(|error| format!("configure rejected: {error}"))?;
    connection
        .map_window(plugin_xid)
        .map_err(|error| format!("map failed: {error}"))?
        .check()
        .map_err(|error| format!("map rejected: {error}"))?;
    connection
        .flush()
        .map_err(|error| format!("X11 flush failed: {error}"))?;
    registry.register(plugin_id, plugin_xid);
    Ok(())
}

/// Unmap and forget a hosted plugin editor.
pub fn release_editor(registry: &mut PluginWindowRegistry, plugin_id: &str) -> Result<(), String> {
    let xid = registry
        .list()
        .into_iter()
        .find(|window| window.plugin_id == plugin_id)
        .map(|window| window.xid)
        .ok_or_else(|| format!("plugin editor '{plugin_id}' is not hosted"))?;
    let (connection, _screen) =
        x11rb::connect(None).map_err(|error| format!("cannot connect to X11: {error}"))?;
    connection
        .unmap_window(xid)
        .map_err(|error| format!("unmap failed: {error}"))?
        .check()
        .map_err(|error| format!("unmap rejected: {error}"))?;
    connection
        .flush()
        .map_err(|error| format!("X11 flush failed: {error}"))?;
    registry.unregister(plugin_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_tracks_editors() {
        let mut registry = PluginWindowRegistry::new();
        assert!(registry.host_xid().is_none());
        registry.set_host_xid(42);
        assert_eq!(registry.host_xid(), Some(42));
        registry.register("vst3:Comp", 100);
        registry.register("clap:EQ", 200);
        let list = registry.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].plugin_id, "clap:EQ");
        assert!(registry.unregister("vst3:Comp"));
        assert!(!registry.unregister("vst3:Comp"));
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn embed_requires_a_host_window() {
        let mut registry = PluginWindowRegistry::new();
        let error =
            embed_editor(&mut registry, "clap:EQ", 100, 0, 0, 640, 360).expect_err("no host id");
        assert!(error.contains("no X11 host window id"));
    }
}
