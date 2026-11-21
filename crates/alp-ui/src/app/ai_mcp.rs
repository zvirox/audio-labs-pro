//! Full MCP calls prepared on worker threads and committed by `ai_control`.
//!
//! An `ApiSession` here is deliberately ephemeral scratch execution for one
//! call. The GUI snapshot supplies its starting state, and only an unchanged
//! revision can merge the resulting project into the authoritative
//! `EditSession`. No worker result is rendered directly or quietly overwrites
//! concurrent operator work.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use alp_api::control::{ControlLimits, ControlResponse};
use alp_api::{
    ApiSession, AttachedProjectState, AuditLogger, PermissionTier, ProjectSummary, WorkspaceSandbox,
};
use alp_core::SourceId;
use alp_engine::SourceAudio;
use alp_mcp::limits::Limits;
use alp_mcp::{JsonRpcRequest, JsonRpcResponse, McpServer, ToolCallResult, live};
use serde_json::Value;

#[derive(Default)]
pub(super) struct AttachedMcpState {
    sender: Option<Sender<AttachedCompletion>>,
    receiver: Option<Receiver<AttachedCompletion>>,
    clients: HashMap<String, Sender<AttachedWork>>,
    pub(super) last_commit_ms: Option<u64>,
    pub(super) max_commit_ms: u64,
    pub(super) commits: u64,
}

impl AttachedMcpState {
    fn channels(&mut self) -> (Sender<AttachedCompletion>, &Receiver<AttachedCompletion>) {
        if self.sender.is_none() {
            let (sender, receiver) = mpsc::channel();
            self.sender = Some(sender);
            self.receiver = Some(receiver);
        }
        (
            self.sender.as_ref().expect("initialized").clone(),
            self.receiver.as_ref().expect("initialized"),
        )
    }

    pub(super) fn dispatch(&mut self, call: AttachedCall) -> Result<(), String> {
        let AttachedCall {
            host_id,
            client_id,
            request_value,
            limits,
            state,
            summary,
            tier,
            response,
        } = call;
        let limits = clamp_limits(limits);
        let payload_bytes = serde_json::to_vec(&request_value)
            .map_err(|error| format!("cannot encode MCP request: {error}"))?
            .len();
        if payload_bytes > limits.max_payload_bytes {
            return Err(format!(
                "MCP request is {payload_bytes} bytes, over the attached {}-byte limit",
                limits.max_payload_bytes
            ));
        }
        let request: JsonRpcRequest = serde_json::from_value(request_value)
            .map_err(|error| format!("invalid MCP JSON-RPC request: {error}"))?;
        let (events, _) = self.channels();
        let completed = events;
        let sender = self.clients.entry(client_id.clone()).or_insert_with(|| {
            let (sender, receiver) = mpsc::channel();
            let worker_id = client_id.clone();
            let spawn = std::thread::Builder::new()
                .name(format!("alp-attached-mcp-{worker_id}"))
                .spawn(move || run_client(worker_id, receiver, completed));
            if let Err(error) = spawn {
                let _ = response.send(ControlResponse::error(
                    "worker_unavailable",
                    format!("could not start attached MCP worker: {error}"),
                ));
            }
            sender
        });
        let work = AttachedWork {
            host_id,
            client_id,
            state,
            summary,
            tier,
            limits,
            request,
            response,
            queued: Instant::now(),
        };
        sender
            .send(work)
            .map_err(|_| "attached MCP worker stopped; restart AI control".to_owned())
    }

    pub(super) fn drain(&mut self) -> Vec<AttachedCompletion> {
        const MAX_COMPLETIONS_PER_FRAME: usize = 16;
        let mut completions = Vec::new();
        for _ in 0..MAX_COMPLETIONS_PER_FRAME {
            let result = match &self.receiver {
                Some(receiver) => receiver.try_recv(),
                None => break,
            };
            match result {
                Ok(completion) => completions.push(completion),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        completions
    }

    pub(super) fn clear_clients(&mut self) {
        self.clients.clear();
    }
}

pub(super) struct AttachedCall {
    pub(super) host_id: u64,
    pub(super) client_id: String,
    pub(super) request_value: Value,
    pub(super) limits: ControlLimits,
    pub(super) state: AttachedProjectState,
    pub(super) summary: ProjectSummary,
    pub(super) tier: PermissionTier,
    pub(super) response: SyncSender<ControlResponse>,
}

struct AttachedWork {
    host_id: u64,
    client_id: String,
    state: AttachedProjectState,
    summary: ProjectSummary,
    tier: PermissionTier,
    limits: ControlLimits,
    request: JsonRpcRequest,
    response: SyncSender<ControlResponse>,
    queued: Instant,
}

pub(super) struct AttachedCompletion {
    pub(super) base_revision: u64,
    pub(super) tier: PermissionTier,
    pub(super) host_id: u64,
    pub(super) client_id: String,
    pub(super) request: JsonRpcRequest,
    pub(super) response: JsonRpcResponse,
    pub(super) transaction: Option<PreparedTransaction>,
    pub(super) original_response: SyncSender<ControlResponse>,
    pub(super) queued: Instant,
}

pub(super) enum PreparedTransaction {
    Edit {
        base_revision: u64,
        state: Box<AttachedProjectState>,
        audio: HashMap<SourceId, Arc<SourceAudio>>,
    },
    Saved {
        base_revision: u64,
    },
}

struct AttachedEngine {
    session: Arc<Mutex<ApiSession>>,
    server: McpServer,
    tier: PermissionTier,
    limits: ControlLimits,
}

impl AttachedEngine {
    fn new(
        client_id: &str,
        state: AttachedProjectState,
        tier: PermissionTier,
        limits: ControlLimits,
    ) -> Result<Self, String> {
        let sandbox = WorkspaceSandbox::new([state.paths.root()]).map_err(|e| e.to_string())?;
        let audit = AuditLogger::new(state.paths.root().join("audit.jsonl"))
            .map_err(|error| error.to_string())?;
        let mut session = ApiSession::new(
            state.project.clone(),
            state.paths.clone(),
            tier,
            sandbox,
            audit,
            client_id,
        )
        .map_err(|error| error.to_string())?;
        session.rebase_attached(state).map_err(|e| e.to_string())?;
        let session = Arc::new(Mutex::new(session));
        let (call_timeout, window) = limits.durations();
        let server = McpServer::new(Arc::clone(&session), tier).with_limits(Limits {
            max_payload_bytes: limits.max_payload_bytes,
            call_timeout,
            requests_per_window: limits.requests_per_window,
            window,
        });
        Ok(Self {
            session,
            server,
            tier,
            limits,
        })
    }

    fn handle(&mut self, work: AttachedWork) -> AttachedCompletion {
        if alp_mcp::live::tool_name(&work.request) == Some("rollback_session") {
            return completion_with_failure(
                work,
                "rollback_session is not available in attached mode because it could discard operator edits made after attachment; use history_undo or an explicitly selected checkpoint",
            );
        }
        if work.tier != self.tier || work.limits != self.limits {
            return completion_with_failure(
                work,
                "permission or limits changed; restart the desktop AI control host",
            );
        }
        let before = work.state.clone();
        {
            let mut session = self
                .session
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Err(error) = session.rebase_attached(before.clone()) {
                return completion_with_failure(work, error.to_string());
            }
        }
        let mut response = self.server.handle_request(work.request.clone());
        live::patch_project_summary(&work.request, &mut response, &work.summary);

        let after = self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .attached_state();
        let operation = live::tool_name(&work.request);
        if matches!(
            operation,
            Some("history_undo" | "history_redo" | "history_list")
        ) {
            return completion(work, response, None);
        }
        if response_is_tool_failure(&response) {
            // A partially mutated scratch call is rolled back simply by never
            // offering it to the UI commit boundary.
            return completion(work, response, None);
        }
        let project_changed = after.project != before.project
            || after.paths.root() != before.paths.root()
            || after.revision != before.revision;
        let save_only = !project_changed && after.dirty != before.dirty;
        if save_only {
            return completion(
                work,
                response,
                Some(PreparedTransaction::Saved {
                    base_revision: before.revision,
                }),
            );
        }
        if !project_changed {
            return completion(work, response, None);
        }

        let audio = match decode_project_audio(&after) {
            Ok(audio) => audio,
            Err(error) => return completion_with_failure(work, error),
        };
        completion(
            work,
            response,
            Some(PreparedTransaction::Edit {
                base_revision: before.revision,
                state: Box::new(after),
                audio,
            }),
        )
    }
}

fn run_client(
    client_id: String,
    receiver: Receiver<AttachedWork>,
    completed: Sender<AttachedCompletion>,
) {
    let mut engine: Option<AttachedEngine> = None;
    while let Ok(work) = receiver.recv() {
        if engine.is_none() {
            match AttachedEngine::new(&client_id, work.state.clone(), work.tier, work.limits) {
                Ok(created) => engine = Some(created),
                Err(error) => {
                    let _ = work.response.send(ControlResponse::error(
                        "session_initialization_failed",
                        error,
                    ));
                    continue;
                }
            }
        }
        let completion = engine.as_mut().expect("initialized above").handle(work);
        if completed.send(completion).is_err() {
            break;
        }
    }
}

fn decode_project_audio(
    state: &AttachedProjectState,
) -> Result<HashMap<SourceId, Arc<SourceAudio>>, String> {
    let mut audio = HashMap::new();
    for source in &state.project.sources {
        let path = state
            .paths
            .resolve_relative(&source.relative_path)
            .map_err(|error| format!("cannot resolve attached source: {error}"))?;
        let buffer = alp_media::decode(&path).map_err(|error| {
            format!("cannot decode attached source {}: {error}", path.display())
        })?;
        audio.insert(source.id, Arc::new(SourceAudio::from_pcm(&buffer)));
    }
    Ok(audio)
}

fn clamp_limits(requested: ControlLimits) -> ControlLimits {
    let maximum = ControlLimits::default();
    ControlLimits {
        max_payload_bytes: requested
            .max_payload_bytes
            .clamp(1024, maximum.max_payload_bytes),
        call_timeout_ms: requested.call_timeout_ms.clamp(1, maximum.call_timeout_ms),
        requests_per_window: requested
            .requests_per_window
            .clamp(1, maximum.requests_per_window),
        rate_window_secs: requested
            .rate_window_secs
            .clamp(1, maximum.rate_window_secs),
    }
}

fn response_is_tool_failure(response: &JsonRpcResponse) -> bool {
    response.result.as_ref().is_some_and(|result| {
        result.get("isError").and_then(serde_json::Value::as_bool) == Some(true)
    })
}

fn completion_with_failure(work: AttachedWork, message: impl Into<String>) -> AttachedCompletion {
    let mut response = JsonRpcResponse::success(
        work.request.id.clone().unwrap_or(Value::Null),
        serde_json::to_value(ToolCallResult::failure(message)).unwrap(),
    );
    live::patch_project_summary(&work.request, &mut response, &work.summary);
    completion(work, response, None)
}

fn completion(
    work: AttachedWork,
    response: JsonRpcResponse,
    transaction: Option<PreparedTransaction>,
) -> AttachedCompletion {
    let base_revision = work.state.revision;
    AttachedCompletion {
        base_revision,
        tier: work.tier,
        host_id: work.host_id,
        client_id: work.client_id,
        request: work.request,
        response,
        transaction,
        original_response: work.response,
        queued: work.queued,
    }
}

pub(super) fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
