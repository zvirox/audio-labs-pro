//! Stdio-to-desktop attach transport. No project, workspace, sandbox, audit
//! file, or model state is initialized in this mode.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use alp_api::control::{
    CONTROL_PROTOCOL_VERSION, ControlLimits, ControlRequest, ControlResponse, default_socket_path,
    read_response, read_response_buffered, write_request,
};
use alp_mcp::{JsonRpcRequest, JsonRpcResponse};
use serde_json::{Value, json};

use crate::Args;

pub(crate) fn run(args: &Args) -> anyhow::Result<()> {
    let socket = match &args.control_socket {
        Some(path) => path.clone(),
        None => {
            let runtime = std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
                anyhow::anyhow!("--attach requires XDG_RUNTIME_DIR or --control-socket")
            })?;
            default_socket_path(Path::new(&runtime))
        }
    };
    let token = std::env::var("ALP_CONTROL_TOKEN").map_err(|_| {
        anyhow::anyhow!(
            "--attach requires the current GUI token in ALP_CONTROL_TOKEN; tokens are deliberately not persisted"
        )
    })?;
    let limits = ControlLimits {
        max_payload_bytes: args.max_payload_bytes,
        call_timeout_ms: args.call_timeout_ms,
        requests_per_window: args.requests_per_window,
        rate_window_secs: args.rate_window_secs,
    };
    let attached = Arc::new(AttachedClient::new(
        socket,
        token,
        args.client_id.clone(),
        limits,
    )?);
    subscribed_output_worker(Arc::clone(&attached));

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.len() > limits.max_payload_bytes {
            let response = JsonRpcResponse::error(
                Value::Null,
                -32600,
                format!(
                    "request payload is {} bytes, over the {}-byte limit",
                    trimmed.len(),
                    limits.max_payload_bytes
                ),
                None,
            );
            attached.write_json(&response)?;
            continue;
        }
        let request = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(request) => request,
            Err(error) => {
                attached.write_json(&JsonRpcResponse::error(
                    Value::Null,
                    -32700,
                    format!("Parse error: {error}"),
                    None,
                ))?;
                continue;
            }
        };
        if attached.handle_subscription(&request)? {
            continue;
        }
        let id = request.id.clone().unwrap_or(Value::Null);
        attached.send(ControlRequest::Mcp {
            message: serde_json::to_value(request)?,
            limits,
        })?;
        match attached.read()? {
            ControlResponse::Mcp { response } => attached.write_raw(&response)?,
            ControlResponse::Error { code, message } => {
                attached.write_json(&JsonRpcResponse::error(
                    id,
                    -32000,
                    message,
                    Some(json!({ "code": code })),
                ))?;
            }
            other => {
                attached.write_json(&JsonRpcResponse::error(
                    id,
                    -32000,
                    format!(
                        "unexpected desktop control response: {}",
                        response_name(&other)
                    ),
                    None,
                ))?;
            }
        }
    }
    attached.closed.store(true, Ordering::SeqCst);
    Ok(())
}

struct AttachedClient {
    socket_path: PathBuf,
    token: String,
    client_id: String,
    stream: Mutex<UnixStream>,
    reader: Mutex<BufReader<UnixStream>>,
    output: Mutex<std::io::Stdout>,
    subscribed: AtomicBool,
    closed: AtomicBool,
}

impl AttachedClient {
    fn new(
        socket_path: PathBuf,
        token: String,
        client_id: String,
        _limits: ControlLimits,
    ) -> anyhow::Result<Self> {
        let (stream, reader) = connect(&socket_path, &token, &client_id)?;
        Ok(Self {
            socket_path,
            token,
            client_id,
            stream: Mutex::new(stream),
            reader: Mutex::new(reader),
            output: Mutex::new(std::io::stdout()),
            subscribed: AtomicBool::new(false),
            closed: AtomicBool::new(false),
        })
    }

    fn send(&self, request: ControlRequest) -> anyhow::Result<()> {
        let mut stream = self
            .stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        write_request(&mut stream, &request).map_err(Into::into)
    }

    fn read(&self) -> anyhow::Result<ControlResponse> {
        let mut reader = self
            .reader
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        read_response(&mut reader)?.ok_or_else(|| anyhow::anyhow!("desktop control socket closed"))
    }

    fn write_json(&self, value: &impl serde::Serialize) -> anyhow::Result<()> {
        self.write_raw(&serde_json::to_value(value)?)
    }

    fn write_raw(&self, value: &Value) -> anyhow::Result<()> {
        let bytes = serde_json::to_vec(value)?;
        let bounded;
        let bytes = if bytes.len() > alp_api::control::MAX_CONTROL_MESSAGE_BYTES {
            bounded = serde_json::to_vec(&JsonRpcResponse::error(
                value.get("id").cloned().unwrap_or(Value::Null),
                -32000,
                format!(
                    "response is {} bytes, over the {}-byte attach limit; use pagination or narrower arguments",
                    bytes.len(),
                    alp_api::control::MAX_CONTROL_MESSAGE_BYTES
                ),
                Some(json!({ "code": "response_too_large" })),
            ))?;
            &bounded
        } else {
            &bytes
        };
        let mut stdout = self
            .output
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        stdout.write_all(bytes)?;
        writeln!(&mut *stdout)?;
        stdout.flush()?;
        Ok(())
    }

    fn handle_subscription(&self, request: &JsonRpcRequest) -> anyhow::Result<bool> {
        let id = request.id.clone().unwrap_or(Value::Null);
        match request.method.as_str() {
            "resources/subscribe" => {
                let uri = request
                    .params
                    .get("uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if uri != "alp://project/active" {
                    self.write_json(&JsonRpcResponse::error(
                        id,
                        -32602,
                        format!("resource '{uri}' cannot be subscribed"),
                        None,
                    ))?;
                } else {
                    self.subscribed.store(true, Ordering::SeqCst);
                    self.write_json(&JsonRpcResponse::success(id, json!({})))?;
                }
                Ok(true)
            }
            "resources/unsubscribe" => {
                self.subscribed.store(false, Ordering::SeqCst);
                self.write_json(&JsonRpcResponse::success(id, json!({})))?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

fn subscribed_output_worker(client: Arc<AttachedClient>) {
    std::thread::Builder::new()
        .name("alp-mcp-attach-events".into())
        .spawn(move || {
            let result = run_subscription(client);
            if let Err(error) = result {
                tracing::debug!("attached change feed closed: {error}");
            }
        })
        .map(|_| ())
        .unwrap_or(());
}

fn run_subscription(client: Arc<AttachedClient>) -> anyhow::Result<()> {
    let (mut stream, mut reader) = connect(&client.socket_path, &client.token, &client.client_id)?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    write_request(&mut stream, &ControlRequest::Subscribe)?;
    if !matches!(
        read_response(&mut reader)?,
        Some(ControlResponse::Subscribed)
    ) {
        anyhow::bail!("desktop refused project change subscription");
    }
    let mut frame = Vec::new();
    while !client.closed.load(Ordering::SeqCst) {
        match read_response_buffered(&mut reader, &mut frame) {
            Ok(Some(ControlResponse::ProjectChanged { change })) => {
                if !client.subscribed.load(Ordering::SeqCst) {
                    continue;
                }
                client.write_json(&json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/resources/updated",
                    "params": { "uri": "alp://project/active" }
                }))?;
                client.write_json(&json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/alp/project_changed",
                    "params": change
                }))?;
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn connect(
    socket_path: &Path,
    token: &str,
    client_id: &str,
) -> anyhow::Result<(UnixStream, BufReader<UnixStream>)> {
    let mut stream = UnixStream::connect(socket_path).map_err(|error| {
        anyhow::anyhow!(
            "cannot attach to {}: {error}; enable AI control in the running desktop",
            socket_path.display()
        )
    })?;
    let mut reader = BufReader::new(stream.try_clone()?);
    write_request(
        &mut stream,
        &ControlRequest::Authenticate {
            protocol_version: CONTROL_PROTOCOL_VERSION,
            token: token.to_owned(),
            client_id: client_id.to_owned(),
        },
    )?;
    loop {
        match read_response(&mut reader)? {
            Some(ControlResponse::ApprovalRequired { .. }) => {}
            Some(ControlResponse::Authenticated { .. }) => return Ok((stream, reader)),
            Some(ControlResponse::Error { message, .. }) => anyhow::bail!(message),
            Some(other) => anyhow::bail!(
                "unexpected authentication response: {}",
                response_name(&other)
            ),
            None => anyhow::bail!("desktop closed the attach connection during authentication"),
        }
    }
}

fn response_name(response: &ControlResponse) -> &'static str {
    match response {
        ControlResponse::ApprovalRequired { .. } => "approval_required",
        ControlResponse::Authenticated { .. } => "authenticated",
        ControlResponse::ProjectInfo { .. } => "project_info",
        ControlResponse::TrackList { .. } => "track_list",
        ControlResponse::TrackAdded { .. } => "track_added",
        ControlResponse::ClipSplit { .. } => "clip_split",
        ControlResponse::Mcp { .. } => "mcp",
        ControlResponse::Subscribed => "subscribed",
        ControlResponse::ProjectChanged { .. } => "project_changed",
        ControlResponse::Error { .. } => "error",
    }
}
