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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let tier = match args.tier.to_lowercase().as_str() {
        "observe" => PermissionTier::Observe,
        "edit" => PermissionTier::Edit,
        _ => PermissionTier::Export,
    };

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
}
