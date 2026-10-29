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
}
