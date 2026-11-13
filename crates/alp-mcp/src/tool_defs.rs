//! Tool definitions and JSON schemas for Audio Labs Pro MCP server.

use alp_api::PermissionTier;
use serde_json::json;

use crate::protocol::McpTool;

/// Return all MCP tools available for the given permission tier.
pub fn tools_for_tier(tier: PermissionTier) -> Vec<McpTool> {
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
                description: "Measure integrated loudness (LUFS, ITU-R BS.1770), sample peak, noise floor, active-frame ratio and clipping on the project mix".into(),
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
            McpTool {
                name: "project_list".into(),
                description: "List all projects in the workspace sandbox".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "project_create".into(),
                description: "Create a new project in the workspace".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Project name" },
                        "sample_rate": { "type": "integer", "description": "Sample rate in Hz" }
                    },
                    "required": ["name"]
                }),
            },
            McpTool {
                name: "project_open".into(),
                description: "Open an existing project from disk".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path to project file or directory" }
                    },
                    "required": ["path"]
                }),
            },
            McpTool {
                name: "project_close".into(),
                description: "Close the current project and revert to an empty draft project".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "history_list".into(),
                description: "List undo/redo history stack entries".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "track_update".into(),
                description: "Update track properties (gain, pan, muted, solo, armed, name)".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Track ID" },
                        "name": { "type": "string", "description": "Track name" },
                        "gain": { "type": "number", "description": "Linear volume gain (1.0 = 0 dBFS)" },
                        "pan": { "type": "number", "description": "Stereo panning from -1.0 (left) to +1.0 (right)" },
                        "muted": { "type": "boolean", "description": "Whether track is muted" },
                        "solo": { "type": "boolean", "description": "Whether track is soloed" },
                        "armed": { "type": "boolean", "description": "Whether track is armed for recording" }
                    },
                    "required": ["track_id"]
                }),
            },
            McpTool {
                name: "clip_update".into(),
                description: "Update clip properties (gain, fades, start position)".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "Clip ID" },
                        "start_secs": { "type": "number", "description": "Timeline start in seconds" },
                        "gain": { "type": "number", "description": "Linear clip gain multiplier" },
                        "fade_in_secs": { "type": "number", "description": "Fade-in length in seconds" },
                        "fade_out_secs": { "type": "number", "description": "Fade-out length in seconds" }
                    },
                    "required": ["clip_id"]
                }),
            },
            McpTool {
                name: "clip_remove".into(),
                description: "Remove a clip from the timeline".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "Clip ID" }
                    },
                    "required": ["clip_id"]
                }),
            },
            McpTool {
                name: "marker_add".into(),
                description: "Add a project timeline marker".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Marker name" },
                        "position_secs": { "type": "number", "description": "Timeline position in seconds" }
                    },
                    "required": ["name", "position_secs"]
                }),
            },
            McpTool {
                name: "edit_batch".into(),
                description: "Execute several edits atomically: either every operation applies or the project is left untouched, and the whole batch becomes a single undo step. Operations run in array order.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "label": { "type": "string", "description": "Batch label for the audit log" },
                        "operations": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Operations to apply in order",
                            "items": {
                                "type": "object",
                                "required": ["op"],
                                "properties": {
                                    "op": {
                                        "type": "string",
                                        "enum": ["track_update", "clip_update", "clip_remove", "clip_split", "marker_add", "range_delete", "effect_apply", "effect_remove"]
                                    },
                                    "track_id": { "type": "string" },
                                    "clip_id": { "type": "string" },
                                    "effect_id": { "type": "string" },
                                    "name": { "type": "string" },
                                    "gain": { "type": "number" },
                                    "pan": { "type": "number" },
                                    "muted": { "type": "boolean" },
                                    "solo": { "type": "boolean" },
                                    "armed": { "type": "boolean" },
                                    "start_secs": { "type": "number" },
                                    "end_secs": { "type": "number" },
                                    "split_secs": { "type": "number" },
                                    "position_secs": { "type": "number" },
                                    "fade_in_secs": { "type": "number" },
                                    "fade_out_secs": { "type": "number" },
                                    "params": { "type": "object" }
                                }
                            }
                        }
                    },
                    "required": ["label", "operations"]
                }),
            },
            McpTool {
                name: "effect_catalog".into(),
                description: "List all built-in effects and presets available in the catalog".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "effect_apply".into(),
                description: "Insert an effect from the catalog into a track's chain. Unknown effect ids are rejected with the list".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Track ID" },
                        "effect_id": { "type": "string", "description": "Effect ID from catalog" },
                        "params": { "type": "object", "description": "Effect parameters" }
                    },
                    "required": ["track_id", "effect_id"]
                }),
            },
            McpTool {
                name: "effect_update".into(),
                description: "Update parameters of an already-inserted effect".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Track ID" },
                        "effect_id": { "type": "string", "description": "Effect ID" },
                        "params": { "type": "object", "description": "New parameters" }
                    },
                    "required": ["track_id", "effect_id", "params"]
                }),
            },
            McpTool {
                name: "effect_remove".into(),
                description: "Remove an effect from a track's effect chain".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Track ID" },
                        "effect_id": { "type": "string", "description": "Effect ID" }
                    },
                    "required": ["track_id", "effect_id"]
                }),
            },
            McpTool {
                name: "preset_apply".into(),
                description: "Replace a track's chain with a catalog preset (voice_cleanup, podcast_master). Unknown names are rejected with the list".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Track ID" },
                        "preset": { "type": "string", "description": "Preset name" }
                    },
                    "required": ["track_id", "preset"]
                }),
            },
            McpTool {
                name: "edit_trim_silence".into(),
                description: "Cut silence from a track below a dBFS threshold and ripple the track closed. Reports the measured length before/after.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Track ID" },
                        "threshold_db": { "type": "number", "description": "Threshold in dBFS" },
                        "min_silence_secs": { "type": "number", "description": "Minimum silence duration in seconds" }
                    },
                    "required": ["track_id", "threshold_db", "min_silence_secs"]
                }),
            },
            McpTool {
                name: "edit_shorten_pauses".into(),
                description: "Shorten pauses longer than max_pause_secs, keeping that much pause and cutting the excess. Ripples the track.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Track ID" },
                        "max_pause_secs": { "type": "number", "description": "Maximum pause duration in seconds" },
                        "crossfade_ms": { "type": "integer", "description": "Crossfade duration in milliseconds" }
                    },
                    "required": ["track_id", "max_pause_secs"]
                }),
            },
            McpTool {
                name: "edit_remove_fillers".into(),
                description: "Cut filler words using transcript word timestamps. Requires a speech model with word-level alignment: the call fails rather than guessing when only placeholder words exist.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Track ID" },
                        "filler_word_ids": { "type": "array", "items": { "type": "string" }, "description": "Optional list of filler word IDs" }
                    },
                    "required": ["track_id"]
                }),
            },
            McpTool {
                name: "edit_match_loudness".into(),
                description: "Scale every track's gain to reach the target LUFS, then re-measure. The gain is reduced if the sample peak would exceed -1 dBFS.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "target_lufs": { "type": "number", "description": "Target LUFS value" }
                    },
                    "required": ["target_lufs"]
                }),
            },
            McpTool {
                name: "analyze_silence".into(),
                description: "Find silence intervals of at least 100 ms below -30 dBFS in the project mix".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "analyze_quality".into(),
                description: "Measure integrated LUFS, sample peak, 10th-percentile noise floor, clipping spikes and the fraction of active 20 ms frames on the project mix".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "analyze_spectrum".into(),
                description: "Measure 16 log-spaced band levels averaged over blocks spread across the whole project mix".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "render_waveform".into(),
                description: "Return an ASCII waveform envelope of the project mix (one cell per column, real per-column peaks)".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "width": { "type": "integer", "description": "Width in pixels" },
                        "height": { "type": "integer", "description": "Height in pixels" }
                    },
                    "required": ["width", "height"]
                }),
            },
            McpTool {
                name: "render_spectrogram".into(),
                description: "Return an ASCII spectrogram of the project mix: Goertzel band energy per time column, low frequencies at the bottom".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "width": { "type": "integer", "description": "Width in pixels" },
                        "height": { "type": "integer", "description": "Height in pixels" }
                    },
                    "required": ["width", "height"]
                }),
            },
            McpTool {
                name: "compare_renders".into(),
                description: "Compare two rendered audio outputs and report similarity".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path_a": { "type": "string", "description": "Path to first audio file" },
                        "path_b": { "type": "string", "description": "Path to second audio file" }
                    },
                    "required": ["path_a", "path_b"]
                }),
            },
            McpTool {
                name: "export_presets".into(),
                description: "List available export presets".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "export_stems".into(),
                description: "Export one audio file per track, each rendered from that track alone (not the master copied per track)".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "output_dir": { "type": "string", "description": "Output directory path" },
                        "preset_id": { "type": "string", "description": "Export preset ID" }
                    },
                    "required": ["output_dir"]
                }),
            },
            McpTool {
                name: "export_transcript".into(),
                description: "Transcribe the project audio and write it as text, SRT or VTT. The report names the engine that produced the words".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "format": { "type": "string", "enum": ["text", "srt", "vtt"], "description": "Transcript format" },
                        "path": { "type": "string", "description": "Output file path" }
                    },
                    "required": ["format", "path"]
                }),
            },
            McpTool {
                name: "preview_render".into(),
                description: "Render a project segment to a WAV in the media directory and report that slice's own peak".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "start_secs": { "type": "number", "description": "Start position in seconds" },
                        "duration_secs": { "type": "number", "description": "Duration in seconds" }
                    },
                    "required": ["start_secs", "duration_secs"]
                }),
            },
            McpTool {
                name: "job_status".into(),
                description: "Get the state and progress of a job created by tts_generate, tts_script_generate, transcribe or an export".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "job_id": { "type": "string", "description": "Job ID" }
                    },
                    "required": ["job_id"]
                }),
            },
            McpTool {
                name: "job_cancel".into(),
                description: "Request cancellation of a running job; completed or unknown jobs report an error".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "job_id": { "type": "string", "description": "Job ID" }
                    },
                    "required": ["job_id"]
                }),
            },
            McpTool {
                name: "tts_script_generate".into(),
                description: "Synthesize a JSON script and lay the clips out on the \"Script\" track: {\"segments\":[{\"text\":...,\"voice_id\":...,\"speed\"?:1.0,\"gap_secs\"?:0.35}]}. One undo step.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "script_json": { "type": "string", "description": "Script JSON content" }
                    },
                    "required": ["script_json"]
                }),
            },
            McpTool {
                name: "tts_regenerate".into(),
                description: "Replace a clip's audio with a fresh synthesis of new text, keeping its timeline position.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "Clip ID" },
                        "new_text": { "type": "string", "description": "New text for the clip" },
                        "voice_id": { "type": "string", "description": "Voice profile ID" },
                        "speed": { "type": "number", "description": "Speed multiplier" }
                    },
                    "required": ["clip_id", "new_text", "voice_id"]
                }),
            },
        ];

    if tier >= PermissionTier::Edit {
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

    if tier >= PermissionTier::Export {
        tools.push(McpTool {
            name: "export_audio".into(),
            description:
                "Export audio master using a delivery preset ('podcast', 'audiobook', 'voice_over')"
                    .into(),
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

    tools
}
