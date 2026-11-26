//! Tool-call dispatch for `alp-mcp`: maps a tool name and its JSON arguments
//! onto real `alp-api` operations.
//!
//! Keeping the dispatch out of the JSON-RPC server makes two things clear that
//! are easy to lose in a 1300-line `match`: the server owns transport, limits
//! and the project lock, while this module owns the mapping from tool to
//! operation. Every arm returns a `ToolCallResult`, so a tool either produces
//! the artefact it names or fails with an explanation; nothing here invents a
//! success.
//!
//! The conservative `return` arms are deliberate: a missing argument is
//! reported as a failure that names the argument, not as a zero-value default
//! that silently edits the wrong thing.

use alp_api::{
    ApiSession, BatchTtsRequest, ExportPreset, MacroRequest, MacroStep, VoiceParamsUpdate,
    VoiceProfileUpdate,
};
use serde_json::{Value, json};

use crate::protocol::ToolCallResult;

/// Read `limit`/`cursor` from tool arguments into a bounded page request.
///
/// A malformed cursor or a zero limit is a caller error, so it surfaces as a
/// failure naming the argument rather than as a silent restart at offset 0.
pub(crate) fn page_request(arguments: &Value) -> Result<alp_api::PageRequest, String> {
    let limit = arguments.get("limit").and_then(Value::as_u64);
    let cursor = arguments.get("cursor").and_then(Value::as_str);
    alp_api::PageRequest::resolve(limit, cursor).map_err(|error| error.to_string())
}

/// Dispatch one tool call against the session.
///
/// `arguments` is the raw JSON object the client sent; `session` is the locked,
/// mutable automation session. Callers are responsible for permission-tier
/// filtering (the tool list only advertises what the tier allows) and for the
/// optimistic-concurrency check, so this function never re-rejects a call for a
/// reason the server already checked.
pub fn dispatch(session: &mut ApiSession, tool_name: &str, arguments: &Value) -> ToolCallResult {
    match tool_name {
        "project_info" => match session.project_info() {
            Ok(info) => {
                let summary = format!(
                    "Project '{}' (rev {}): {:.2}s, {} tracks, {} clips",
                    info.name, info.revision, info.duration_secs, info.track_count, info.clip_count
                );
                ToolCallResult::success(summary, serde_json::to_value(info).unwrap())
            }
            Err(e) => ToolCallResult::failure(format!("Failed to retrieve project info: {e}")),
        },
        "track_list" => {
            let request = match page_request(arguments) {
                Ok(request) => request,
                Err(error) => {
                    return ToolCallResult::failure(error);
                }
            };
            match session.list_tracks_page(request) {
                Ok(page) => {
                    let summary = if page.is_last() {
                        format!("Project contains {} tracks", page.total)
                    } else {
                        format!(
                            "Tracks {}–{} of {} (call track_list with cursor '{}' for more)",
                            page.offset + 1,
                            page.offset + page.items.len(),
                            page.total,
                            page.next_cursor.clone().unwrap_or_default()
                        )
                    };
                    ToolCallResult::success(summary, serde_json::to_value(page).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Failed to list tracks: {e}")),
            }
        }

        "analyze_loudness" => match session.analyze_loudness() {
            Ok(report) => {
                let summary = format!(
                    "Loudness: {:.1} LUFS · Peak: {:.1} dBFS · Active: {:.0}%",
                    report.integrated_lufs,
                    report.peak_dbfs,
                    report.active_ratio * 100.0
                );
                ToolCallResult::success(summary, serde_json::to_value(report).unwrap())
            }
            Err(e) => ToolCallResult::failure(format!("Loudness analysis failed: {e}")),
        },
        "plugin_list" => {
            let request = match page_request(arguments) {
                Ok(request) => request,
                Err(error) => {
                    return ToolCallResult::failure(error);
                }
            };
            match session.plugin_list_page(request) {
                Ok(page) => {
                    let summary = format!(
                        "Discovered {} audio plugin(s){}",
                        page.total,
                        if page.is_last() {
                            String::new()
                        } else {
                            format!(
                                "; this page has {} — pass cursor '{}' for more",
                                page.items.len(),
                                page.next_cursor.clone().unwrap_or_default()
                            )
                        }
                    );
                    ToolCallResult::success(summary, serde_json::to_value(page).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Failed to list plugins: {e}")),
            }
        }

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
                    format!("Inserted plugin {plugin_id} on track {track_id} (effect {effect_id})"),
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
                Err(e) => ToolCallResult::failure(format!("Failed to set plugin parameter: {e}")),
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
                    return ToolCallResult::failure(format!(
                        "unknown export preset '{other}': use 'podcast', 'audiobook' or 'voice_over'"
                    ));
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
        "media_list" => {
            let request = match page_request(arguments) {
                Ok(request) => request,
                Err(error) => {
                    return ToolCallResult::failure(error);
                }
            };
            match session.media_list_page(request) {
                Ok(page) => {
                    let missing = page.items.iter().filter(|m| !m.file_exists).count();
                    let summary = if page.is_last() {
                        if missing == 0 {
                            format!("Project has {} media source(s)", page.total)
                        } else {
                            format!(
                                "Project has {} media source(s) ({missing} file(s) missing on disk)",
                                page.total
                            )
                        }
                    } else {
                        format!(
                            "Media {}–{} of {} (call media_list with cursor '{}' for more)",
                            page.offset + 1,
                            page.offset + page.items.len(),
                            page.total,
                            page.next_cursor.clone().unwrap_or_default()
                        )
                    };
                    ToolCallResult::success(summary, serde_json::to_value(page).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Failed to list media: {e}")),
            }
        }

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
            let source_offset_secs = arguments.get("source_offset_secs").and_then(Value::as_f64);
            let gain = arguments
                .get("gain")
                .and_then(Value::as_f64)
                .map(|g| g as f32);
            let Some(start_secs) = start_secs else {
                return ToolCallResult::failure(
                    "missing 'start_secs': clips need a timeline start in seconds",
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
        "project_list" => {
            let request = match page_request(arguments) {
                Ok(request) => request,
                Err(error) => {
                    return ToolCallResult::failure(error);
                }
            };
            match session.project_list_page(request) {
                Ok(page) => {
                    let summary = format!(
                        "Found {} project(s){}",
                        page.total,
                        if page.is_last() {
                            String::new()
                        } else {
                            format!(
                                "; this page has {} — pass cursor '{}' for more",
                                page.items.len(),
                                page.next_cursor.clone().unwrap_or_default()
                            )
                        }
                    );
                    ToolCallResult::success(summary, serde_json::to_value(page).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Failed to list projects: {e}")),
            }
        }

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
        "history_list" => {
            let request = match page_request(arguments) {
                Ok(request) => request,
                Err(error) => {
                    return ToolCallResult::failure(error);
                }
            };
            match session.history_list_page(request) {
                Ok(page) => {
                    let undoable = page.items.iter().filter(|e| e.is_undo).count();
                    let summary = format!(
                        "{} history entr{} on this page ({} undoable, {} total){}",
                        page.items.len(),
                        if page.items.len() == 1 { "y" } else { "ies" },
                        undoable,
                        page.total,
                        if page.is_last() {
                            String::new()
                        } else {
                            format!(
                                "; pass cursor '{}' for the next page",
                                page.next_cursor.clone().unwrap_or_default()
                            )
                        }
                    );
                    ToolCallResult::success(summary, serde_json::to_value(page).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Failed to list history: {e}")),
            }
        }

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
                    format!("Added marker '{name}' at {position_secs:.3}s with handle {marker_id}"),
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
            let raw_ops = arguments
                .get("operations")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new()));
            match serde_json::from_value::<Vec<alp_api::BatchOp>>(raw_ops) {
                Ok(ops) => match session.edit_batch(label, &ops) {
                    Ok(report) => ToolCallResult::success(
                        format!(
                            "Executed batch '{}': {} operation(s){}",
                            report.label,
                            report.operations_applied,
                            if report.single_undo_step {
                                ", one undo step"
                            } else {
                                ""
                            }
                        ),
                        serde_json::to_value(report).unwrap(),
                    ),
                    Err(e) => ToolCallResult::failure(format!("Batch execution failed: {e}")),
                },
                Err(e) => ToolCallResult::failure(format!(
                    "invalid 'operations' item: {e}; each item needs an \"op\" field (track_update, clip_update, clip_remove, clip_split, marker_add, range_delete, effect_apply, effect_remove)"
                )),
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
                        "Trimmed {:.2}s of silence in {} range(s) from track {track_id} ({:.2}s -> {:.2}s)",
                        report.silence_trimmed_secs,
                        report.ranges_cut,
                        report.before_secs,
                        report.after_secs
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
                        "Shortened {} pause(s) by {:.2}s on track {track_id} ({:.2}s -> {:.2}s)",
                        report.pauses_processed,
                        report.seconds_removed,
                        report.before_secs,
                        report.after_secs
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
                        "Removed {} filler word(s) from track {track_id} ({:.2}s -> {:.2}s) using {}{}",
                        report.fillers_removed,
                        report.before_secs,
                        report.after_secs,
                        report.engine,
                        if report.fillers_found.is_empty() {
                            String::new()
                        } else {
                            format!(": {}", report.fillers_found.join(", "))
                        }
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
                        "Applied {:+.1} dB: {:.1} -> {:.1} LUFS (target {:.1}), peak {:.1} dBFS{}",
                        report.gain_db,
                        report.before_lufs,
                        report.actual_lufs,
                        report.target_lufs,
                        report.peak_dbfs,
                        if report.limited_by_peak {
                            " — gain reduced to keep the peak under the ceiling"
                        } else {
                            ""
                        }
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
                    "Found {} silence interval(s) ≥{:.2}s at {:.0} dBFS ({:.2}s total, longest {:.2}s)",
                    report.silence_count,
                    report.min_silence_secs,
                    report.threshold_dbfs,
                    report.total_silence_secs,
                    report.max_silence_secs
                ),
                serde_json::to_value(report).unwrap(),
            ),
            Err(e) => ToolCallResult::failure(format!("Silence analysis failed: {e}")),
        },
        "analyze_quality" => match session.analyze_quality() {
            Ok(report) => ToolCallResult::success(
                format!(
                    "Quality: LUFS={:.1}, Peak={:.1}dB, Noise floor={:.1}dB, Clips={}, Active={:.0}%",
                    report.integrated_lufs,
                    report.peak_dbfs,
                    report.noise_floor_dbfs,
                    report.clipping_spikes,
                    report.active_ratio * 100.0
                ),
                serde_json::to_value(report).unwrap(),
            ),
            Err(e) => ToolCallResult::failure(format!("Quality analysis failed: {e}")),
        },
        "analyze_spectrum" => match session.analyze_spectrum() {
            Ok(report) => ToolCallResult::success(
                format!(
                    "Spectrum: {} log-spaced bands averaged over {} block(s) at {} Hz",
                    report.bins.len(),
                    report.blocks,
                    report.sample_rate
                ),
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
                    format!(
                        "Waveform {}x{} of {:.2}s (peak {:.1} dBFS)\n{}",
                        report.width,
                        report.height,
                        report.duration_secs,
                        20.0 * report.peak.max(1e-6).log10(),
                        report.ascii
                    ),
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
                    format!(
                        "Spectrogram {}x{} at {} Hz, {:.0} dB range (low frequencies at the bottom)\n{}",
                        report.width,
                        report.height,
                        report
                            .band_centers_hz
                            .last()
                            .map(|top| format!("{top:.0}"))
                            .unwrap_or_else(|| "?".into()),
                        report.range_db,
                        report.ascii
                    ),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Spectrogram render failed: {e}")),
            }
        }
        "timeline_spectral_analyze" => {
            let clip_id = arguments
                .get("clip_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let fft_size = arguments
                .get("fft_size")
                .and_then(Value::as_u64)
                .map(|v| v as usize);
            let window_type = arguments
                .get("window_type")
                .and_then(Value::as_str)
                .map(String::from);
            let freq_scale = arguments
                .get("freq_scale")
                .and_then(Value::as_str)
                .map(String::from);
            let num_freq_bins = arguments
                .get("num_freq_bins")
                .and_then(Value::as_u64)
                .map(|v| v as usize);
            match session.spectral_analyze_clip(
                clip_id,
                fft_size,
                window_type,
                freq_scale,
                num_freq_bins,
            ) {
                Ok(data) => ToolCallResult::success(
                    format!(
                        "Spectrogram computed: {} time frames x {} frequency bins ({:.1} Hz to {:.1} Hz, duration {:.2}s)",
                        data.num_frames,
                        data.num_bins,
                        data.freq_bins_hz.first().copied().unwrap_or(0.0),
                        data.freq_bins_hz.last().copied().unwrap_or(0.0),
                        data.duration_sec
                    ),
                    serde_json::to_value(data).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Spectral analysis failed: {e}")),
            }
        }
        "timeline_spectral_select" => {
            let track_id = arguments
                .get("track_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let t_start = arguments
                .get("t_start")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let t_end = arguments
                .get("t_end")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let f_low = arguments
                .get("f_low")
                .and_then(Value::as_f64)
                .unwrap_or(20.0) as f32;
            let f_high = arguments
                .get("f_high")
                .and_then(Value::as_f64)
                .unwrap_or(20000.0) as f32;
            match session.spectral_select(track_id, t_start, t_end, f_low, f_high) {
                Ok(report) => ToolCallResult::success(
                    format!(
                        "Spectral selection on track {}: [{:.2}s - {:.2}s] x [{:.1} Hz - {:.1} Hz] (fc={:.1} Hz, BW={:.1} Hz, Q={:.2})",
                        report.track_id,
                        report.t_start,
                        report.t_end,
                        report.f_low,
                        report.f_high,
                        report.center_freq_hz,
                        report.bandwidth_hz,
                        report.q_factor
                    ),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Spectral select failed: {e}")),
            }
        }
        "timeline_set_view_mode" => {
            let track_id = arguments
                .get("track_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let view_mode = arguments
                .get("view_mode")
                .and_then(Value::as_str)
                .unwrap_or("waveform");
            match session.set_track_view_mode(track_id, view_mode) {
                Ok(report) => ToolCallResult::success(
                    format!(
                        "Set track {} view mode to '{}'",
                        report.track_id, report.view_mode
                    ),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Set view mode failed: {e}")),
            }
        }
        "clip_set_pan_envelope" => {
            let clip_id = arguments
                .get("clip_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let points: Vec<(f64, f32)> = arguments
                .get("points")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| {
                            let offset = p.get("offset_secs")?.as_f64()?;
                            let pan = p.get("pan")?.as_f64()? as f32;
                            Some((offset, pan))
                        })
                        .collect()
                })
                .unwrap_or_default();
            match session.set_clip_pan_envelope(clip_id, &points) {
                Ok(report) => ToolCallResult::success(
                    format!(
                        "Configured {} pan envelope points on clip {}",
                        report.points_count, report.clip_id
                    ),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Set pan envelope failed: {e}")),
            }
        }
        "clip_slip" => {
            let clip_id = arguments
                .get("clip_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let delta_secs = arguments
                .get("delta_secs")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            match session.slip_clip(clip_id, delta_secs) {
                Ok(report) => ToolCallResult::success(
                    format!(
                        "Slipped clip {} source offset by {:+.3}s",
                        report.clip_id, report.delta_secs
                    ),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Clip slip failed: {e}")),
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
                    format!(
                        "Exported {} per-track stem(s) to {output_dir}",
                        reports.len()
                    ),
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
                        "Exported a {} transcript ({} segment(s) from {}) to {}",
                        report.format, report.segments, report.engine, report.path
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
                        "Preview written to {} ({:.2}s from {:.2}s, peak {:.1} dBFS)",
                        report.preview_path,
                        report.duration_secs,
                        report.start_secs,
                        20.0 * report.peak.max(1e-6).log10()
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
                Ok(report) => ToolCallResult::success(
                    format!(
                        "Generated {} clip(s) on track {} ({:.2}s of audio, ends at {:.2}s){}",
                        report.clip_ids.len(),
                        report.track_id,
                        report.total_duration_secs,
                        report.end_secs,
                        if report.single_undo_step {
                            ", one undo step"
                        } else {
                            ""
                        }
                    ),
                    serde_json::to_value(report).unwrap(),
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
                        "Replaced clip {clip_id} with {} at {:.2}s ({:.2}s of new audio from {})",
                        report.new_clip_id, report.start_secs, report.duration_secs, report.engine
                    ),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Regeneration failed: {e}")),
            }
        }
        "voice_profile_update" => {
            let voice_id = arguments
                .get("voice_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let default_params = arguments.get("default_params").and_then(Value::as_object);
            let update = VoiceProfileUpdate {
                name: arguments
                    .get("name")
                    .and_then(Value::as_str)
                    .map(String::from),
                tags: arguments
                    .get("tags")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(String::from)
                            .collect()
                    }),
                default_params: default_params.map(|params| VoiceParamsUpdate {
                    speed: params
                        .get("speed")
                        .and_then(Value::as_f64)
                        .map(|v| v as f32),
                    pitch: params
                        .get("pitch")
                        .and_then(Value::as_f64)
                        .map(|v| v as f32),
                    energy: params
                        .get("energy")
                        .and_then(Value::as_f64)
                        .map(|v| v as f32),
                    emotion: params
                        .get("emotion")
                        .and_then(Value::as_str)
                        .map(String::from),
                }),
            };
            match session.voice_profile_update(voice_id, update) {
                Ok(detail) => ToolCallResult::success(
                    format!("Updated voice '{}' ({})", detail.name, detail.id),
                    serde_json::to_value(detail).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Voice profile update failed: {e}")),
            }
        }
        "voice_profile_lock" => {
            let voice_id = arguments
                .get("voice_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            match session.voice_profile_lock(voice_id) {
                Ok(detail) => ToolCallResult::success(
                    format!("Locked voice '{}' ({})", detail.name, detail.id),
                    serde_json::to_value(detail).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Voice profile lock failed: {e}")),
            }
        }
        "voice_profile_unlock" => {
            let voice_id = arguments
                .get("voice_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            match session.voice_profile_unlock(voice_id) {
                Ok(detail) => ToolCallResult::success(
                    format!("Unlocked voice '{}' ({})", detail.name, detail.id),
                    serde_json::to_value(detail).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Voice profile unlock failed: {e}")),
            }
        }
        "tts_batch" => {
            let requests: Vec<BatchTtsRequest> = arguments
                .get("requests")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            Some(BatchTtsRequest {
                                voice_id: item.get("voice_id").and_then(Value::as_str)?.to_owned(),
                                text: item.get("text").and_then(Value::as_str)?.to_owned(),
                                speed: item.get("speed").and_then(Value::as_f64).map(|v| v as f32),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            if requests.is_empty() {
                return ToolCallResult::failure(
                    "requests is empty; provide at least one {voice_id, text} item",
                );
            }
            match session.tts_batch(&requests) {
                Ok(report) => {
                    let summary = format!(
                        "Synthesized {} clip(s): {}",
                        report.results.len(),
                        report
                            .results
                            .iter()
                            .map(|r| r.path.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    ToolCallResult::success(summary, serde_json::to_value(report).unwrap())
                }
                Err(e) => ToolCallResult::failure(format!("Batch synthesis failed: {e}")),
            }
        }
        "audiobook_encode" => {
            let script_json = arguments
                .get("script_json")
                .and_then(Value::as_str)
                .unwrap_or("");
            let output_path = arguments.get("output_path").and_then(Value::as_str);
            match session.audiobook_encode(script_json, output_path) {
                Ok(report) => ToolCallResult::success(
                    format!(
                        "Encoded audiobook on track {}: {} segment(s), master at {}",
                        report.track_id, report.segment_count, report.master_path
                    ),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("Audiobook encode failed: {e}")),
            }
        }
        "dsp_macro_execute" => {
            let input_dir = arguments
                .get("input_dir")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            let output_dir = arguments
                .get("output_dir")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            if input_dir.is_empty() || output_dir.is_empty() {
                return ToolCallResult::failure(
                    "input_dir and output_dir are required workspace paths",
                );
            }
            let steps_json = match arguments.get("steps").and_then(Value::as_array) {
                Some(steps) if !steps.is_empty() => steps,
                _ => return ToolCallResult::failure("steps must be a non-empty array"),
            };
            let mut steps = Vec::with_capacity(steps_json.len());
            for step in steps_json {
                let effect_id = step.get("effect_id").and_then(Value::as_str);
                let Some(effect_id) = effect_id else {
                    return ToolCallResult::failure("each step requires effect_id");
                };
                let params = step.get("params").cloned().unwrap_or_else(|| json!({}));
                let params: std::collections::BTreeMap<String, f64> =
                    match serde_json::from_value(params) {
                        Ok(params) => params,
                        Err(error) => {
                            return ToolCallResult::failure(format!(
                                "invalid parameters for {effect_id}: {error}"
                            ));
                        }
                    };
                steps.push(MacroStep {
                    effect_id: effect_id.to_owned(),
                    params,
                });
            }
            let request = MacroRequest {
                input_dir,
                output_dir,
                pattern: arguments.get("pattern").and_then(Value::as_str).map(str::to_owned),
                overwrite: arguments
                    .get("overwrite")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                steps,
            };
            match session.dsp_macro_execute(&request) {
                Ok(report) => ToolCallResult::success(
                    format!(
                        "Macro job {} processed {} file(s): {} succeeded, {} failed",
                        report.job_id,
                        report.report.processed,
                        report.report.succeeded,
                        report.report.failed
                    ),
                    serde_json::to_value(report).unwrap(),
                ),
                Err(e) => ToolCallResult::failure(format!("DSP macro failed: {e}")),
            }
        }
        _ => ToolCallResult::failure(format!("tool '{tool_name}' not recognized")),
    }
}
