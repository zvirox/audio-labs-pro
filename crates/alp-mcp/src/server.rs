//! Core Model Context Protocol (MCP) server dispatcher.

use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

use alp_api::{ApiSession, ExportPreset, PermissionTier};

use crate::protocol::{JsonRpcRequest, JsonRpcResponse, McpTool, ToolCallResult};

/// MCP Server handling JSON-RPC 2.0 requests over an active `ApiSession`.
pub struct McpServer {
    session: Arc<Mutex<ApiSession>>,
    tier: PermissionTier,
}

impl McpServer {
    /// Create a new server wrapping an active session.
    pub fn new(session: Arc<Mutex<ApiSession>>, tier: PermissionTier) -> Self {
        Self { session, tier }
    }

    /// Dispatch a JSON-RPC request and return a response.
    pub fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone().unwrap_or(Value::Null);

        match req.method.as_str() {
            "initialize" => self.handle_initialize(id),
            "server/discover" => self.handle_discover(id),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tool_call(id, req.params),
            "resources/list" => self.handle_resources_list(id),
            "resources/read" => self.handle_resources_read(id, req.params),
            "prompts/list" => self.handle_prompts_list(id),
            "prompts/get" => self.handle_prompts_get(id, req.params),
            _ => JsonRpcResponse::error(
                id,
                -32601,
                format!("method '{}' not found", req.method),
                None,
            ),
        }
    }

    fn handle_initialize(&self, id: Value) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            json!({
                "protocolVersion": "2026-07-28",
                "capabilities": {
                    "tools": { "listChanged": true },
                    "resources": { "subscribe": true },
                    "prompts": { "listChanged": true }
                },
                "serverInfo": {
                    "name": "audio-labs-pro",
                    "version": "1.1.0"
                }
            }),
        )
    }

    fn handle_discover(&self, id: Value) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            json!({
                "name": "Audio Labs Pro",
                "specRevision": "2026-07-28",
                "tier": format!("{:?}", self.tier),
                "features": ["tools", "resources", "prompts", "checkpoints", "undo_rollback"]
            }),
        )
    }

    fn handle_tools_list(&self, id: Value) -> JsonRpcResponse {
        let mut tools = vec![
            McpTool {
                name: "project_info".into(),
                description: "Get summary information about the active project (sample rate, tracks, clips, revision, duration)".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "track_list".into(),
                description: "List all tracks in the active project timeline with volume and pan state".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "analyze_loudness".into(),
                description: "Analyze integrated loudness (LUFS) and true peak level according to ITU-R BS.1770".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "plugin_list".into(),
                description: "List discovered CLAP and LV2 audio plugins installed on the host system".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ];

        if self.tier >= PermissionTier::Edit {
            tools.extend(vec![
                McpTool {
                    name: "plugin_insert".into(),
                    description: "Insert a CLAP or LV2 audio plugin onto a track's effect chain".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "track_id": { "type": "string", "description": "Track ID" },
                            "plugin_id": { "type": "string", "description": "CLAP ID, LV2 URI, or plugin identifier" }
                        },
                        "required": ["track_id", "plugin_id"]
                    }),
                },
                McpTool {
                    name: "plugin_param_set".into(),
                    description: "Set an audio parameter value on an inserted plugin".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "track_id": { "type": "string", "description": "Track ID" },
                            "effect_id": { "type": "string", "description": "Effect ID on the track" },
                            "param_name": { "type": "string", "description": "Parameter name (e.g. gain, drive, mix)" },
                            "value": { "type": "number", "description": "Parameter numeric value" }
                        },
                        "required": ["track_id", "effect_id", "param_name", "value"]
                    }),
                },
                McpTool {
                    name: "project_save".into(),
                    description: "Save the current project to disk immediately".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                McpTool {
                    name: "history_undo".into(),
                    description: "Revert the most recent edit command".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                McpTool {
                    name: "history_redo".into(),
                    description: "Reapply the most recently undone command".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                McpTool {
                    name: "checkpoint_create".into(),
                    description: "Create a named checkpoint of the project state for rollback".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "label": { "type": "string", "description": "Human-readable label" }
                        },
                        "required": ["label"]
                    }),
                },
                McpTool {
                    name: "checkpoint_restore".into(),
                    description: "Restore project state to a previously captured checkpoint".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "checkpoint_id": { "type": "string", "description": "Checkpoint handle" }
                        },
                        "required": ["checkpoint_id"]
                    }),
                },
                McpTool {
                    name: "rollback_session".into(),
                    description: "Rollback all edits made during the current AI session (restores session-start checkpoint)".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                McpTool {
                    name: "track_add".into(),
                    description: "Add a new track to the timeline".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "Track name" }
                        },
                        "required": ["name"]
                    }),
                },
                McpTool {
                    name: "track_remove".into(),
                    description: "Remove a track from the timeline by track ID".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "track_id": { "type": "string", "description": "Track handle" }
                        },
                        "required": ["track_id"]
                    }),
                },
                McpTool {
                    name: "clip_split".into(),
                    description: "Split an existing clip at a timeline timestamp in seconds".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "clip_id": { "type": "string", "description": "Clip handle" },
                            "split_secs": { "type": "number", "description": "Split timestamp in seconds" }
                        },
                        "required": ["clip_id", "split_secs"]
                    }),
                },
                McpTool {
                    name: "range_delete".into(),
                    description: "Delete an audio range and ripple cut subsequent audio".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "start_secs": { "type": "number", "description": "Range start in seconds" },
                            "end_secs": { "type": "number", "description": "Range end in seconds" }
                        },
                        "required": ["start_secs", "end_secs"]
                    }),
                },
            ]);
        }

        if self.tier >= PermissionTier::Export {
            tools.push(McpTool {
                name: "export_audio".into(),
                description: "Export audio master using a delivery preset ('master', 'podcast', 'voice_over')".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "preset": { "type": "string", "enum": ["master", "podcast", "voice_over"] },
                        "output_path": { "type": "string", "description": "Destination file path" }
                    },
                    "required": ["preset", "output_path"]
                }),
            });
        }

        JsonRpcResponse::success(id, json!({ "tools": tools }))
    }

    fn handle_tool_call(&self, id: Value, params: Value) -> JsonRpcResponse {
        let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

        let mut session = match self.session.lock() {
            Ok(s) => s,
            Err(_) => {
                return JsonRpcResponse::success(
                    id,
                    serde_json::to_value(ToolCallResult::failure("failed to acquire session lock"))
                        .unwrap(),
                );
            }
        };

        let result = match tool_name {
            "project_info" => match session.project_info() {
                Ok(info) => {
                    let summary = format!(
                        "Project '{}' (rev {}): {:.2}s, {} tracks, {} clips",
                        info.name,
                        info.revision,
                        info.duration_secs,
                        info.track_count,
                        info.clip_count
                    );
                    ToolCallResult::success(summary, serde_json::to_value(info).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Failed to retrieve project info: {e}")),
            },
            "track_list" => match session.list_tracks() {
                Ok(tracks) => {
                    let summary = format!("Project contains {} tracks", tracks.len());
                    ToolCallResult::success(summary, serde_json::to_value(tracks).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Failed to list tracks: {e}")),
            },
            "analyze_loudness" => match session.analyze_loudness() {
                Ok(report) => {
                    let summary = format!(
                        "Loudness: {:.1} LUFS · True Peak: {:.1} dBFS · Silence: {:.1}%",
                        report.integrated_lufs,
                        report.true_peak_dbfs,
                        report.silence_ratio * 100.0
                    );
                    ToolCallResult::success(summary, serde_json::to_value(report).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Loudness analysis failed: {e}")),
            },
            "plugin_list" => match session.plugin_list() {
                Ok(plugins) => {
                    let summary = format!("Discovered {} audio plugins", plugins.len());
                    ToolCallResult::success(summary, serde_json::to_value(plugins).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Failed to list plugins: {e}")),
            },
            "plugin_insert" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let plugin_id = arguments
                    .get("plugin_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match session.plugin_insert(track_id, plugin_id) {
                    Ok(effect_id) => ToolCallResult::success(
                        format!(
                            "Inserted plugin {plugin_id} on track {track_id} (effect {effect_id})"
                        ),
                        json!({ "effect_id": effect_id, "track_id": track_id }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Failed to insert plugin: {e}")),
                }
            }
            "plugin_param_set" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let effect_id = arguments
                    .get("effect_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let param_name = arguments
                    .get("param_name")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let value = arguments
                    .get("value")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                match session.plugin_param_set(track_id, effect_id, param_name, value) {
                    Ok(()) => ToolCallResult::success(
                        format!("Set parameter '{param_name}' to {value} on effect {effect_id}"),
                        json!({ "updated": true }),
                    ),
                    Err(e) => {
                        ToolCallResult::failure(format!("Failed to set plugin parameter: {e}"))
                    }
                }
            }
            "project_save" => match session.save_project() {
                Ok(()) => {
                    ToolCallResult::success("Project saved successfully", json!({ "saved": true }))
                }
                Err(e) => ToolCallResult::failure(format!("Failed to save project: {e}")),
            },
            "history_undo" => match session.undo() {
                Ok(undone) => ToolCallResult::success(
                    if undone {
                        "Undone last command"
                    } else {
                        "Nothing to undo"
                    },
                    json!({ "undone": undone }),
                ),
                Err(e) => ToolCallResult::failure(format!("Undo failed: {e}")),
            },
            "history_redo" => match session.redo() {
                Ok(redone) => ToolCallResult::success(
                    if redone {
                        "Redone command"
                    } else {
                        "Nothing to redo"
                    },
                    json!({ "redone": redone }),
                ),
                Err(e) => ToolCallResult::failure(format!("Redo failed: {e}")),
            },
            "checkpoint_create" => {
                let label = arguments
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("Checkpoint");
                match session.create_checkpoint(label) {
                    Ok(cp_id) => ToolCallResult::success(
                        format!("Created checkpoint '{label}' with handle {cp_id}"),
                        json!({ "checkpoint_id": cp_id }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Failed to create checkpoint: {e}")),
                }
            }
            "checkpoint_restore" => {
                let cp_id = arguments
                    .get("checkpoint_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match session.restore_checkpoint(cp_id) {
                    Ok(()) => ToolCallResult::success(
                        format!("Restored project to checkpoint {cp_id}"),
                        json!({ "restored": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Failed to restore checkpoint: {e}")),
                }
            }
            "rollback_session" => match session.rollback_session() {
                Ok(rolled) => ToolCallResult::success(
                    if rolled {
                        "Rolled back all AI changes to session start"
                    } else {
                        "No initial checkpoint to roll back to"
                    },
                    json!({ "rolled_back": rolled }),
                ),
                Err(e) => ToolCallResult::failure(format!("Rollback failed: {e}")),
            },
            "track_add" => {
                let name = arguments
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("New Track");
                match session.add_track(name) {
                    Ok(track_id) => ToolCallResult::success(
                        format!("Added track '{name}' with handle {track_id}"),
                        json!({ "track_id": track_id }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Failed to add track: {e}")),
                }
            }
            "track_remove" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match session.remove_track(track_id) {
                    Ok(()) => ToolCallResult::success(
                        format!("Removed track {track_id}"),
                        json!({ "removed": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Failed to remove track: {e}")),
                }
            }
            "clip_split" => {
                let clip_id = arguments
                    .get("clip_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let split_secs = arguments
                    .get("split_secs")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                match session.split_clip(clip_id, split_secs) {
                    Ok(()) => ToolCallResult::success(
                        format!("Split clip {clip_id} at {split_secs:.3}s"),
                        json!({ "split": true, "split_secs": split_secs }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Clip split failed: {e}")),
                }
            }
            "range_delete" => {
                let start = arguments
                    .get("start_secs")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let end = arguments
                    .get("end_secs")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                match session.ripple_delete_range(start, end) {
                    Ok(()) => ToolCallResult::success(
                        format!("Deleted range [{start:.2}s - {end:.2}s] and rippled audio"),
                        json!({ "deleted": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Range delete failed: {e}")),
                }
            }
            "export_audio" => {
                let preset_str = arguments
                    .get("preset")
                    .and_then(Value::as_str)
                    .unwrap_or("master");
                let output_path = arguments
                    .get("output_path")
                    .and_then(Value::as_str)
                    .unwrap_or("export.wav");
                let preset = match preset_str {
                    "podcast" => ExportPreset::Podcast,
                    "audiobook" => ExportPreset::Audiobook,
                    _ => ExportPreset::VoiceOver,
                };
                match session.export_audio(preset, output_path) {
                    Ok(path) => ToolCallResult::success(
                        format!("Exported audio with {preset_str} preset to {path}"),
                        json!({ "exported_path": path }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Export audio failed: {e}")),
                }
            }
            _ => ToolCallResult::failure(format!("tool '{tool_name}' not recognized")),
        };

        JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
    }

    fn handle_resources_list(&self, id: Value) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            json!({
                "resources": [
                    {
                        "uri": "alp://project/active",
                        "name": "Active Project State",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "alp://effects/catalog",
                        "name": "Audio Effects Catalog",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "alp://docs/agent-guide",
                        "name": "AI Agent Guide & Workflows",
                        "mimeType": "text/markdown"
                    }
                ]
            }),
        )
    }

    fn handle_resources_read(&self, id: Value, params: Value) -> JsonRpcResponse {
        let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
        match uri {
            "alp://docs/agent-guide" => JsonRpcResponse::success(
                id,
                json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "text/markdown",
                        "text": "# Audio Labs Pro AI Agent Guide\nAlways analyze loudness and silence before and after editing.\nUse checkpoints before applying complex multi-step operations."
                    }]
                }),
            ),
            "alp://effects/catalog" => JsonRpcResponse::success(
                id,
                json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": "[\"voice_cleanup\", \"podcast_master\", \"parametric_eq\", \"compressor\", \"peak_limiter\", \"de_hum\", \"de_click\"]"
                    }]
                }),
            ),
            _ => JsonRpcResponse::error(id, -32602, format!("resource '{uri}' not found"), None),
        }
    }

    fn handle_prompts_list(&self, id: Value) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            json!({
                "prompts": [
                    {
                        "name": "clean_up_voice_recording",
                        "description": "Workflow recipe to analyze speech, apply voice cleanup chain, normalize to -16 LUFS, and verify audio quality."
                    },
                    {
                        "name": "produce_podcast_episode",
                        "description": "Workflow recipe to import speech, add intro/outro music beds, apply auto-ducking, master to -16 LUFS, and export."
                    }
                ]
            }),
        )
    }

    fn handle_prompts_get(&self, id: Value, params: Value) -> JsonRpcResponse {
        let prompt_name = params.get("name").and_then(Value::as_str).unwrap_or("");
        match prompt_name {
            "clean_up_voice_recording" => JsonRpcResponse::success(
                id,
                json!({
                    "description": "Clean up voice recording workflow",
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": "Execute voice cleanup recipe:\n1. Call 'analyze_loudness'\n2. Apply voice cleanup preset to vocal tracks\n3. Call 'range_delete' to remove unwanted silence\n4. Re-analyze loudness and export master audio."
                            }
                        }
                    ]
                }),
            ),
            _ => JsonRpcResponse::error(
                id,
                -32602,
                format!("prompt '{prompt_name}' not found"),
                None,
            ),
        }
    }
}
