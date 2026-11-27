//! Model Context Protocol (MCP) server binary for Audio Labs Pro.
//!
//! Exposes non-destructive multitrack editing, vocal cleanup DSP, speech
//! synthesis, and master audio export to external AI models and IDE agents.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use alp_api::{ApiSession, AuditLogger, PermissionTier, WorkspaceSandbox};
use alp_core::{Project, ProjectPaths};
use clap::Parser;

mod attach;

use alp_mcp::{JsonRpcRequest, JsonRpcResponse, McpServer, limits, protocol};

#[derive(Parser, Debug)]
#[command(
    name = "alp-mcp",
    version,
    about = "Audio Labs Pro Model Context Protocol Server"
)]
struct Args {
    /// Workspace root directory.
    #[arg(short, long, default_value = ".")]
    workspace: PathBuf,

    /// Permission tier for this session (observe, edit, export).
    #[arg(short, long, default_value = "export")]
    tier: String,

    /// Unique client or agent session identifier.
    #[arg(short, long, default_value = "mcp-agent")]
    client_id: String,

    /// Attach to the running desktop instead of opening a project from disk.
    #[arg(long)]
    attach: bool,

    /// Desktop control socket. Defaults to `$XDG_RUNTIME_DIR/alp/control.sock`.
    #[arg(long, requires = "attach")]
    control_socket: Option<PathBuf>,

    /// Maximum size of one JSON-RPC request line, in bytes.
    #[arg(long, default_value = "1048576")]
    max_payload_bytes: usize,

    /// Advisory time budget for one tool call, in milliseconds. Calls that
    /// exceed it report `over_budget` instead of a fabricated success.
    #[arg(long, default_value = "120000")]
    call_timeout_ms: u64,

    /// Tool calls allowed per client per rate-limit window.
    #[arg(long, default_value = "600")]
    requests_per_window: u32,

    /// Rate-limit window length, in seconds.
    #[arg(long, default_value = "60")]
    rate_window_secs: u64,
}

/// Parse a permission-tier CLI value. Unknown values fail closed instead of
/// silently escalating to the most permissive tier.
fn parse_tier(value: &str) -> anyhow::Result<PermissionTier> {
    match value.trim().to_lowercase().as_str() {
        "observe" => Ok(PermissionTier::Observe),
        "edit" => Ok(PermissionTier::Edit),
        "export" => Ok(PermissionTier::Export),
        other => Err(anyhow::anyhow!(
            "unknown permission tier '{other}': use observe, edit or export"
        )),
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let tier = parse_tier(&args.tier)?;
    if args.attach {
        return attach::run(&args);
    }

    let workspace_path = args
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| args.workspace.clone());
    std::fs::create_dir_all(&workspace_path)?;

    let proj_paths = ProjectPaths::new(&workspace_path);
    proj_paths.ensure_layout()?;

    let project = match proj_paths.load() {
        Ok(p) => p,
        Err(_) => {
            let p = Project::new("MCP Session Project")?;
            proj_paths.save(&p)?;
            p
        }
    };

    let sandbox = WorkspaceSandbox::new([&workspace_path])?;
    let audit_file = workspace_path.join("audit.jsonl");
    let audit = AuditLogger::new(&audit_file)?;

    let session = ApiSession::new(project, proj_paths, tier, sandbox, audit, &args.client_id)?;

    let limits = limits::Limits {
        max_payload_bytes: args.max_payload_bytes,
        call_timeout: std::time::Duration::from_millis(args.call_timeout_ms),
        requests_per_window: args.requests_per_window,
        window: std::time::Duration::from_secs(args.rate_window_secs.max(1)),
    };

    let server = McpServer::new(Arc::new(Mutex::new(session)), tier).with_limits(limits);

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Err(error) = limits.check_payload(trimmed.len()) {
            let json_out = serde_json::to_string(&JsonRpcResponse::success(
                serde_json::Value::Null,
                serde_json::to_value(protocol::ToolCallResult::failure(error.to_string()))?,
            ))?;
            writeln!(stdout, "{json_out}")?;
            stdout.flush()?;
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => server.handle_request(req),
            Err(e) => JsonRpcResponse::error(
                serde_json::Value::Null,
                -32700,
                format!("Parse error: {e}"),
                None,
            ),
        };

        let json_out = serde_json::to_string(&response)?;
        writeln!(stdout, "{json_out}")?;
        stdout.flush()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tempfile::tempdir;

    #[test]
    fn server_initialization_and_tool_discovery() {
        let dir = tempdir().expect("tempdir");
        let proj_paths = ProjectPaths::new(dir.path());
        proj_paths.ensure_layout().expect("layout");
        let project = Project::new("Test").expect("project");

        let sandbox = WorkspaceSandbox::new([dir.path()]).expect("sandbox");
        let audit = AuditLogger::new(dir.path().join("audit.jsonl")).expect("audit");
        let session = ApiSession::new(
            project,
            proj_paths,
            PermissionTier::Export,
            sandbox,
            audit,
            "agent",
        )
        .expect("session");

        let server = McpServer::new(Arc::new(Mutex::new(session)), PermissionTier::Export);

        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "initialize".into(),
            params: serde_json::Value::Null,
        };
        let resp = server.handle_request(init_req);
        assert!(resp.result.is_some());

        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(2)),
            method: "tools/list".into(),
            params: serde_json::Value::Null,
        };
        let list_resp = server.handle_request(list_req);
        let result_val = list_resp.result.unwrap();
        let tools = result_val.get("tools").unwrap().as_array().unwrap();
        assert!(tools.iter().any(|t| t["name"] == "project_info"));
        assert!(tools.iter().any(|t| t["name"] == "clip_split"));
        assert!(tools.iter().any(|t| t["name"] == "export_audio"));
    }

    fn export_session_server() -> McpServer {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().to_path_buf();
        std::mem::forget(dir); // keep the tempdir alive for the process lifetime
        let proj_paths = ProjectPaths::new(&path);
        proj_paths.ensure_layout().expect("layout");
        let project = Project::new("Export Test").expect("project");
        let sandbox = WorkspaceSandbox::new([&path]).expect("sandbox");
        let audit = AuditLogger::new(path.join("audit.jsonl")).expect("audit");
        let session = ApiSession::new(
            project,
            proj_paths,
            PermissionTier::Export,
            sandbox,
            audit,
            "agent",
        )
        .expect("session");
        McpServer::new(Arc::new(Mutex::new(session)), PermissionTier::Export)
    }

    #[test]
    fn export_audio_advertises_only_real_presets() {
        let server = export_session_server();
        let resp = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "tools/list".into(),
            params: serde_json::Value::Null,
        });
        let tools = resp.result.expect("result")["tools"]
            .as_array()
            .expect("tools")
            .clone();
        let export_tool = tools
            .iter()
            .find(|t| t["name"] == "export_audio")
            .expect("export tool");
        let enum_values: Vec<&str> = export_tool["inputSchema"]["properties"]["preset"]["enum"]
            .as_array()
            .expect("preset enum")
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(
            enum_values,
            vec!["podcast", "audiobook", "voice_over"],
            "the advertised preset enum must match what the dispatcher actually supports"
        );
    }

    #[test]
    fn unknown_export_preset_is_rejected_not_silently_mapped() {
        let server = export_session_server();
        let resp = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(2)),
            method: "tools/call".into(),
            params: serde_json::json!({
                "name": "export_audio",
                "arguments": { "preset": "not_a_preset", "output_path": "out.wav" }
            }),
        });
        let result = resp.result.expect("tool result payload");
        assert_eq!(
            result["isError"], true,
            "an unknown preset must be reported as a tool error, not silently mapped to a delivery preset"
        );
    }

    #[test]
    fn unknown_tier_fails_closed() {
        assert!(parse_tier("observe").is_ok());
        assert!(parse_tier("EDIT").is_ok());
        assert!(parse_tier("export ").is_ok());
        assert!(
            parse_tier("Observatory").is_err(),
            "typos must not escalate to the most permissive tier"
        );
        assert!(parse_tier("").is_err());
    }

    /// Sandbox- and store-isolated session server used by the voice and
    /// media tool tests; tests never touch the user's XDG profile directory.
    fn voice_session_server(tier: PermissionTier) -> McpServer {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().to_path_buf();
        std::mem::forget(dir); // keep the tempdir alive for the process lifetime
        let proj_paths = ProjectPaths::new(&path);
        proj_paths.ensure_layout().expect("layout");
        let project = Project::new("Voice Test").expect("project");
        let sandbox = WorkspaceSandbox::new([&path]).expect("sandbox");
        let audit = AuditLogger::new(path.join("audit.jsonl")).expect("audit");
        let session = ApiSession::new(project, proj_paths, tier, sandbox, audit, "agent")
            .expect("session")
            .with_voice_store_dir(path.join("voices"));
        McpServer::new(Arc::new(Mutex::new(session)), tier)
    }

    /// First page's first track id, for tests that only need one handle.
    fn first_track_id(server: &McpServer) -> String {
        let tracks = call_tool(server, "track_list", serde_json::json!({}));
        tracks["structuredContent"]["items"][0]["id"]
            .as_str()
            .expect("track id")
            .to_owned()
    }

    fn call_tool(server: &McpServer, name: &str, arguments: Value) -> Value {
        let resp = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".into(),
            params: serde_json::json!({ "name": name, "arguments": arguments }),
        });
        resp.result.expect("tool result payload")
    }

    fn tool_names(server: &McpServer) -> Vec<String> {
        let resp = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "tools/list".into(),
            params: Value::Null,
        });
        resp.result.expect("tools result")["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .filter_map(|t| t["name"].as_str().map(String::from))
            .collect()
    }

    #[test]
    fn tool_visibility_follows_the_tier_contract() {
        let observe_names = tool_names(&voice_session_server(PermissionTier::Observe));
        for observe_tool in [
            "tts_voices",
            "voice_profile_get",
            "transcribe",
            "media_list",
        ] {
            assert!(
                observe_names.iter().any(|n| n == observe_tool),
                "Observe tier must list {observe_tool}"
            );
        }
        for edit_tool in ["tts_generate", "voice_design", "media_import", "clip_add"] {
            assert!(
                !observe_names.iter().any(|n| n == edit_tool),
                "Observe tier must not list {edit_tool}"
            );
        }

        let edit_names = tool_names(&voice_session_server(PermissionTier::Edit));
        for edit_tool in [
            "tts_voices",
            "voice_profile_get",
            "transcribe",
            "media_list",
            "tts_generate",
            "voice_design",
            "media_import",
            "clip_add",
        ] {
            assert!(
                edit_names.iter().any(|n| n == edit_tool),
                "Edit tier must list {edit_tool}"
            );
        }

        // Enforcement lives in alp-api even if a client forces the call.
        let observe = voice_session_server(PermissionTier::Observe);
        for forced in [
            (
                "tts_generate",
                serde_json::json!({ "text": "nope", "voice_id": "aura" }),
            ),
            ("media_import", serde_json::json!({ "path": "some.wav" })),
            (
                "clip_add",
                serde_json::json!({ "track_id": "t", "source_id": "s", "start_secs": 0.0 }),
            ),
        ] {
            let result = call_tool(&observe, forced.0, forced.1);
            assert_eq!(
                result["isError"], true,
                "Observe tier must not run {}",
                forced.0
            );
            assert!(
                result["content"][0]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("permission"),
                "denial must name the tier: {result}"
            );
        }
    }

    #[test]
    fn tts_generate_writes_a_wav_and_names_the_engine() {
        let server = voice_session_server(PermissionTier::Edit);
        let result = call_tool(
            &server,
            "tts_generate",
            serde_json::json!({
                "text": "Hello from the voice toolset",
                "voice_id": "aura",
                "speed": 1.5
            }),
        );
        assert_ne!(result["isError"], true, "tts_generate failed: {result}");
        let report = &result["structuredContent"];
        let path = report["path"].as_str().expect("path reported");
        assert!(path.ends_with(".wav"), "v1 writes WAV: {path}");
        assert!(
            path.contains("media"),
            "output lands in the project media dir"
        );
        assert!(std::path::Path::new(path).is_file(), "the WAV file exists");
        assert!(report["duration_secs"].as_f64().expect("duration") > 0.0);
        let engine = report["engine"].as_str().expect("engine named");
        assert!(
            engine == "builtin-acoustic" || engine.starts_with("daemon:"),
            "engine must be honest: {engine}"
        );

        // Unknown voice ids fail with a plain-language cause.
        let unknown = call_tool(
            &server,
            "tts_generate",
            serde_json::json!({
                "text": "hi", "voice_id": "no-such-voice"
            }),
        );
        assert_eq!(unknown["isError"], true);
        assert!(
            unknown["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("tts_voices"),
            "the failure points at discovery: {unknown}"
        );
    }

    #[test]
    fn transcribe_reports_placeholder_words_and_writes_formats() {
        let server = voice_session_server(PermissionTier::Edit);
        let tts = call_tool(
            &server,
            "tts_generate",
            serde_json::json!({
                "text": "Round trip transcription check", "voice_id": "aura"
            }),
        );
        let wav_path = tts["structuredContent"]["path"]
            .as_str()
            .expect("generated wav")
            .to_owned();

        let result = call_tool(
            &server,
            "transcribe",
            serde_json::json!({ "path": wav_path }),
        );
        assert_ne!(result["isError"], true, "transcribe failed: {result}");
        let report = &result["structuredContent"];
        let engine = report["engine"].as_str().expect("engine named").to_owned();
        if engine == "builtin" {
            assert!(
                report["text"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("Speech segment"),
                "built-in STT must admit placeholder words in the result"
            );
        } else {
            assert!(
                engine.starts_with("daemon:"),
                "engine must be honest: {engine}"
            );
        }

        // Unknown formats are rejected, not silently coerced.
        let bad_format = call_tool(
            &server,
            "transcribe",
            serde_json::json!({ "path": wav_path, "format": "xml" }),
        );
        assert_eq!(
            bad_format["isError"], true,
            "unknown format must fail: {bad_format}"
        );
    }

    #[test]
    fn media_chain_imports_generated_audio_and_places_clips() {
        let server = voice_session_server(PermissionTier::Edit);
        let track = call_tool(
            &server,
            "track_add",
            serde_json::json!({ "name": "Narration" }),
        );
        assert_ne!(track["isError"], true, "track_add failed: {track}");
        let track_id = track["structuredContent"]["track_id"]
            .as_str()
            .expect("track id")
            .to_owned();

        let tts = call_tool(
            &server,
            "tts_generate",
            serde_json::json!({ "text": "Media chain check", "voice_id": "aura" }),
        );
        let wav_path = tts["structuredContent"]["path"]
            .as_str()
            .expect("generated wav")
            .to_owned();

        let imported = call_tool(
            &server,
            "media_import",
            serde_json::json!({ "path": wav_path }),
        );
        assert_ne!(imported["isError"], true, "media_import failed: {imported}");
        let source = &imported["structuredContent"];
        assert!(
            source["relative_path"]
                .as_str()
                .unwrap_or_default()
                .starts_with("media/"),
            "imports register a media/ relative path: {source}"
        );
        assert!(source["duration_secs"].as_f64().expect("duration") > 0.0);
        let source_id = source["source_id"].as_str().expect("source id").to_owned();

        let placed = call_tool(
            &server,
            "clip_add",
            serde_json::json!({ "track_id": track_id, "source_id": source_id, "start_secs": 0.0 }),
        );
        assert_ne!(placed["isError"], true, "clip_add failed: {placed}");
        assert!(
            placed["structuredContent"]["clip_id"].as_str().is_some(),
            "clip id returned: {placed}"
        );

        let info = call_tool(&server, "project_info", serde_json::json!({}));
        assert_eq!(
            info["structuredContent"]["clip_count"], 1,
            "the placed clip is counted: {info}"
        );

        // Unknown source ids fail with a pointer at media_list.
        let unknown = call_tool(
            &server,
            "clip_add",
            serde_json::json!({ "track_id": track_id, "source_id": "src_missing", "start_secs": 0.0 }),
        );
        assert_eq!(unknown["isError"], true);
        assert!(
            unknown["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("media_list"),
            "the failure points at discovery: {unknown}"
        );

        // Non-audio (or absent) files fail with the decode error, not a
        // silent import: the relative path resolves inside the sandbox and
        // then fails to decode.
        let bad = call_tool(
            &server,
            "media_import",
            serde_json::json!({ "path": "not-audio.txt" }),
        );
        assert_eq!(bad["isError"], true, "non-audio import must fail: {bad}");
    }

    #[test]
    fn voice_design_lifecycle_stays_designed_only() {
        let server = voice_session_server(PermissionTier::Edit);
        let result = call_tool(
            &server,
            "voice_design",
            serde_json::json!({
                "name": "Calm Guide",
                "prompt": "A calm, deep documentary narrator",
                "tags": ["narration"]
            }),
        );
        assert_ne!(result["isError"], true, "voice_design failed: {result}");
        let profile = &result["structuredContent"];
        assert_eq!(profile["kind"], "designed", "MCP can only design voices");
        assert_eq!(
            profile["consent_recorded"], false,
            "consent stays human-only"
        );
        let designed_id = profile["id"].as_str().expect("id").to_owned();

        let voices = call_tool(&server, "tts_voices", serde_json::json!({}));
        let list = &voices["structuredContent"]["voices"];
        assert!(
            list.as_array()
                .expect("voices list")
                .iter()
                .all(|v| v["kind"] != "cloned"),
            "no tool in this slice may produce a cloned profile: {list}"
        );
        assert!(
            list.as_array()
                .expect("voices list")
                .iter()
                .any(|v| v["id"] == designed_id),
            "the designed voice appears in tts_voices"
        );

        let inspect = call_tool(
            &server,
            "voice_profile_get",
            serde_json::json!({ "voice_id": designed_id }),
        );
        assert_ne!(
            inspect["isError"], true,
            "voice_profile_get failed: {inspect}"
        );
    }

    #[test]
    fn active_project_and_voice_resources_are_served() {
        let server = voice_session_server(PermissionTier::Observe);

        let read = |uri: &str| {
            server.handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(1)),
                method: "resources/read".into(),
                params: serde_json::json!({ "uri": uri }),
            })
        };

        let project = read("alp://project/active")
            .result
            .expect("project resource");
        let text = project["contents"][0]["text"].as_str().expect("json text");
        let summary: Value = serde_json::from_str(text).expect("parses as project summary");
        assert_eq!(summary["name"], "Voice Test");
        assert!(summary["sample_rate"].as_u64().is_some());

        let voices = read("alp://voices").result.expect("voices resource");
        let voices_text = voices["contents"][0]["text"].as_str().expect("json text");
        let report: Value = serde_json::from_str(voices_text).expect("parses as voice report");
        assert!(
            report["voices"].as_array().expect("voices").len() >= 4,
            "seed voices listed"
        );

        let unknown = read("alp://nope").error.expect("unknown resource errors");
        assert_eq!(unknown.code, -32602);
    }

    /// Write a minimal 16-bit PCM WAV (no external crate needed) and return a
    /// server holding a project with that file imported and placed on one
    /// track. `pattern` is `[(kind, secs)]` where `kind` is `"tone"` or
    /// `"silence"`, so the fixture is exactly the sum of its parts and the
    /// leading/trailing runs of silence are real.
    fn audio_session_server(pattern: &[(&str, f64)]) -> McpServer {
        const RATE: u32 = 48_000;
        let dir = tempdir().expect("tempdir");
        let path = dir.path().to_path_buf();
        std::mem::forget(dir);
        let proj_paths = ProjectPaths::new(&path);
        proj_paths.ensure_layout().expect("layout");

        let mut samples: Vec<i16> = Vec::new();
        for (kind, secs) in pattern {
            let count = (*secs * f64::from(RATE)) as usize;
            if *kind == "tone" {
                samples.extend((0..count).map(|i| {
                    (0.5 * (i as f32 * 220.0 * 2.0 * std::f32::consts::PI / RATE as f32).sin()
                        * 32767.0) as i16
                }));
            } else {
                samples.extend(std::iter::repeat_n(0i16, count));
            }
        }

        let bytes = write_int16_wav(RATE, 1, &samples);
        let fixture = path.join("fixture.wav");
        std::fs::write(&fixture, bytes).expect("write fixture");

        let project = Project::new("Audio Test").expect("project");
        let sandbox = WorkspaceSandbox::new([&path]).expect("sandbox");
        let audit = AuditLogger::new(path.join("audit.jsonl")).expect("audit");
        let session = ApiSession::new(
            project,
            proj_paths,
            PermissionTier::Edit,
            sandbox,
            audit,
            "agent",
        )
        .expect("session")
        .with_voice_store_dir(path.join("voices"));
        let server = McpServer::new(Arc::new(Mutex::new(session)), PermissionTier::Edit);

        let track = call_tool(&server, "track_add", serde_json::json!({ "name": "Voice" }));
        assert_ne!(track["isError"], true, "track_add failed: {track}");
        let track_id = track["structuredContent"]["track_id"]
            .as_str()
            .expect("track id")
            .to_owned();
        let imported = call_tool(
            &server,
            "media_import",
            serde_json::json!({ "path": fixture.display().to_string() }),
        );
        assert_ne!(imported["isError"], true, "media_import failed: {imported}");
        let source_id = imported["structuredContent"]["source_id"]
            .as_str()
            .expect("source id")
            .to_owned();
        let placed = call_tool(
            &server,
            "clip_add",
            serde_json::json!({ "track_id": track_id, "source_id": source_id, "start_secs": 0.0 }),
        );
        assert_ne!(placed["isError"], true, "clip_add failed: {placed}");
        server
    }

    /// Encode a 16-bit PCM WAV by hand so the test fixture does not need a
    /// media crate behind it. RIFF/WAVE, one fmt chunk, one data chunk.
    fn write_int16_wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data_len = samples.len() * 2;
        let mut out = Vec::with_capacity(44 + data_len);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
        out.extend_from_slice(&(channels * 2).to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data_len as u32).to_le_bytes());
        for sample in samples {
            out.extend_from_slice(&sample.to_le_bytes());
        }
        out
    }

    fn project_duration(server: &McpServer) -> f64 {
        call_tool(server, "project_info", serde_json::json!({}))["structuredContent"]
            ["duration_secs"]
            .as_f64()
            .expect("duration")
    }

    #[test]
    fn edit_batch_applies_real_operations_and_collapses_into_one_undo() {
        let server = audio_session_server(&[("tone", 1.0)]);
        let track_id = first_track_id(&server);
        let result = call_tool(
            &server,
            "edit_batch",
            serde_json::json!({
                "label": "setup",
                "operations": [
                    { "op": "track_update", "track_id": track_id, "name": "Narration", "gain": 0.5 },
                    { "op": "marker_add", "name": "Chapter 1", "position_secs": 0.5 }
                ]
            }),
        );
        assert_ne!(result["isError"], true, "edit_batch failed: {result}");
        let report = &result["structuredContent"];
        assert_eq!(report["operations_applied"], 2, "{report}");
        assert_eq!(report["single_undo_step"], true, "{report}");

        let after = call_tool(&server, "track_list", serde_json::json!({}));
        assert_eq!(
            after["structuredContent"]["items"][0]["name"], "Narration",
            "the track rename must have really happened: {after}"
        );

        // One undo must revert both operations; two separate commands would
        // leave the rename behind.
        call_tool(&server, "history_undo", serde_json::json!({}));
        let restored = call_tool(&server, "track_list", serde_json::json!({}));
        assert_eq!(
            restored["structuredContent"]["items"][0]["name"], "Voice",
            "one undo must revert the whole batch: {restored}"
        );
    }

    #[test]
    fn a_failing_edit_batch_reports_no_changes() {
        let server = audio_session_server(&[("tone", 1.0)]);
        let track_id = first_track_id(&server);

        let result = call_tool(
            &server,
            "edit_batch",
            serde_json::json!({
                "label": "broken",
                "operations": [
                    { "op": "track_update", "track_id": track_id, "name": "Half applied" },
                    { "op": "clip_remove", "clip_id": "not-a-clip" }
                ]
            }),
        );
        assert_eq!(result["isError"], true, "the batch must fail: {result}");
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("no changes were applied"),
            "the failure must state that nothing stuck: {result}"
        );

        let after = call_tool(&server, "track_list", serde_json::json!({}));
        assert_eq!(
            after["structuredContent"]["items"][0]["name"], "Voice",
            "the first operation must have been rolled back: {after}"
        );
    }

    #[test]
    fn render_waveform_returns_the_art_it_computed() {
        let server = audio_session_server(&[("silence", 0.5), ("tone", 1.0), ("silence", 0.5)]);
        let result = call_tool(
            &server,
            "render_waveform",
            serde_json::json!({ "width": 40, "height": 10 }),
        );
        assert_ne!(result["isError"], true, "render_waveform failed: {result}");

        let ascii = result["structuredContent"]["ascii"]
            .as_str()
            .expect("ascii art");
        assert!(!ascii.is_empty(), "the render must return art");
        assert!(
            ascii.contains('#'),
            "a 1 s tone must show filled cells, got:\n{ascii}"
        );
        let lines: Vec<&str> = ascii.lines().collect();
        assert_eq!(lines.len(), 10, "one line per requested row");
        assert_eq!(lines[0].chars().count(), 40, "one cell per column");
        // The tone runs from 0.5 s to 1.5 s of a 2 s file, so columns 10..30
        // must be filled on the lowest threshold rows and empty elsewhere.
        let lowest_threshold = *lines.last().expect("rows");
        let middle: String = lowest_threshold.chars().skip(12).take(16).collect();
        assert!(
            middle.chars().all(|c| c == '#'),
            "the tone must appear where it actually is in time:\n{ascii}"
        );
        let leading: String = lowest_threshold.chars().take(8).collect();
        assert!(
            leading.chars().all(|c| c == ' '),
            "the leading silence must stay empty:\n{ascii}"
        );
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains('#'),
            "the text content must carry the art too, not a locator"
        );
    }

    #[test]
    fn render_spectrogram_returns_a_real_time_frequency_map() {
        let server = audio_session_server(&[("tone", 1.0)]);
        let result = call_tool(
            &server,
            "render_spectrogram",
            serde_json::json!({ "width": 32, "height": 8 }),
        );
        assert_ne!(
            result["isError"], true,
            "render_spectrogram failed: {result}"
        );

        let ascii = result["structuredContent"]["ascii"]
            .as_str()
            .expect("ascii art");
        let lines: Vec<&str> = ascii.lines().collect();
        assert_eq!(lines.len(), 8, "one line per band");
        assert_eq!(lines[0].chars().count(), 32, "one cell per column");

        let centers = result["structuredContent"]["band_centers_hz"]
            .as_array()
            .expect("band centers");
        assert_eq!(centers.len(), 8);
        assert!(
            centers
                .windows(2)
                .all(|w| { w[0].as_f64().unwrap() < w[1].as_f64().unwrap() }),
            "band centers must ascend: {centers:?}"
        );

        // A 220 Hz tone must light up the band nearest 220 Hz, not the
        // highest band in the map.
        // Character level ranks: the renderer maps louder cells to denser
        // glyphs, so the peak glyph ranks higher in the tone's band.
        let levels = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
        let energy = |line: &str| {
            line.chars()
                .filter_map(|c| levels.iter().position(|level| *level == c))
                .max()
                .unwrap_or(0)
        };
        let nearest = centers
            .iter()
            .enumerate()
            .min_by(|a, b| {
                let da = (a.1.as_f64().unwrap() - 220.0).abs();
                let db = (b.1.as_f64().unwrap() - 220.0).abs();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
            .expect("a nearest band");
        // Rows are printed high frequency first, so the band's row is mirrored.
        let tone_rank = energy(lines[centers.len() - 1 - nearest]);
        let top_rank = energy(lines[0]);
        assert!(
            tone_rank > top_rank,
            "the 220 Hz band must be denser than the highest band (tone rank {tone_rank}, top rank {top_rank}):\n{ascii}"
        );
    }

    #[test]
    fn trim_silence_through_mcp_actually_shortens_the_timeline() {
        let server = audio_session_server(&[("silence", 0.5), ("tone", 1.0), ("silence", 0.5)]);
        let before = project_duration(&server);
        assert!((before - 2.0).abs() < 0.05, "fixture is 2 s, got {before}");

        let track_id = first_track_id(&server);

        let result = call_tool(
            &server,
            "edit_trim_silence",
            serde_json::json!({
                "track_id": track_id,
                "threshold_db": -45.0,
                "min_silence_secs": 0.2
            }),
        );
        assert_ne!(result["isError"], true, "trim failed: {result}");
        let report = &result["structuredContent"];
        assert_eq!(report["ranges_cut"], 2, "{report}");

        let after = project_duration(&server);
        assert!(
            after < before - 0.8,
            "the timeline must really shrink (was {before}s, now {after}s)"
        );
    }

    #[test]
    fn trim_silence_rejects_an_impossible_threshold() {
        let server = audio_session_server(&[("tone", 1.0)]);
        let result = call_tool(
            &server,
            "edit_trim_silence",
            serde_json::json!({
                "track_id": "whatever",
                "threshold_db": 12.0,
                "min_silence_secs": 0.1
            }),
        );
        assert_eq!(result["isError"], true, "positive dBFS is not a threshold");
    }

    #[test]
    fn every_advertised_tool_is_dispatched() {
        let server = audio_session_server(&[("tone", 0.2)]);
        let names = tool_names(&server);
        assert!(
            names.len() >= 55,
            "the surface should be the full Phase 10 set, got {}",
            names.len()
        );
        for name in names {
            let result = call_tool(&server, &name, serde_json::json!({}));
            let text = result["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            assert!(
                !text.contains("not recognized"),
                "{name} is advertised but not dispatched"
            );
        }
    }

    #[test]
    fn both_prompts_are_served() {
        let server = voice_session_server(PermissionTier::Observe);
        for prompt in ["clean_up_voice_recording", "produce_podcast_episode"] {
            let resp = server.handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(1)),
                method: "prompts/get".into(),
                params: serde_json::json!({ "name": prompt }),
            });
            let result = resp.result.expect("prompt result");
            let text = result["messages"][0]["content"]["text"]
                .as_str()
                .unwrap_or_default();
            assert!(
                text.len() > 200 && text.contains("Call '"),
                "{prompt} must return a real recipe, got: {text}"
            );
        }
    }

    #[test]
    fn the_agent_guide_resource_serves_the_file() {
        let server = voice_session_server(PermissionTier::Observe);
        let resp = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "resources/read".into(),
            params: serde_json::json!({ "uri": "alp://docs/agent-guide" }),
        });
        let text = resp.result.expect("resource")["contents"][0]["text"]
            .as_str()
            .expect("text")
            .to_owned();
        let on_disk = include_str!("../../../docs/mcp/AGENT_GUIDE.md");
        assert_eq!(
            text, on_disk,
            "the resource must serve the real guide, not a copy"
        );
    }

    #[test]
    fn every_advertised_tool_declares_a_title_and_annotations() {
        let server = voice_session_server(PermissionTier::Export);
        let resp = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "tools/list".into(),
            params: Value::Null,
        });
        let tools = resp.result.expect("result")["tools"]
            .as_array()
            .expect("tools")
            .clone();
        assert_eq!(tools.len(), 87, "the advertised surface is 87 tools");
        for tool in &tools {
            let name = tool["name"].as_str().expect("name");
            let title = tool["title"].as_str().unwrap_or_default();
            assert!(!title.is_empty(), "{name} must carry a title");
            assert_ne!(
                title,
                format!("Tool {name}"),
                "{name} is missing from the metadata table"
            );
            let annotations = tool["annotations"]
                .as_object()
                .unwrap_or_else(|| panic!("{name} must carry annotations"));
            for key in [
                "readOnlyHint",
                "destructiveHint",
                "idempotentHint",
                "openWorldHint",
            ] {
                assert!(
                    annotations[key].is_boolean(),
                    "{name} is missing boolean annotation {key}"
                );
            }
        }

        // Spot-check that the hints match reality rather than being all-false.
        let annotation = |name: &str, key: &str| {
            tools
                .iter()
                .find(|t| t["name"] == name)
                .unwrap_or_else(|| panic!("{name} advertised"))["annotations"][key]
                .as_bool()
                .expect("annotation")
        };
        assert!(annotation("project_info", "readOnlyHint"));
        assert!(annotation("project_info", "idempotentHint"));
        assert!(!annotation("clip_remove", "readOnlyHint"));
        assert!(annotation("clip_remove", "destructiveHint"));
        assert!(!annotation("clip_add", "destructiveHint"));
        assert!(annotation("plugin_list", "openWorldHint"));
    }

    #[test]
    fn tools_list_carries_cache_hints_and_discover_documents_the_limits() {
        let server = voice_session_server(PermissionTier::Edit);
        let list = server
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(1)),
                method: "tools/list".into(),
                params: Value::Null,
            })
            .result
            .expect("tools list");
        assert!(list["ttlMs"].as_u64().is_some(), "{list}");
        assert_eq!(list["cacheScope"], "session");

        let discover = server
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(2)),
                method: "server/discover".into(),
                params: Value::Null,
            })
            .result
            .expect("discover");
        assert_eq!(discover["pagination"]["maxLimit"], 200);
        assert!(discover["limits"]["maxPayloadBytes"].as_u64().is_some());
        assert!(discover["limits"]["remainingInWindow"].as_u64().is_some());
    }

    #[test]
    fn list_tools_paginate_and_report_total() {
        let server = audio_session_server(&[("tone", 0.2)]);
        // The fixture has one track, so a page of one is complete.
        let full = call_tool(&server, "track_list", serde_json::json!({}));
        assert_eq!(full["structuredContent"]["total"], 1);
        assert_eq!(
            full["structuredContent"]["items"].as_array().unwrap().len(),
            1
        );
        assert!(
            full["structuredContent"]["next_cursor"].is_null(),
            "a complete page must not offer a cursor: {full}"
        );

        // Ask for zero items: rejected with a reason, not silently defaulted.
        let zero = call_tool(&server, "track_list", serde_json::json!({ "limit": 0 }));
        assert_eq!(zero["isError"], true, "{zero}");
        assert!(
            zero["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("at least 1"),
            "{zero}"
        );

        // A malformed cursor is an error rather than a silent restart at 0.
        let bad = call_tool(
            &server,
            "track_list",
            serde_json::json!({ "cursor": "nonsense" }),
        );
        assert_eq!(bad["isError"], true, "{bad}");
        assert!(
            bad["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("next_cursor"),
            "the cursor error must say how to recover: {bad}"
        );

        // An offset past the end is an empty page that still reports the total.
        let past_end = call_tool(
            &server,
            "track_list",
            serde_json::json!({ "cursor": alp_api::encode_cursor(99) }),
        );
        assert_ne!(past_end["isError"], true, "{past_end}");
        assert_eq!(past_end["structuredContent"]["total"], 1);
        assert!(
            past_end["structuredContent"]["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn history_list_reports_the_real_command_labels() {
        let server = audio_session_server(&[("tone", 0.2)]);
        let track_id = first_track_id(&server);
        call_tool(
            &server,
            "track_update",
            serde_json::json!({ "track_id": track_id, "name": "Narration" }),
        );

        let history = call_tool(&server, "history_list", serde_json::json!({}));
        let first = &history["structuredContent"]["items"][0];
        assert_eq!(
            first["description"], "Rename track",
            "history must name the command that really ran, not 'Undoable edit step #1': {history}"
        );
        assert_eq!(first["is_undo"], true);

        // `undo_steps` is now a real count, so it must agree with the list.
        let info = call_tool(&server, "project_info", serde_json::json!({}));
        assert_eq!(
            info["structuredContent"]["undo_steps"], history["structuredContent"]["total"],
            "undo_steps must be a count that matches history_list: {info}"
        );
        assert!(
            info["structuredContent"]["undo_steps"]
                .as_u64()
                .unwrap_or(0)
                > 1,
            "the fixture built a track, a source and a clip, so this is more than one step: {info}"
        );
    }

    #[test]
    fn expected_revision_guards_writes_without_touching_state() {
        let server = audio_session_server(&[("tone", 0.2)]);
        let info = call_tool(&server, "project_info", serde_json::json!({}));
        let revision = info["structuredContent"]["revision"]
            .as_u64()
            .expect("revision");

        // A stale revision must be refused before anything changes.
        let stale = call_tool(
            &server,
            "track_add",
            serde_json::json!({ "name": "Stale", "expected_revision": revision + 100 }),
        );
        assert_eq!(stale["isError"], true, "{stale}");
        let text = stale["content"][0]["text"].as_str().unwrap_or_default();
        assert!(
            text.contains("concurrency") && text.contains("project_info"),
            "the conflict must name the cause and the recovery call: {stale}"
        );

        let tracks = call_tool(&server, "track_list", serde_json::json!({}));
        assert_eq!(
            tracks["structuredContent"]["total"], 1,
            "a refused write must not add a track: {tracks}"
        );

        // The current revision is accepted.
        let fresh = call_tool(
            &server,
            "track_add",
            serde_json::json!({ "name": "Accepted", "expected_revision": revision }),
        );
        assert_ne!(
            fresh["isError"], true,
            "a current revision must be accepted: {fresh}"
        );
        let tracks = call_tool(&server, "track_list", serde_json::json!({}));
        assert_eq!(tracks["structuredContent"]["total"], 2);
    }

    #[test]
    fn reads_ignore_expected_revision_and_write_tools_advertise_it() {
        // Export tier so every write tool, including the file writers, is listed.
        let server = voice_session_server(PermissionTier::Export);
        // A read cannot invalidate a client's view, so the guard must not apply.
        let info = call_tool(
            &server,
            "project_info",
            serde_json::json!({ "expected_revision": 9999 }),
        );
        assert_ne!(info["isError"], true, "reads ignore the guard: {info}");

        let tools = server
            .handle_request(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(1)),
                method: "tools/list".into(),
                params: Value::Null,
            })
            .result
            .expect("tools")["tools"]
            .as_array()
            .expect("tools")
            .clone();

        for name in ["track_add", "clip_remove", "edit_batch", "export_audio"] {
            let tool = tools
                .iter()
                .find(|t| t["name"] == name)
                .unwrap_or_else(|| panic!("{name} advertised"));
            assert!(
                tool["inputSchema"]["properties"]["expected_revision"].is_object(),
                "{name} is a write and must advertise expected_revision: {tool}"
            );
        }
        let read_tool = tools
            .iter()
            .find(|t| t["name"] == "project_info")
            .expect("project_info");
        assert!(
            read_tool["inputSchema"]["properties"]["expected_revision"].is_null(),
            "a read must not advertise a concurrency guard"
        );

        for name in [
            "track_list",
            "media_list",
            "history_list",
            "plugin_list",
            "project_list",
        ] {
            let tool = tools
                .iter()
                .find(|t| t["name"] == name)
                .unwrap_or_else(|| panic!("{name} advertised"));
            assert!(
                tool["inputSchema"]["properties"]["limit"].is_object(),
                "{name} lists items and must advertise limit"
            );
            assert!(
                tool["inputSchema"]["properties"]["cursor"].is_object(),
                "{name} lists items and must advertise cursor"
            );
        }
    }

    #[test]
    fn the_rate_limit_refuses_the_call_it_cannot_serve() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().to_path_buf();
        std::mem::forget(dir);
        let proj_paths = ProjectPaths::new(&path);
        proj_paths.ensure_layout().expect("layout");
        let project = Project::new("Rate Test").expect("project");
        let sandbox = WorkspaceSandbox::new([&path]).expect("sandbox");
        let audit = AuditLogger::new(path.join("audit.jsonl")).expect("audit");
        let session = ApiSession::new(
            project,
            proj_paths,
            PermissionTier::Observe,
            sandbox,
            audit,
            "agent",
        )
        .expect("session");
        let server = McpServer::new(Arc::new(Mutex::new(session)), PermissionTier::Observe)
            .with_limits(limits::Limits {
                requests_per_window: 2,
                window: std::time::Duration::from_secs(60),
                ..limits::Limits::default()
            });

        assert_ne!(
            call_tool(&server, "project_info", serde_json::json!({}))["isError"],
            true
        );
        assert_ne!(
            call_tool(&server, "track_list", serde_json::json!({}))["isError"],
            true
        );
        let refused = call_tool(&server, "project_info", serde_json::json!({}));
        assert_eq!(refused["isError"], true, "{refused}");
        let text = refused["content"][0]["text"].as_str().unwrap_or_default();
        assert!(
            text.contains("rate limit") && text.contains("wait"),
            "the refusal must name the limit and the recovery: {refused}"
        );
    }
}
