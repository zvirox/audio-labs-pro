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

mod protocol;
mod server;

use protocol::{JsonRpcRequest, JsonRpcResponse};
use server::McpServer;

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

    let workspace_path = args.workspace.canonicalize().unwrap_or(args.workspace);
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

    let server = McpServer::new(Arc::new(Mutex::new(session)), tier);

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
}
