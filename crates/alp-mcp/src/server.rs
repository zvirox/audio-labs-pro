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
            McpTool {
                name: "media_list".into(),
                description: "List imported project media sources: duration, sample rate, channels, peak, and whether each file exists".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "tts_voices".into(),
                description: "List known TTS voices (builtin, designed, cloned) plus daemon synthesis state. Cloned profiles are visible but can never be created or imported through MCP".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "voice_profile_get".into(),
                description: "Inspect one voice profile by id: kind, backend, tags, default parameters. Consent records are reported only as a boolean".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "voice_id": { "type": "string", "description": "Voice profile handle (see tts_voices)" }
                    },
                    "required": ["voice_id"]
                }),
            },
            McpTool {
                name: "transcribe".into(),
                description: "Transcribe a workspace audio file (daemon STT when available; the built-in segmenter emits placeholder 'Speech segment N' words and says so in the result). Optionally writes text/srt/vtt output".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Audio file path inside the workspace sandbox" },
                        "format": { "type": "string", "enum": ["text", "srt", "vtt"], "description": "Transcript format when writing `out` (default text)" },
                        "out": { "type": "string", "description": "Optional destination path for the transcript file (inside the sandbox)" }
                    },
                    "required": ["path"]
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
                    description: "Delete an audio range on the first track and ripple cut subsequent audio on that track".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "start_secs": { "type": "number", "description": "Range start in seconds" },
                            "end_secs": { "type": "number", "description": "Range end in seconds" }
                        },
                        "required": ["start_secs", "end_secs"]
                    }),
                },
                McpTool {
                    name: "tts_generate".into(),
                    description: "Synthesize speech into the project media dir as a WAV file. Reports the engine that actually ran (daemon model or built-in acoustic)".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "text": { "type": "string", "description": "Text to speak" },
                            "voice_id": { "type": "string", "description": "Voice profile handle (see tts_voices)" },
                            "speed": { "type": "number", "description": "Speed multiplier, 0.25 to 4.0 (default 1.0)" }
                        },
                        "required": ["text", "voice_id"]
                    }),
                },
                McpTool {
                    name: "voice_design".into(),
                    description: "Design a new voice from a natural-language brief and persist it. Designed profiles only: cloned voice creation and import are never available through MCP".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "Friendly name for the voice" },
                            "prompt": { "type": "string", "description": "Descriptive brief, e.g. 'warm, deep BBC documentary narrator with calm pacing'" },
                            "tags": { "type": "array", "items": { "type": "string" }, "description": "Optional category tags" }
                        },
                        "required": ["name", "prompt"]
                    }),
                },
                McpTool {
                    name: "media_import".into(),
                    description: "Import a sandboxed audio file into the project media dir and register it as a source. Stored under a fresh generated name, so it never overwrites existing media".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Audio file path inside the workspace sandbox" },
                            "name": { "type": "string", "description": "Display name (defaults to the file's stem)" }
                        },
                        "required": ["path"]
                    }),
                },
                McpTool {
                    name: "clip_add".into(),
                    description: "Place media on a track as a new clip at a timeline position (extends to the end of the source unless a duration is given). Overlapping clips are legal and summed by the mix engine".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "track_id": { "type": "string", "description": "Track handle (see track_list)" },
                            "source_id": { "type": "string", "description": "Media source handle (see media_list)" },
                            "start_secs": { "type": "number", "description": "Timeline start in seconds" },
                            "duration_secs": { "type": "number", "description": "Clip length in seconds (default: whole remaining source)" },
                            "source_offset_secs": { "type": "number", "description": "Offset into the source in seconds (default 0)" },
                            "gain": { "type": "number", "description": "Linear clip gain, 0.0 to 4.0 (default 1.0)" }
                        },
                        "required": ["track_id", "source_id", "start_secs"]
                    }),
                },
            ]);
        }

        if self.tier >= PermissionTier::Export {
            tools.push(McpTool {
                name: "export_audio".into(),
                description: "Export audio master using a delivery preset ('podcast', 'audiobook', 'voice_over')".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "preset": { "type": "string", "enum": ["podcast", "audiobook", "voice_over"] },
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

        // Record what the client actually asked for so the audit trail can
        // reconstruct the request arguments, not just the timing.
        session.stage_arguments(&arguments);

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
                        format!(
                            "Deleted range [{start:.2}s - {end:.2}s] on the first track and rippled that track"
                        ),
                        json!({ "deleted": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Range delete failed: {e}")),
                }
            }
            "export_audio" => {
                let preset_str = arguments
                    .get("preset")
                    .and_then(Value::as_str)
                    .unwrap_or("voice_over");
                let output_path = arguments
                    .get("output_path")
                    .and_then(Value::as_str)
                    .unwrap_or("export.wav");
                let preset = match preset_str {
                    "podcast" => ExportPreset::Podcast,
                    "audiobook" => ExportPreset::Audiobook,
                    "voice_over" => ExportPreset::VoiceOver,
                    other => {
                        return JsonRpcResponse::success(
                            id,
                            serde_json::to_value(ToolCallResult::failure(format!(
                                "unknown export preset '{other}': use 'podcast', 'audiobook' or 'voice_over'"
                            )))
                            .unwrap(),
                        );
                    }
                };
                match session.export_audio(preset, output_path) {
                    Ok(path) => ToolCallResult::success(
                        format!("Exported audio with {preset_str} preset to {path}"),
                        json!({ "exported_path": path }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Export audio failed: {e}")),
                }
            }
            "tts_voices" => match session.tts_voices() {
                Ok(report) => {
                    let daemon_state = match (&report.daemon_active_model, report.daemon_alive) {
                        (Some(model), _) => format!("active model {model}"),
                        (None, true) => "alive (builtin tier)".to_owned(),
                        (None, false) => "not detected".to_owned(),
                    };
                    let summary = format!(
                        "Found {} voice(s); synthesis daemon: {daemon_state}",
                        report.voices.len()
                    );
                    ToolCallResult::success(summary, serde_json::to_value(report).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Failed to list voices: {e}")),
            },
            "voice_profile_get" => {
                let voice_id = arguments
                    .get("voice_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match session.voice_profile_get(voice_id) {
                    Ok(profile) => ToolCallResult::success(
                        format!(
                            "Voice '{}' ({}, backend {})",
                            profile.name, profile.kind, profile.backend
                        ),
                        serde_json::to_value(profile).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Failed to inspect voice: {e}")),
                }
            }
            "tts_generate" => {
                let text = arguments.get("text").and_then(Value::as_str).unwrap_or("");
                let voice_id = arguments
                    .get("voice_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let speed = arguments
                    .get("speed")
                    .and_then(Value::as_f64)
                    .map(|v| v as f32);
                match session.tts_generate(text, voice_id, speed) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Synthesized {:.2}s of speech with {} ({}) to {}",
                            report.duration_secs, report.engine, voice_id, report.path
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Speech synthesis failed: {e}")),
                }
            }
            "voice_design" => {
                let name = arguments.get("name").and_then(Value::as_str).unwrap_or("");
                let prompt = arguments
                    .get("prompt")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let tags: Vec<String> = arguments
                    .get("tags")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                match session.voice_design(name, prompt, tags) {
                    Ok(profile) => ToolCallResult::success(
                        format!(
                            "Designed voice '{}' ({}) and saved it to the profile store",
                            profile.name, profile.id
                        ),
                        serde_json::to_value(profile).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Voice design failed: {e}")),
                }
            }
            "transcribe" => {
                let path = arguments.get("path").and_then(Value::as_str).unwrap_or("");
                let format = arguments.get("format").and_then(Value::as_str);
                let out = arguments.get("out").and_then(Value::as_str);
                match session.transcribe(path, format, out) {
                    Ok(report) => {
                        let mut summary = format!(
                            "Transcribed by {} ({} segment(s))",
                            report.engine,
                            report.segments.len()
                        );
                        if report.engine.ends_with("builtin") {
                            summary.push_str("; note: the built-in STT emits placeholder words");
                        }
                        ToolCallResult::success(summary, serde_json::to_value(report).unwrap())
                    }
                    Err(e) => ToolCallResult::failure(format!("Transcription failed: {e}")),
                }
            }
            "media_list" => match session.media_list() {
                Ok(report) => {
                    let missing = report.media.iter().filter(|m| !m.file_exists).count();
                    let summary = if missing == 0 {
                        format!("Project has {} media source(s)", report.media.len())
                    } else {
                        format!(
                            "Project has {} media source(s) ({missing} file(s) missing on disk)",
                            report.media.len()
                        )
                    };
                    ToolCallResult::success(summary, serde_json::to_value(report).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Failed to list media: {e}")),
            },
            "media_import" => {
                let path = arguments.get("path").and_then(Value::as_str).unwrap_or("");
                let name = arguments.get("name").and_then(Value::as_str);
                match session.media_import(path, name) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Imported '{}' ({:.2}s, {} Hz, {} channel(s)) as source {} ({})",
                            report.name,
                            report.duration_secs,
                            report.sample_rate,
                            report.channels,
                            report.source_id,
                            report.relative_path
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Media import failed: {e}")),
                }
            }
            "clip_add" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let source_id = arguments
                    .get("source_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let start_secs = arguments.get("start_secs").and_then(Value::as_f64);
                let duration_secs = arguments.get("duration_secs").and_then(Value::as_f64);
                let source_offset_secs =
                    arguments.get("source_offset_secs").and_then(Value::as_f64);
                let gain = arguments
                    .get("gain")
                    .and_then(Value::as_f64)
                    .map(|g| g as f32);
                let Some(start_secs) = start_secs else {
                    return JsonRpcResponse::success(
                        id,
                        serde_json::to_value(ToolCallResult::failure(
                            "missing 'start_secs': clips need a timeline start in seconds",
                        ))
                        .unwrap(),
                    );
                };
                match session.clip_add(
                    track_id,
                    source_id,
                    start_secs,
                    duration_secs,
                    source_offset_secs,
                    gain,
                ) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Placed clip {} on track {} at {:.2}s ({:.2}s long from source {})",
                            report.clip_id,
                            report.track_id,
                            report.start_secs,
                            report.duration_secs,
                            report.source_id
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Clip placement failed: {e}")),
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
                        "uri": "alp://voices",
                        "name": "Voice Profiles and Synthesis State",
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
            "alp://project/active" => {
                let summary = self
                    .session
                    .lock()
                    .map(|session| session.project_info())
                    .unwrap_or_else(|_| {
                        Err(alp_api::ApiError::Validation(
                            "failed to acquire session lock".into(),
                        ))
                    });
                match summary {
                    Ok(info) => JsonRpcResponse::success(
                        id,
                        json!({
                            "contents": [{
                                "uri": uri,
                                "mimeType": "application/json",
                                "text": serde_json::to_string(&info).unwrap_or_default()
                            }]
                        }),
                    ),
                    Err(e) => JsonRpcResponse::error(
                        id,
                        -32602,
                        format!("failed to read active project: {e}"),
                        None,
                    ),
                }
            }
            "alp://voices" => {
                let voices = self
                    .session
                    .lock()
                    .map(|session| session.tts_voices())
                    .unwrap_or_else(|_| {
                        Err(alp_api::ApiError::Validation(
                            "failed to acquire session lock".into(),
                        ))
                    });
                match voices {
                    Ok(report) => JsonRpcResponse::success(
                        id,
                        json!({
                            "contents": [{
                                "uri": uri,
                                "mimeType": "application/json",
                                "text": serde_json::to_string(&report).unwrap_or_default()
                            }]
                        }),
                    ),
                    Err(e) => JsonRpcResponse::error(
                        id,
                        -32602,
                        format!("failed to read voices: {e}"),
                        None,
                    ),
                }
            }
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
