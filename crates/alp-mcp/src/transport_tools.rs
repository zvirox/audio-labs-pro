//! Tool definitions for live transport control.
//!
//! Kept in a separate module so `tool_defs.rs` stays under the 1500-line
//! guard. These tools are meaningful only when `alp-mcp --attach` is connected
//! to a running desktop app; headless dispatch returns an explicit error.

use alp_api::PermissionTier;
use serde_json::json;

use crate::protocol::McpTool;

/// All live transport tool names.
pub(crate) const TRANSPORT_TOOLS: &[&str] = &[
    "transport_state",
    "transport_play",
    "transport_pause",
    "transport_stop",
    "transport_seek",
    "transport_set_volume",
    "transport_set_loop",
];

/// Build the transport tool schemas for the requested permission tier.
pub(crate) fn definitions(tier: PermissionTier) -> Vec<McpTool> {
    let mut tools = vec![McpTool {
        name: "transport_state".into(),
        description: "Read the live transport state (playing, position, duration, loop, volume and meter levels). Requires a running desktop app with AI control attached.".into(),
        input_schema: json!({ "type": "object", "properties": {} }),
    }];
    if tier >= PermissionTier::Edit {
        tools.extend([
            McpTool {
                name: "transport_play".into(),
                description: "Start playback on the live desktop transport. Requires an attached desktop app.".into(),
                input_schema: json!({ "type": "object", "properties": {} }),
            },
            McpTool {
                name: "transport_pause".into(),
                description: "Pause the live desktop transport, preserving the playhead.".into(),
                input_schema: json!({ "type": "object", "properties": {} }),
            },
            McpTool {
                name: "transport_stop".into(),
                description: "Stop the live desktop transport and rewind to the loop or project start.".into(),
                input_schema: json!({ "type": "object", "properties": {} }),
            },
            McpTool {
                name: "transport_seek".into(),
                description: "Move the live playhead to a timestamp in seconds.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "position_secs": { "type": "number", "description": "Target position in seconds" }
                    },
                    "required": ["position_secs"]
                }),
            },
            McpTool {
                name: "transport_set_volume".into(),
                description: "Set the live master volume (clamped to 0.0..=1.5).".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "volume": { "type": "number" }
                    },
                    "required": ["volume"]
                }),
            },
            McpTool {
                name: "transport_set_loop".into(),
                description: "Enable or clear the live transport loop range.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "enabled": { "type": "boolean" },
                        "start_secs": { "type": "number" },
                        "end_secs": { "type": "number" }
                    },
                    "required": ["enabled", "start_secs", "end_secs"]
                }),
            },
        ]);
    }
    tools
}
