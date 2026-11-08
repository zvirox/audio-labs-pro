//! Core Model Context Protocol (MCP) server dispatcher.

use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

use alp_api::{ApiSession, ExportPreset, PermissionTier};

use crate::protocol::{JsonRpcRequest, JsonRpcResponse, ToolCallResult};

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
        let tools = crate::tool_defs::tools_for_tier(self.tier);
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
            // --- Project lifecycle ---
            "project_list" => match session.project_list() {
                Ok(list) => ToolCallResult::success(
                    format!("Found {} project(s)", list.len()),
                    serde_json::to_value(list).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Failed to list projects: {e}")),
            },
            "project_create" => {
                let name = arguments.get("name").and_then(Value::as_str).unwrap_or("");
                let sample_rate = arguments.get("sample_rate").and_then(Value::as_u64);
                match session.project_create(name, sample_rate.map(|v| v as u32)) {
                    Ok(project_id) => ToolCallResult::success(
                        format!("Created project '{name}' with handle {project_id}"),
                        json!({ "project_id": project_id }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Project creation failed: {e}")),
                }
            }
            "project_open" => {
                let path = arguments.get("path").and_then(Value::as_str).unwrap_or("");
                match session.project_open(path) {
                    Ok(project_id) => ToolCallResult::success(
                        format!("Opened project at '{path}' with handle {project_id}"),
                        json!({ "project_id": project_id }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Project open failed: {e}")),
                }
            }
            "project_close" => match session.project_close() {
                Ok(()) => ToolCallResult::success(
                    "Project closed, reverted to empty draft",
                    json!({ "closed": true }),
                ),
                Err(e) => ToolCallResult::failure(format!("Project close failed: {e}")),
            },
            "history_list" => match session.history_list() {
                Ok(entries) => ToolCallResult::success(
                    format!("History has {} entries", entries.len()),
                    serde_json::to_value(entries).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Failed to list history: {e}")),
            },
            // --- Track & Clip updates ---
            "track_update" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let name = arguments
                    .get("name")
                    .and_then(Value::as_str)
                    .map(String::from);
                let gain = arguments
                    .get("gain")
                    .and_then(Value::as_f64)
                    .map(|v| v as f32);
                let pan = arguments
                    .get("pan")
                    .and_then(Value::as_f64)
                    .map(|v| v as f32);
                let muted = arguments.get("muted").and_then(Value::as_bool);
                let solo = arguments.get("solo").and_then(Value::as_bool);
                let armed = arguments.get("armed").and_then(Value::as_bool);
                match session.track_update(track_id, name, gain, pan, muted, solo, armed) {
                    Ok(()) => ToolCallResult::success(
                        format!("Updated track {track_id}"),
                        json!({ "updated": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Track update failed: {e}")),
                }
            }
            "clip_update" => {
                let clip_id = arguments
                    .get("clip_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let start_secs = arguments.get("start_secs").and_then(Value::as_f64);
                let gain = arguments
                    .get("gain")
                    .and_then(Value::as_f64)
                    .map(|v| v as f32);
                let fade_in_secs = arguments.get("fade_in_secs").and_then(Value::as_f64);
                let fade_out_secs = arguments.get("fade_out_secs").and_then(Value::as_f64);
                match session.clip_update(clip_id, start_secs, gain, fade_in_secs, fade_out_secs) {
                    Ok(()) => ToolCallResult::success(
                        format!("Updated clip {clip_id}"),
                        json!({ "updated": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Clip update failed: {e}")),
                }
            }
            "clip_remove" => {
                let clip_id = arguments
                    .get("clip_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match session.clip_remove(clip_id) {
                    Ok(()) => ToolCallResult::success(
                        format!("Removed clip {clip_id}"),
                        json!({ "removed": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Clip removal failed: {e}")),
                }
            }
            "marker_add" => {
                let name = arguments.get("name").and_then(Value::as_str).unwrap_or("");
                let position_secs = arguments
                    .get("position_secs")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                match session.marker_add(name, position_secs) {
                    Ok(marker_id) => ToolCallResult::success(
                        format!(
                            "Added marker '{name}' at {position_secs:.3}s with handle {marker_id}"
                        ),
                        json!({ "marker_id": marker_id }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Marker add failed: {e}")),
                }
            }
            "edit_batch" => {
                let label = arguments
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("Batch");
                match session.edit_batch(label) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Executed batch '{label}': {} operations",
                            report.operations_applied
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Batch execution failed: {e}")),
                }
            }
            // --- Effects & Presets ---
            "effect_catalog" => match session.effect_catalog() {
                Ok(report) => ToolCallResult::success(
                    format!(
                        "Found {} effects and {} presets",
                        report.effects.len(),
                        report.presets.len()
                    ),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Effect catalog failed: {e}")),
            },
            "effect_apply" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let effect_id = arguments
                    .get("effect_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let params = arguments.get("params").cloned().unwrap_or(Value::Null);
                match session.effect_apply(track_id, effect_id, params) {
                    Ok(fx_id) => ToolCallResult::success(
                        format!("Applied effect {effect_id} on track {track_id} as {fx_id}"),
                        json!({ "effect_id": fx_id, "track_id": track_id }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Effect apply failed: {e}")),
                }
            }
            "effect_update" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let effect_id = arguments
                    .get("effect_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let params = arguments.get("params").cloned().unwrap_or(Value::Null);
                match session.effect_update(track_id, effect_id, params) {
                    Ok(()) => ToolCallResult::success(
                        format!("Updated effect {effect_id} on track {track_id}"),
                        json!({ "updated": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Effect update failed: {e}")),
                }
            }
            "effect_remove" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let effect_id = arguments
                    .get("effect_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match session.effect_remove(track_id, effect_id) {
                    Ok(()) => ToolCallResult::success(
                        format!("Removed effect {effect_id} from track {track_id}"),
                        json!({ "removed": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Effect removal failed: {e}")),
                }
            }
            "preset_apply" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let preset = arguments
                    .get("preset")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match session.preset_apply(track_id, preset) {
                    Ok(()) => ToolCallResult::success(
                        format!("Applied preset '{preset}' to track {track_id}"),
                        json!({ "applied": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Preset apply failed: {e}")),
                }
            }
            // --- Smart Edits ---
            "edit_trim_silence" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let threshold_db = arguments
                    .get("threshold_db")
                    .and_then(Value::as_f64)
                    .unwrap_or(-30.0) as f32;
                let min_silence_secs = arguments
                    .get("min_silence_secs")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.1);
                match session.edit_trim_silence(track_id, threshold_db, min_silence_secs) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Trimmed {:.2}s of silence from track {track_id}",
                            report.silence_trimmed_secs
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Silence trim failed: {e}")),
                }
            }
            "edit_shorten_pauses" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let max_pause_secs = arguments
                    .get("max_pause_secs")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.5);
                let crossfade_ms = arguments
                    .get("crossfade_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(10);
                match session.edit_shorten_pauses(track_id, max_pause_secs, crossfade_ms) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Shortened {:.2}s of pauses on track {track_id}",
                            report.seconds_removed
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Pause shorten failed: {e}")),
                }
            }
            "edit_remove_fillers" => {
                let track_id = arguments
                    .get("track_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let filler_words = arguments
                    .get("filler_word_ids")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(Value::as_str)
                            .map(String::from)
                            .collect()
                    });
                match session.edit_remove_fillers(track_id, filler_words) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Removed {} filler words from track {track_id}",
                            report.fillers_removed
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Filler removal failed: {e}")),
                }
            }
            "edit_match_loudness" => {
                let target_lufs = arguments
                    .get("target_lufs")
                    .and_then(Value::as_f64)
                    .unwrap_or(-16.0) as f32;
                match session.edit_match_loudness(target_lufs) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Matched loudness to {target_lufs:.1} LUFS (actual {actual:.1} LUFS)",
                            actual = report.actual_lufs
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Loudness match failed: {e}")),
                }
            }
            // --- Analysis ---
            "analyze_silence" => match session.analyze_silence() {
                Ok(report) => ToolCallResult::success(
                    format!(
                        "Found {} silence intervals ({:.2}s total)",
                        report.silence_count, report.total_silence_secs
                    ),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Silence analysis failed: {e}")),
            },
            "analyze_quality" => match session.analyze_quality() {
                Ok(report) => ToolCallResult::success(
                    format!(
                        "Quality: LUFS={:.1}, Peak={:.1}dB, Noise={:.1}dB, Clips={}, Speech={:.0}%",
                        report.integrated_lufs,
                        report.true_peak_dbfs,
                        report.noise_floor_dbfs,
                        report.clipping_spikes,
                        report.speech_ratio * 100.0
                    ),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Quality analysis failed: {e}")),
            },
            "analyze_spectrum" => match session.analyze_spectrum() {
                Ok(report) => ToolCallResult::success(
                    format!("Spectrum: {} frequency bins", report.bins.len()),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Spectrum analysis failed: {e}")),
            },
            "render_waveform" => {
                let width = arguments.get("width").and_then(Value::as_u64).unwrap_or(80) as usize;
                let height = arguments
                    .get("height")
                    .and_then(Value::as_u64)
                    .unwrap_or(20) as usize;
                match session.render_waveform(width, height) {
                    Ok(report) => ToolCallResult::success(
                        format!("Rendered waveform ({}x{})", width, height),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Waveform render failed: {e}")),
                }
            }
            "render_spectrogram" => {
                let width = arguments.get("width").and_then(Value::as_u64).unwrap_or(80) as usize;
                let height = arguments
                    .get("height")
                    .and_then(Value::as_u64)
                    .unwrap_or(40) as usize;
                match session.render_spectrogram(width, height) {
                    Ok(report) => ToolCallResult::success(
                        format!("Rendered spectrogram ({}x{})", width, height),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Spectrogram render failed: {e}")),
                }
            }
            "compare_renders" => {
                let path_a = arguments
                    .get("path_a")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let path_b = arguments
                    .get("path_b")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match session.compare_renders(path_a, path_b) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Similarity: {:.1}% - {}",
                            report.similarity * 100.0,
                            report.recommendation
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Comparison failed: {e}")),
                }
            }
            // --- Export ---
            "export_presets" => match session.export_presets() {
                Ok(report) => ToolCallResult::success(
                    format!("Found {} export presets", report.presets.len()),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Export presets failed: {e}")),
            },
            "export_stems" => {
                let output_dir = arguments
                    .get("output_dir")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let preset_id = arguments
                    .get("preset_id")
                    .and_then(Value::as_str)
                    .unwrap_or("voice_over");
                match session.export_stems(output_dir, preset_id) {
                    Ok(reports) => ToolCallResult::success(
                        format!("Exported {} stems to {output_dir}", reports.len()),
                        serde_json::to_value(reports).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Stem export failed: {e}")),
                }
            }
            "export_transcript" => {
                let format = arguments
                    .get("format")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let path = arguments.get("path").and_then(Value::as_str).unwrap_or("");
                match session.export_transcript(format, path) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Exported transcript in {} format to {}",
                            report.format, report.path
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Transcript export failed: {e}")),
                }
            }
            "preview_render" => {
                let start_secs = arguments
                    .get("start_secs")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let duration_secs = arguments
                    .get("duration_secs")
                    .and_then(Value::as_f64)
                    .unwrap_or(10.0);
                match session.preview_render(start_secs, duration_secs) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Preview render: {:.2}s from {:.2}s",
                            report.duration_secs, report.start_secs
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Preview render failed: {e}")),
                }
            }
            // --- Jobs ---
            "job_status" => {
                let job_id = arguments
                    .get("job_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match session.job_status(job_id) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Job {job_id}: {} ({:.0}%)",
                            report.state,
                            report.progress * 100.0
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Job status failed: {e}")),
                }
            }
            "job_cancel" => {
                let job_id = arguments
                    .get("job_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match session.job_cancel(job_id) {
                    Ok(()) => ToolCallResult::success(
                        format!("Cancelled job {job_id}"),
                        json!({ "cancelled": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Job cancel failed: {e}")),
                }
            }
            // --- Script & Voice ---
            "tts_script_generate" => {
                let script_json = arguments
                    .get("script_json")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match session.tts_script_generate(script_json) {
                    Ok(()) => ToolCallResult::success(
                        "Script generated and placed on timeline",
                        json!({ "generated": true }),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Script generation failed: {e}")),
                }
            }
            "tts_regenerate" => {
                let clip_id = arguments
                    .get("clip_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let new_text = arguments
                    .get("new_text")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let voice_id = arguments
                    .get("voice_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let speed = arguments
                    .get("speed")
                    .and_then(Value::as_f64)
                    .map(|v| v as f32);
                match session.tts_regenerate(clip_id, new_text, voice_id, speed) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Regenerated clip {clip_id} with new text ({:.2}s)",
                            report.duration_secs
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Regeneration failed: {e}")),
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
