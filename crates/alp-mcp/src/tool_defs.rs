//! Tool definitions and JSON schemas for Audio Labs Pro MCP server.

use alp_api::PermissionTier;
use serde_json::{Value, json};

use crate::protocol::{McpTool, ToolAnnotations, ToolMetadata};

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
            McpTool {
                name: "timeline_spectral_analyze".into(),
                description: "Compute high-resolution STFT spectrogram for an audio clip (FFT sizes 512-8192, dBFS power spectral density, linear/log/mel scales)".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "Clip ID" },
                        "fft_size": { "type": "integer", "description": "FFT size (512, 1024, 2048, 4096, 8192)" },
                        "window_type": { "type": "string", "enum": ["hanning", "hamming", "blackman_harris", "rectangular"], "description": "Window function" },
                        "freq_scale": { "type": "string", "enum": ["linear", "logarithmic", "mel"], "description": "Frequency scale" },
                        "num_freq_bins": { "type": "integer", "description": "Target number of frequency display bins" }
                    },
                    "required": ["clip_id"]
                }),
            },
            McpTool {
                name: "timeline_spectral_select".into(),
                description: "Define 2D time-frequency spectral selection box on a track, calculating geometric center frequency and filter Q factor".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Track ID" },
                        "t_start": { "type": "number", "description": "Start time in seconds" },
                        "t_end": { "type": "number", "description": "End time in seconds" },
                        "f_low": { "type": "number", "description": "Lower frequency boundary in Hz" },
                        "f_high": { "type": "number", "description": "Upper frequency boundary in Hz" }
                    },
                    "required": ["track_id", "t_start", "t_end", "f_low", "f_high"]
                }),
            },
            McpTool {
                name: "timeline_set_view_mode".into(),
                description: "Set visualization mode for a track: waveform, spectrogram, or split multi-view".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Track ID" },
                        "view_mode": { "type": "string", "enum": ["waveform", "spectrogram", "split"], "description": "View mode" }
                    },
                    "required": ["track_id", "view_mode"]
                }),
            },
            McpTool {
                name: "effects_response".into(),
                description: "Compute the combined magnitude/phase response of an EQ band chain from explicit band settings (pure calculation; does not read or modify the project).".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "bands": {
                            "type": "array",
                            "description": "EQ bands to evaluate",
                            "items": {
                                "type": "object",
                                "required": ["filter_type", "frequency_hz", "q", "gain_db", "enabled"],
                                "properties": {
                                    "filter_type": { "type": "string", "description": "highpass, lowpass, peaking, lowshelf, highshelf, notch or bandpass" },
                                    "frequency_hz": { "type": "number" },
                                    "q": { "type": "number" },
                                    "gain_db": { "type": "number" },
                                    "enabled": { "type": "boolean" }
                                }
                            }
                        },
                        "sample_rate": { "type": "integer", "description": "Analysis sample rate in Hz" },
                        "points": { "type": "integer", "description": "Log-spaced sample count, clamped to 16..=512" }
                    },
                    "required": ["bands", "sample_rate"]
                }),
            },
            McpTool {
                name: "acx_qa_analyze".into(),
                description: "Analyse chapter audio files against ACX targets (RMS -23..-18 dBFS, true peak <= -3 dBFS, noise floor <= -60 dBFS). Paths must be inside the workspace.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "chapters": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "type": "object",
                                "required": ["title", "audio_path"],
                                "properties": {
                                    "title": { "type": "string" },
                                    "audio_path": { "type": "string", "description": "Workspace path to a decodable audio file" }
                                }
                            }
                        }
                    },
                    "required": ["chapters"]
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
                    name: "clip_set_pan_envelope".into(),
                    description: "Set stereo pan automation envelope points for a clip".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "clip_id": { "type": "string", "description": "Clip ID" },
                            "points": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "offset_secs": { "type": "number", "description": "Time offset in seconds from clip start" },
                                        "pan": { "type": "number", "description": "Stereo pan value (-1.0 to 1.0)" }
                                    },
                                    "required": ["offset_secs", "pan"]
                                },
                                "description": "Pan automation points"
                            }
                        },
                        "required": ["clip_id", "points"]
                    }),
                },
                McpTool {
                    name: "clip_slip".into(),
                    description: "Slip audio content within fixed clip boundaries by adjusting source audio offset".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "clip_id": { "type": "string", "description": "Clip ID" },
                            "delta_secs": { "type": "number", "description": "Time delta in seconds to slip audio source" }
                        },
                        "required": ["clip_id", "delta_secs"]
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
                McpTool {
                    name: "voice_profile_update".into(),
                    description: "Update a stored voice profile's name, tags and default parameters (speed, pitch, energy, emotion). Profile kind, backend and consent records are not mutable: cloning is never creatable through MCP, and a locked profile refuses mutation until it is unlocked.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "voice_id": { "type": "string", "description": "Voice profile handle (see tts_voices)" },
                            "name": { "type": "string", "description": "New display name (non-empty)" },
                            "tags": { "type": "array", "items": { "type": "string" }, "description": "New descriptive tags" },
                            "default_params": {
                                "type": "object",
                                "description": "Partial default-parameter update (speed, pitch, energy, emotion); omitted fields keep stored values",
                                "properties": {
                                    "speed": { "type": "number", "description": "Speed multiplier, 0.25 to 4.0 (1.0 neutral)" },
                                    "pitch": { "type": "number", "description": "Pitch adjustment in semitones (0.0 neutral)" },
                                    "energy": { "type": "number", "description": "Energy / vocal effort (1.0 neutral)" },
                                    "emotion": { "type": "string", "description": "Expressive emotion / style tag" }
                                }
                            }
                        },
                        "required": ["voice_id"]
                    }),
                },
                McpTool {
                    name: "voice_profile_lock".into(),
                    description: "Lock a stored voice profile so it refuses agent-visible mutation until it is unlocked.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "voice_id": { "type": "string", "description": "Voice profile handle (see tts_voices)" }
                        },
                        "required": ["voice_id"]
                    }),
                },
                McpTool {
                    name: "voice_profile_unlock".into(),
                    description: "Unlock a stored voice profile so agent-visible mutation is allowed again.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "voice_id": { "type": "string", "description": "Voice profile handle (see tts_voices)" }
                        },
                        "required": ["voice_id"]
                    }),
                },
                McpTool {
                    name: "tts_batch".into(),
                    description: "Synthesize a batch of speech requests (batch parity: enqueue a text list). One background job per request; jobs run sequentially and job_cancel stops the batch at the next segment boundary. Returns one report per synthesized clip with its job id.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "requests": {
                                "type": "array",
                                "minItems": 1,
                                "description": "Items to synthesize, in order",
                                "items": {
                                    "type": "object",
                                    "required": ["voice_id", "text"],
                                    "properties": {
                                        "voice_id": { "type": "string", "description": "Voice profile handle (see tts_voices)" },
                                        "text": { "type": "string", "description": "Text to speak" },
                                        "speed": { "type": "number", "description": "Speed multiplier, 0.25 to 4.0 (default 1.0)" }
                                    }
                                }
                            }
                        },
                        "required": ["requests"]
                    }),
                },
                McpTool {
                    name: "marker_remove".into(),
                    description: "Remove a timeline marker by id. One undo step.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "marker_id": { "type": "string", "description": "Marker id from marker_add" }
                        },
                        "required": ["marker_id"]
                    }),
                },
                McpTool {
                    name: "loop_range_set".into(),
                    description: "Set the project loop range in seconds, or clear it when both times are omitted. One undo step.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "start_secs": { "type": "number" },
                            "end_secs": { "type": "number" }
                        }
                    }),
                },
                McpTool {
                    name: "clip_trim".into(),
                    description: "Trim a clip by replacing its source frame range and timeline start frame. One undo step.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "clip_id": { "type": "string" },
                            "source_start_frame": { "type": "integer" },
                            "source_end_frame": { "type": "integer" },
                            "timeline_start_frame": { "type": "integer" }
                        },
                        "required": ["clip_id", "source_start_frame", "source_end_frame", "timeline_start_frame"]
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
        tools.push(McpTool {
            name: "audiobook_encode".into(),
            description: "Encode a full audiobook from a script (audiobook parity: synthesize each segment with its voice, place the clips on a dedicated Audiobook track, and export the master with the audiobook delivery preset). Script JSON: {\"segments\":[{\"text\":...,\"voice_id\":...,\"speed\"?,\"gap_secs\"?}]} — the same contract as tts_script_generate.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "script_json": {
                        "type": "string",
                        "description": "Audiobook script JSON ({\"segments\":[...]} or a bare array of segments)"
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Destination path for the exported master (default: generated under the project renders directory)"
                    }
                },
                "required": ["script_json"]
            }),
        });
        tools.push(McpTool {
            name: "dsp_macro_execute".into(),
            description: "Run an offline DSP macro over a directory of audio files as a cancellable job. Effects use the built-in catalog ids (gain, parametric_eq, compressor, limiter, de_esser, de_hum, de_click, noise_gate, spectral_notch, harmonic_suppressor). Returns a job_id plus the per-file report.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input_dir": { "type": "string", "description": "Workspace directory containing source audio files" },
                    "output_dir": { "type": "string", "description": "Workspace directory for exported WAV results" },
                    "pattern": { "type": "string", "description": "Optional filename substring filter" },
                    "overwrite": { "type": "boolean", "description": "Replace existing outputs (default false)" },
                    "steps": {
                        "type": "array",
                        "minItems": 1,
                        "description": "Ordered DSP effects",
                        "items": {
                            "type": "object",
                            "required": ["effect_id"],
                            "properties": {
                                "effect_id": { "type": "string", "description": "Built-in effect id" },
                                "params": { "type": "object", "description": "Effect parameters by name" }
                            }
                        }
                    }
                },
                "required": ["input_dir", "output_dir", "steps"]
            }),
        });
        tools.push(McpTool {
            name: "dubbing_fit_rate".into(),
            description: "Fit a rendered take to a dialogue window with WSOLA rate adjustment and write a WAV. Paths must be inside the workspace.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "audio_path": { "type": "string" },
                    "target_duration_secs": { "type": "number" },
                    "output_path": { "type": "string" }
                },
                "required": ["audio_path", "target_duration_secs", "output_path"]
            }),
        });
        tools.push(McpTool {
            name: "dubbing_remux".into(),
            description: "Remux a video with a dubbed audio track without re-encoding video (requires ffmpeg/ffprobe). Paths must be inside the workspace.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "video_path": { "type": "string" },
                    "audio_path": { "type": "string" },
                    "output_path": { "type": "string" },
                    "subtitle_path": { "type": "string", "description": "Optional side file, not burned in" }
                },
                "required": ["video_path", "output_path"]
            }),
        });
        tools.push(McpTool {
            name: "dubbing_separate_stems".into(),
            description: "Separate a mixed audio file into four WAV stems (vocals, drums, bass, other) via the alp-infer Demucs runner. Paths must be inside the workspace.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "audio_path": { "type": "string", "description": "Mixed input audio path inside the workspace" },
                    "output_dir": { "type": "string", "description": "Destination workspace directory for stem WAV files" }
                },
                "required": ["audio_path", "output_dir"]
            }),
        });
        tools.push(McpTool {
            name: "dsp_spectral_repair".into(),
            description: "Offline spectral repair of a WAV: notch a narrow band or suppress a fundamental and its overtones, writing a new WAV.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input_path": { "type": "string" },
                    "output_path": { "type": "string" },
                    "mode": { "type": "string", "enum": ["notch", "harmonic"] },
                    "center_hz": { "type": "number", "description": "Notch centre, default 1000" },
                    "fundamental_hz": { "type": "number", "description": "Harmonic fundamental, default 220" },
                    "harmonics": { "type": "integer", "description": "1..=64, default 4" },
                    "bandwidth_q": { "type": "number", "description": "Default 12" },
                    "reduction_db": { "type": "number", "description": "Default 24" }
                },
                "required": ["input_path", "output_path", "mode"]
            }),
        });
        tools.push(McpTool {
            name: "dsp_time_stretch".into(),
            description:
                "Offline WSOLA time stretch of a WAV (0 < ratio <= 4) without changing pitch."
                    .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input_path": { "type": "string" },
                    "output_path": { "type": "string" },
                    "ratio": { "type": "number" }
                },
                "required": ["input_path", "output_path", "ratio"]
            }),
        });
        tools.push(McpTool {
            name: "audiobook_export_m4b".into(),
            description: "Export a rendered master audio file as a chaptered M4B audiobook with optional cover art and metadata. Paths must be inside the workspace.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "output_path": { "type": "string", "description": "Destination .m4b path inside the workspace" },
                    "master_audio_path": { "type": "string", "description": "Rendered full-book audio file" },
                    "chapters": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "required": ["title", "start_secs", "duration_secs"],
                            "properties": {
                                "title": { "type": "string" },
                                "start_secs": { "type": "number" },
                                "duration_secs": { "type": "number" }
                            }
                        }
                    },
                    "cover_path": { "type": "string", "description": "Optional JPEG or PNG cover inside the workspace" },
                    "metadata": {
                        "type": "object",
                        "properties": {
                            "title": { "type": "string" },
                            "narrator": { "type": "string" },
                            "series": { "type": "string" },
                            "series_number": { "type": "integer" },
                            "comment": { "type": "string" }
                        }
                    }
                },
                "required": ["output_path", "master_audio_path", "chapters"]
            }),
        });
    }

    tools.extend(crate::transport_tools::definitions(tier));

    for tool in &mut tools {
        decorate_schema(&tool.name, &mut tool.input_schema);
    }

    tools
}

/// Tools whose result is a list and therefore accepts `limit`/`cursor`.
pub const PAGED_TOOLS: [&str; 5] = [
    "project_list",
    "track_list",
    "media_list",
    "history_list",
    "plugin_list",
];

/// Tools that mutate project state, and therefore accept `expected_revision`.
///
/// This is derived from the annotation table at runtime rather than hand-listed:
/// anything that is not read-only may be called optimistically. File-writing
/// export tools are included, since they also read the current mix.
fn accepts_expected_revision(name: &str) -> bool {
    // Transport tools control live device state, not the project snapshot.
    !crate::transport_tools::TRANSPORT_TOOLS.contains(&name)
        && !metadata_for(name).annotations.read_only
}

/// Add the cross-cutting arguments to a tool's input schema.
///
/// Every list tool gets `limit`/`cursor`, every write tool gets
/// `expected_revision`. Doing it here keeps 58 hand-written schemas from having
/// to restate the same three properties, and makes the contract impossible to
/// forget on a new tool.
fn decorate_schema(name: &str, schema: &mut Value) {
    let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) else {
        return;
    };

    if PAGED_TOOLS.contains(&name) {
        properties.insert(
            "limit".into(),
            json!({
                "type": "integer",
                "minimum": 1,
                "maximum": alp_api::MAX_PAGE_SIZE,
                "description": format!(
                    "Maximum items to return (default {}, max {}). The result reports `total` and, when more remain, `next_cursor`.",
                    alp_api::DEFAULT_PAGE_SIZE,
                    alp_api::MAX_PAGE_SIZE
                )
            }),
        );
        properties.insert(
            "cursor".into(),
            json!({
                "type": "string",
                "description": "Opaque `next_cursor` from the previous page. Omit to start at the first item."
            }),
        );
    }

    if accepts_expected_revision(name) {
        properties.insert(
            "expected_revision".into(),
            json!({
                "type": "integer",
                "minimum": 1,
                "description": "Optimistic concurrency guard: fail without changing anything unless the project is still at this revision (see project_info)."
            }),
        );
    }
}

/// Behavioural metadata for one tool: a short title plus MCP annotations.
///
/// The table is the single place that states how a tool behaves. A test asserts
/// every advertised tool appears here, so a new tool cannot ship without
/// declaring whether it reads, adds, or destroys.
pub fn metadata_for(name: &str) -> ToolMetadata {
    let read = ToolAnnotations::read_only();
    let add = ToolAnnotations::additive();
    let destroy = ToolAnnotations::destructive();
    let export_add = ToolAnnotations {
        read_only: false,
        destructive: true,
        idempotent: false,
        open_world: false,
    };

    let (title, annotations) = match name {
        // --- Phase 19 studio coverage ---
        "effects_response" => ("EQ Frequency Response", read),
        "acx_qa_analyze" => ("ACX Chapter QA", read),
        "audiobook_export_m4b" => (
            "Write M4B Audiobook",
            ToolAnnotations {
                read_only: false,
                destructive: true,
                idempotent: false,
                open_world: false,
            },
        ),
        "marker_remove" => ("Remove Marker", destroy),
        "loop_range_set" => ("Set Loop Range", add),
        "clip_trim" => ("Trim Clip", destroy),
        "dubbing_fit_rate" => ("Fit Dubbing Take", export_add),
        "dubbing_remux" => ("Remux Dubbed Video", export_add),
        "dubbing_separate_stems" => ("Separate 4 Audio Stems", export_add),
        "dsp_spectral_repair" => ("Spectral Repair", export_add),
        "dsp_time_stretch" => ("Time Stretch Audio", export_add),
        // --- Live transport (attached mode only) ---
        "transport_state" => ("Transport State", read),
        "transport_play" => ("Start Playback", add),
        "transport_pause" => ("Pause Playback", add),
        "transport_stop" => ("Stop Playback", destroy),
        "transport_seek" => ("Seek Playhead", add),
        "transport_set_volume" => ("Set Master Volume", add),
        "transport_set_loop" => ("Set Transport Loop", add),
        // --- Projects & history ---
        "project_list" => ("List Projects", read),
        "project_info" => ("Project Summary", read),
        "project_create" => ("Create Project", add),
        "project_open" => (
            "Open Project",
            ToolAnnotations {
                read_only: false,
                destructive: false,
                idempotent: true,
                open_world: false,
            },
        ),
        "project_save" => (
            "Save Project",
            ToolAnnotations {
                read_only: false,
                destructive: false,
                idempotent: true,
                open_world: false,
            },
        ),
        "project_close" => ("Close Project", destroy),
        "history_list" => ("List History", read),
        "history_undo" => ("Undo Last Edit", destroy),
        "history_redo" => ("Redo Edit", add),
        "checkpoint_create" => ("Create Checkpoint", add),
        "checkpoint_restore" => ("Restore Checkpoint", destroy),
        "rollback_session" => ("Roll Back Session", destroy),
        // --- Media ---
        "media_list" => ("List Media", read),
        "media_import" => ("Import Media", add),
        // --- Tracks & clips ---
        "track_list" => ("List Tracks", read),
        "track_add" => ("Add Track", add),
        "track_update" => (
            "Update Track",
            ToolAnnotations {
                read_only: false,
                destructive: false,
                idempotent: true,
                open_world: false,
            },
        ),
        "track_remove" => ("Remove Track", destroy),
        "clip_add" => ("Add Clip", add),
        "clip_update" => (
            "Update Clip",
            ToolAnnotations {
                read_only: false,
                destructive: false,
                idempotent: true,
                open_world: false,
            },
        ),
        "clip_split" => ("Split Clip", add),
        "clip_remove" => ("Remove Clip", destroy),
        "range_delete" => ("Delete Range", destroy),
        "marker_add" => ("Add Marker", add),
        "edit_batch" => ("Apply Edit Batch", destroy),
        // --- Effects ---
        "effect_catalog" => ("List Effects", read),
        "effect_apply" => ("Apply Effect", add),
        "effect_update" => (
            "Update Effect",
            ToolAnnotations {
                read_only: false,
                destructive: false,
                idempotent: true,
                open_world: false,
            },
        ),
        "effect_remove" => ("Remove Effect", destroy),
        "preset_apply" => ("Apply Preset", add),
        // --- Smart edits ---
        "edit_trim_silence" => ("Trim Silence", destroy),
        "edit_shorten_pauses" => ("Shorten Pauses", destroy),
        "edit_remove_fillers" => ("Remove Filler Words", destroy),
        "edit_match_loudness" => (
            "Match Loudness",
            ToolAnnotations {
                read_only: false,
                destructive: false,
                idempotent: true,
                open_world: false,
            },
        ),
        // --- Analysis ---
        "analyze_loudness" | "analyze_silence" | "analyze_quality" | "analyze_spectrum" => {
            ("Analyze Mix", read)
        }
        "render_waveform" => ("Render Waveform", read),
        "render_spectrogram" => ("Render Spectrogram", read),
        "timeline_spectral_analyze" => ("Analyze Spectrogram", read),
        "timeline_spectral_select" => ("Select Spectral Region", read),
        "timeline_set_view_mode" => ("Set Track View Mode", read),
        "clip_set_pan_envelope" => ("Set Clip Pan Envelope", add),
        "clip_slip" => ("Slip Clip", add),
        "compare_renders" => ("Compare Renders", read),
        // --- Export ---
        "export_presets" => ("List Export Presets", read),
        "export_audio" | "export_stems" | "export_transcript" | "preview_render" => (
            "Write Audio File",
            ToolAnnotations {
                read_only: false,
                destructive: true,
                idempotent: false,
                open_world: false,
            },
        ),
        "dsp_macro_execute" => (
            "Run DSP Macro Batch",
            ToolAnnotations {
                read_only: false,
                destructive: true,
                idempotent: false,
                open_world: true,
            },
        ),
        // --- Jobs ---
        "job_status" => ("Job Status", read),
        "job_cancel" => ("Cancel Job", destroy),
        // --- Voice & script ---
        "tts_voices" => ("List Voices", read),
        "voice_profile_get" => ("Inspect Voice", read),
        "tts_generate" => ("Synthesize Speech", add),
        "tts_script_generate" => ("Synthesize Script", add),
        "tts_regenerate" => ("Regenerate Clip", destroy),
        "voice_design" => ("Design Voice", add),
        "transcribe" => (
            "Transcribe Audio",
            ToolAnnotations {
                read_only: false,
                destructive: false,
                idempotent: true,
                open_world: false,
            },
        ),
        // --- Plugins ---
        "plugin_list" => (
            "List Plugins",
            ToolAnnotations {
                read_only: true,
                destructive: false,
                idempotent: true,
                // Host-wide scan of installed CLAP/LV2 bundles.
                open_world: true,
            },
        ),
        "plugin_insert" => ("Insert Plugin", add),
        "plugin_param_set" => (
            "Set Plugin Parameter",
            ToolAnnotations {
                read_only: false,
                destructive: false,
                idempotent: true,
                open_world: false,
            },
        ),
        // --- Voice & profile parity (alpha.6) ---
        "voice_profile_update" => ("Update Voice Profile", add),
        "voice_profile_lock" => ("Lock Voice Profile", add),
        "voice_profile_unlock" => ("Unlock Voice Profile", add),
        "tts_batch" => ("Synthesize Batch", add),
        "audiobook_encode" => (
            "Encode Audiobook",
            ToolAnnotations {
                read_only: false,
                destructive: true,
                idempotent: false,
                open_world: false,
            },
        ),
        other => {
            // Unknown ids keep the conservative answer: a client that sees this
            // should not assume the call is safe to retry. The name is still
            // reported in the tool entry itself, so no information is lost.
            let _ = other;
            (
                "Unknown Tool",
                ToolAnnotations {
                    read_only: false,
                    destructive: true,
                    idempotent: false,
                    open_world: true,
                },
            )
        }
    };

    ToolMetadata {
        title: title.into(),
        annotations,
    }
}
