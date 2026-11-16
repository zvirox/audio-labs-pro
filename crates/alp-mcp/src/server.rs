//! Core Model Context Protocol (MCP) server: transport, limits, and the
//! resource/prompt surfaces.
//!
//! Tool dispatch lives in [`crate::dispatch`]; this module owns everything that
//! wraps a call rather than defining one: the rate limit, the single project
//! lock, the optimistic-concurrency guard, the advisory time budget, and the
//! JSON-RPC envelope. Keeping those in one place means every tool is limited
//! the same way, and a new tool cannot bypass the guard by forgetting to
//! restate it.
//!
//! ## Limits
//!
//! All three boundaries are configured on `alp-mcp`'s command line and are
//! reported honestly in `server/discover`: the payload cap is enforced before
//! parsing, the rate limit is a sliding window pruned on every call, and the
//! time budget is advisory — a call that overruns still returns its real
//! result, with a note explaining the overrun. Nothing is silently dropped.

use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

use alp_api::{ApiSession, PermissionTier};

use crate::protocol::{JsonRpcRequest, JsonRpcResponse, ToolCallResult};

/// MCP Server handling JSON-RPC 2.0 requests over an active `ApiSession`.
pub struct McpServer {
    session: Arc<Mutex<ApiSession>>,
    tier: PermissionTier,
    limits: crate::limits::Limits,
    rate_limiter: Mutex<crate::limits::RateLimiter>,
    /// Shared `ttlMs`/`cacheScope` hints for the list surfaces.
    cache_hints: Value,
}

impl McpServer {
    /// Create a new server wrapping an active session.
    pub fn new(session: Arc<Mutex<ApiSession>>, tier: PermissionTier) -> Self {
        let limits = crate::limits::Limits::default();
        Self {
            session,
            tier,
            limits,
            rate_limiter: Mutex::new(crate::limits::RateLimiter::new(limits)),
            cache_hints: json!({ "ttlMs": 30_000, "cacheScope": "session" }),
        }
    }

    /// Replace the boundary limits (payload cap, call budget, rate budget).
    #[must_use]
    pub fn with_limits(mut self, limits: crate::limits::Limits) -> Self {
        self.limits = limits;
        self.rate_limiter = Mutex::new(crate::limits::RateLimiter::new(limits));
        self
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
        let (remaining, used) = self
            .rate_limiter
            .lock()
            .map(|mut limiter| {
                let remaining = limiter.remaining(std::time::Instant::now());
                (remaining, limiter.used())
            })
            .unwrap_or((0, 0));
        JsonRpcResponse::success(
            id,
            json!({
                "name": "Audio Labs Pro",
                "specRevision": "2026-07-28",
                "tier": format!("{:?}", self.tier),
                "features": ["tools", "resources", "prompts", "checkpoints", "undo_rollback"],
                "limits": {
                    "maxPayloadBytes": self.limits.max_payload_bytes,
                    "callTimeoutMs": self.limits.call_timeout.as_millis(),
                    "requestsPerWindow": self.limits.requests_per_window,
                    "windowSecs": self.limits.window.as_secs(),
                    "remainingInWindow": remaining,
                    "usedInWindow": used
                },
                "pagination": {
                    "defaultLimit": alp_api::DEFAULT_PAGE_SIZE,
                    "maxLimit": alp_api::MAX_PAGE_SIZE,
                    "tools": ["project_list", "track_list", "media_list", "history_list", "plugin_list"]
                }
            }),
        )
    }

    fn handle_tools_list(&self, id: Value) -> JsonRpcResponse {
        let tools = crate::tool_defs::tools_for_tier(self.tier);
        let mut result = json!({ "tools": tools });
        if let Some(object) = result.as_object_mut() {
            object.insert("ttlMs".into(), self.cache_hints["ttlMs"].clone());
            object.insert("cacheScope".into(), self.cache_hints["cacheScope"].clone());
        }
        JsonRpcResponse::success(id, result)
    }

    fn handle_tool_call(&self, id: Value, params: Value) -> JsonRpcResponse {
        let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

        // Rate limiting runs before the session lock so a flooding client
        // cannot starve legitimate calls of the mutex.
        if let Ok(mut limiter) = self.rate_limiter.lock()
            && let Err(error) = limiter.admit(std::time::Instant::now())
        {
            return JsonRpcResponse::success(
                id,
                serde_json::to_value(ToolCallResult::failure(error.to_string())).unwrap(),
            );
        }

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

        // Optimistic concurrency: a write may name the revision it expects.
        // Reads ignore the argument, since a read cannot invalidate a client's
        // view of the project — a stale guard on a read would make the very
        // call an agent needs to recover (`project_info`) fail.
        let guards_state = !crate::tool_defs::metadata_for(tool_name)
            .annotations
            .read_only;
        if let Some(expected) = arguments.get("expected_revision").and_then(Value::as_u64)
            && guards_state
            && let Err(error) = session.verify_revision(Some(expected))
        {
            return JsonRpcResponse::success(
                id,
                serde_json::to_value(ToolCallResult::failure(format!(
                    "{error} Current state is unchanged; call project_info to re-read the revision."
                )))
                .unwrap(),
            );
        }

        // Measure the call against the advisory budget. The measurement is
        // real: the note on an over-budget result states what actually
        // happened instead of pretending the call was fast.
        // The dispatch table lives in `dispatch.rs`: this function owns the
        // rate limit, the lock, the concurrency guard and the budget note, and
        // hands the actual tool→operation mapping to a module that is only
        // about that mapping.
        let started = std::time::Instant::now();
        let result = crate::dispatch::dispatch(&mut session, tool_name, &arguments);

        let outcome = crate::limits::TimedOutcome::record(result, started.elapsed());
        let mut result = outcome.value;
        if let Err(error) = self.limits.check_elapsed(outcome.elapsed) {
            result = result.with_note(format!(
                "Note: this call took {} ms, over budget. {error}. \
                 The result above is real; reduce the work or raise --call-timeout-ms \
                 to avoid the overrun.",
                outcome.elapsed.as_millis()
            ));
        }

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
                        "text": include_str!("../../../docs/mcp/AGENT_GUIDE.md")
                    }]
                }),
            ),
            "alp://effects/catalog" => {
                let catalog = self
                    .session
                    .lock()
                    .map(|session| session.effect_catalog())
                    .unwrap_or_else(|_| {
                        Err(alp_api::ApiError::Validation(
                            "failed to acquire session lock".into(),
                        ))
                    });
                match catalog {
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
                        format!("failed to read the effect catalog: {e}"),
                        None,
                    ),
                }
            }
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
                    "description": "Clean up a voice recording: measure, clean, trim, master and export",
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": "Execute the voice-cleanup recipe:\n1. Call 'project_info' and 'media_list' to understand the session.\n2. Call 'analyze_loudness', 'analyze_silence' and 'analyze_quality' and report what you measured.\n3. Call 'checkpoint_create' with label 'before-cleanup'.\n4. Call 'preset_apply' with 'voice_cleanup' on the vocal track.\n5. Call 'edit_trim_silence' (threshold_db -45, min_silence_secs 0.25) and then 'edit_shorten_pauses' if pauses remain long.\n6. Call 'edit_match_loudness' with -16 LUFS.\n7. Re-run the analysis tools and compare; if the result regressed, call 'checkpoint_restore'.\n8. Call 'export_audio' with the 'podcast' preset."
                            }
                        }
                    ]
                }),
            ),
            "produce_podcast_episode" => JsonRpcResponse::success(
                id,
                json!({
                    "description": "Assemble a podcast episode from a script or recording and master it",
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": "Execute the podcast-episode recipe:\n1. Call 'project_create' (or 'project_open' for an existing session) and 'checkpoint_create'.\n2. Import speech with 'media_import', then place it with 'clip_add' on a 'Voice' track ('track_add' first if needed).\n3. Add beds with 'media_import' + 'clip_add' on a second track, then lower them under speech with 'track_update' gain or 'edit_batch'.\n4. Call 'analyze_loudness' and 'edit_match_loudness' targeting -16 LUFS for stereo.\n5. Call 'analyze_quality' and fix clipping with 'clip_update' gain changes.\n6. Call 'export_stems' for the archival mix and 'export_audio' with the 'podcast' preset for delivery.\n7. Report the measured loudness, duration and output paths."
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
