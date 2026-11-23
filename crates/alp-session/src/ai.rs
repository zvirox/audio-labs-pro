//! Headless AI control host and attached Model Context Protocol (MCP) server.
//!
//! Owns the Unix domain control socket listener, permission tiers, client approval,
//! background MCP worker threads, scratch snapshot isolation, and live project commit.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use alp_api::control::{
    ApprovalDecision, ControlEvent, ControlHost, ControlLimits, ControlRequest, ControlResponse,
    MAX_CONTROL_MESSAGE_BYTES, ProjectChange,
};
use alp_api::{
    ApiSession, AttachedProjectState, AuditLogger, PermissionTier, ProjectSummary, TrackSummary,
    WorkspaceSandbox,
};
use alp_core::SourceId;
use alp_engine::SourceAudio;
use alp_mcp::limits::Limits;
use alp_mcp::{JsonRpcRequest, JsonRpcResponse, McpServer, ToolCallResult, live};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::EditSession;

/// Status descriptor for the AI control socket and attached client bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiControlStatus {
    pub enabled: bool,
    pub socket_path: Option<String>,
    pub token: Option<String>,
    pub tier: PermissionTier,
    pub trusted_clients: Vec<String>,
    pub pending_clients: Vec<String>,
    pub commits: u64,
    pub last_commit_ms: Option<u64>,
    pub max_commit_ms: u64,
    pub last_status: String,
}

/// An audit entry representing one autonomous or operator action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiActivityEntry {
    pub timestamp: String,
    pub client_id: String,
    pub operation: String,
    pub outcome: String,
    pub queue_to_commit_ms: Option<u64>,
    pub committed: bool,
}

/// Events emitted during an AI control tick.
#[derive(Debug, Clone)]
pub enum AiEvent {
    RevisionChanged(u64),
    ApprovalRequired { client_id: String },
    Activity(AiActivityEntry),
}

struct PendingApproval {
    client_id: String,
    response: SyncSender<ApprovalDecision>,
}

#[derive(Default)]
struct AttachedMcpState {
    sender: Option<Sender<AttachedCompletion>>,
    receiver: Option<Receiver<AttachedCompletion>>,
    clients: HashMap<String, Sender<AttachedWork>>,
    last_commit_ms: Option<u64>,
    max_commit_ms: u64,
    commits: u64,
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

    fn dispatch(&mut self, call: AttachedCall) -> Result<(), String> {
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

    fn drain(&mut self) -> Vec<AttachedCompletion> {
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

    fn clear_clients(&mut self) {
        self.clients.clear();
    }
}

struct AttachedCall {
    host_id: u64,
    client_id: String,
    request_value: Value,
    limits: ControlLimits,
    state: AttachedProjectState,
    summary: ProjectSummary,
    tier: PermissionTier,
    response: SyncSender<ControlResponse>,
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

struct AttachedCompletion {
    base_revision: u64,
    tier: PermissionTier,
    host_id: u64,
    client_id: String,
    request: JsonRpcRequest,
    response: JsonRpcResponse,
    transaction: Option<PreparedTransaction>,
    original_response: SyncSender<ControlResponse>,
    queued: Instant,
}

enum PreparedTransaction {
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
        if live::tool_name(&work.request) == Some("rollback_session") {
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

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn current_timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    alp_api::format_timestamp(secs)
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn summary_key(summary: &ProjectSummary) -> String {
    serde_json::to_string(summary)
        .unwrap_or_else(|_| format!("{}:{}", summary.id, summary.revision))
}

fn add_attach_meta(
    response: &mut JsonRpcResponse,
    snapshot_revision: u64,
    live_revision: u64,
    queue_to_commit_ms: u64,
    committed: bool,
) {
    if let Some(result) = response
        .result
        .as_mut()
        .and_then(serde_json::Value::as_object_mut)
    {
        result.insert(
            "_meta".into(),
            serde_json::json!({
                "attached": true,
                "snapshotRevision": snapshot_revision,
                "liveRevision": live_revision,
                "queueToCommitMs": queue_to_commit_ms,
                "committed": committed
            }),
        );
    }
}

fn send_mcp_response(response: &SyncSender<ControlResponse>, value: JsonRpcResponse) {
    match serde_json::to_vec(&value) {
        Ok(bytes) if bytes.len() > MAX_CONTROL_MESSAGE_BYTES => {
            let _ = response.send(ControlResponse::error(
                "response_too_large",
                format!(
                    "MCP response is {} bytes, over the {}-byte frame cap; use pagination or narrower arguments",
                    bytes.len(),
                    MAX_CONTROL_MESSAGE_BYTES
                ),
            ));
        }
        Ok(_) => match serde_json::to_value(value) {
            Ok(value) => {
                let _ = response.send(ControlResponse::Mcp { response: value });
            }
            Err(error) => {
                let _ = response.send(ControlResponse::error(
                    "serialization_failed",
                    error.to_string(),
                ));
            }
        },
        Err(error) => {
            let _ = response.send(ControlResponse::error(
                "serialization_failed",
                error.to_string(),
            ));
        }
    }
}

/// Headless coordinator for the native Unix control socket and attached MCP workers.
pub struct AiControlManager {
    sender: Sender<ControlEvent>,
    receiver: Receiver<ControlEvent>,
    host: Option<ControlHost>,
    pending: VecDeque<PendingApproval>,
    trusted: Arc<Mutex<BTreeSet<String>>>,
    mcp: AttachedMcpState,
    tier: PermissionTier,
    last_publication: Option<String>,
    socket_path: Option<PathBuf>,
    token: Option<String>,
    last_status: String,
    activity_log: Vec<AiActivityEntry>,
}

impl Default for AiControlManager {
    fn default() -> Self {
        Self::new(BTreeSet::new())
    }
}

impl AiControlManager {
    /// Create a new headless AI control manager with optional pre-trusted clients.
    #[must_use]
    pub fn new(trusted: BTreeSet<String>) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            host: None,
            pending: VecDeque::new(),
            trusted: Arc::new(Mutex::new(trusted)),
            mcp: AttachedMcpState::default(),
            tier: PermissionTier::Edit,
            last_publication: None,
            socket_path: None,
            token: None,
            last_status: "AI attach standby: socket inactive".into(),
            activity_log: Vec::new(),
        }
    }

    /// Whether the control socket listener is currently active.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.host.is_some()
    }

    /// The current permission tier for attached clients.
    #[must_use]
    pub fn tier(&self) -> PermissionTier {
        self.tier
    }

    /// Update the permission tier.
    pub fn set_tier(&mut self, tier: PermissionTier) {
        self.tier = tier;
        self.activity_log.push(AiActivityEntry {
            timestamp: current_timestamp(),
            client_id: "operator".into(),
            operation: "set_permission_tier".into(),
            outcome: format!("{:?}", tier),
            queue_to_commit_ms: None,
            committed: false,
        });
    }

    /// Start listening on `$XDG_RUNTIME_DIR/alp/control.sock`.
    pub fn start(
        &mut self,
        runtime_dir: &Path,
        tier: PermissionTier,
    ) -> Result<AiControlStatus, String> {
        if self.is_enabled() {
            return Ok(self.status());
        }
        self.tier = tier;
        match ControlHost::bind(runtime_dir, Arc::clone(&self.trusted), self.sender.clone()) {
            Ok(host) => {
                let path = host.socket_path().to_path_buf();
                let token = host.token().to_owned();
                self.socket_path = Some(path.clone());
                self.token = Some(token);
                self.host = Some(host);
                self.last_status = format!("Live control socket active at {}", path.display());
                self.activity_log.push(AiActivityEntry {
                    timestamp: current_timestamp(),
                    client_id: "operator".into(),
                    operation: "start_ai_control".into(),
                    outcome: format!("bound {} with {:?}", path.display(), tier),
                    queue_to_commit_ms: None,
                    committed: false,
                });
                Ok(self.status())
            }
            Err(error) => {
                let message = format!("could not bind control socket: {error}");
                self.last_status = message.clone();
                Err(message)
            }
        }
    }

    /// Stop the control socket listener and revoke all connected clients.
    pub fn stop(&mut self, reason: &str) {
        self.host.take();
        self.mcp.clear_clients();
        self.last_publication = None;
        self.socket_path = None;
        self.token = None;
        for pending in self.pending.drain(..) {
            let _ = pending.response.send(ApprovalDecision::Deny);
        }
        self.last_status = reason.into();
        self.activity_log.push(AiActivityEntry {
            timestamp: current_timestamp(),
            client_id: "operator".into(),
            operation: "stop_ai_control".into(),
            outcome: reason.into(),
            queue_to_commit_ms: None,
            committed: false,
        });
    }

    /// Snapshot of current status for UI inspection.
    #[must_use]
    pub fn status(&self) -> AiControlStatus {
        let trusted_clients = self
            .trusted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .cloned()
            .collect();
        let pending_clients = self.pending.iter().map(|p| p.client_id.clone()).collect();
        AiControlStatus {
            enabled: self.is_enabled(),
            socket_path: self.socket_path.as_ref().map(|p| p.display().to_string()),
            token: self.token.clone(),
            tier: self.tier,
            trusted_clients,
            pending_clients,
            commits: self.mcp.commits,
            last_commit_ms: self.mcp.last_commit_ms,
            max_commit_ms: self.mcp.max_commit_ms,
            last_status: self.last_status.clone(),
        }
    }

    /// Retrieve the recent AI activity entries.
    #[must_use]
    pub fn activity_log(&self) -> Vec<AiActivityEntry> {
        self.activity_log.clone()
    }

    /// Respond to a pending client authentication request.
    pub fn respond_approval(
        &mut self,
        client_id: &str,
        decision: ApprovalDecision,
        remember: bool,
    ) -> bool {
        if let Some(pos) = self.pending.iter().position(|p| p.client_id == client_id) {
            let pending = self.pending.remove(pos).expect("position verified");
            if remember && decision != ApprovalDecision::Deny {
                self.trusted
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(client_id.to_string());
            }
            let _ = pending.response.send(decision);
            self.activity_log.push(AiActivityEntry {
                timestamp: current_timestamp(),
                client_id: client_id.into(),
                operation: "approval_decision".into(),
                outcome: format!("{:?} (remember={remember})", decision),
                queue_to_commit_ms: None,
                committed: false,
            });
            true
        } else {
            false
        }
    }

    /// Poll incoming control events and attached MCP worker completions,
    /// committing approved transactions to the live `EditSession`.
    pub fn tick(&mut self, session: &mut EditSession) -> Vec<AiEvent> {
        let mut events = Vec::new();

        // 1. Drain incoming socket requests (bounded to 64 per tick)
        for _ in 0..64 {
            match self.receiver.try_recv() {
                Ok(event) => {
                    if let Some(ai_event) = self.handle_control_event(event, session) {
                        events.push(ai_event);
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        // 2. Drain completed worker MCP transactions
        for completion in self.mcp.drain() {
            if let Some(ai_event) = self.handle_mcp_completion(completion, session) {
                events.push(ai_event);
            }
        }

        // 3. Check for operator edits and broadcast change notifications
        if self.is_enabled() {
            let summary = session.project_summary();
            let key = summary_key(&summary);
            if self.last_publication.as_ref() != Some(&key) {
                self.last_publication = Some(key);
                let change = ProjectChange {
                    origin: "operator".into(),
                    operation: "GUI edit".into(),
                    summary,
                    committed_at_ms: current_timestamp_ms(),
                    queue_to_commit_ms: None,
                };
                if let Some(host) = &self.host {
                    host.publish(change);
                }
            }
        }

        events
    }

    fn handle_control_event(
        &mut self,
        event: ControlEvent,
        session: &mut EditSession,
    ) -> Option<AiEvent> {
        match event {
            ControlEvent::ApprovalRequest {
                host_id,
                client_id,
                response,
            } => {
                if !self.is_enabled()
                    || !self.host.as_ref().is_some_and(|host| host.id() == host_id)
                {
                    let _ = response.send(ApprovalDecision::Deny);
                    return None;
                }
                self.activity_log.push(AiActivityEntry {
                    timestamp: current_timestamp(),
                    client_id: client_id.clone(),
                    operation: "approval_requested".into(),
                    outcome: "pending operator approval".into(),
                    queue_to_commit_ms: None,
                    committed: false,
                });
                self.pending.push_back(PendingApproval {
                    client_id: client_id.clone(),
                    response,
                });
                Some(AiEvent::ApprovalRequired { client_id })
            }
            ControlEvent::Request {
                host_id,
                client_id,
                request,
                response,
            } => {
                if let ControlRequest::Mcp { message, limits } = request {
                    self.handle_mcp_request(host_id, client_id, message, limits, response, session);
                    return None;
                }
                let operation = request.operation_name().to_string();
                let live = self.is_enabled()
                    && self.host.as_ref().is_some_and(|host| host.id() == host_id);
                let before_revision = session.revision();
                let result = if live {
                    dispatch_control(session, self.tier, &request)
                } else {
                    ControlResponse::error("session_revoked", "the operator stopped AI control")
                };
                let outcome = match &result {
                    ControlResponse::Error { code, message } => {
                        format!("refused ({code}: {message})")
                    }
                    _ => "ok".to_owned(),
                };
                let committed = session.revision() != before_revision;
                let entry = AiActivityEntry {
                    timestamp: current_timestamp(),
                    client_id: client_id.clone(),
                    operation: operation.clone(),
                    outcome,
                    queue_to_commit_ms: None,
                    committed,
                };
                self.activity_log.push(entry.clone());
                let _ = response.send(result);
                if committed {
                    self.publish_change(&client_id, &operation, None, session);
                    Some(AiEvent::RevisionChanged(session.revision()))
                } else {
                    Some(AiEvent::Activity(entry))
                }
            }
        }
    }

    fn handle_mcp_request(
        &mut self,
        host_id: u64,
        client_id: String,
        message: Value,
        limits: ControlLimits,
        response: SyncSender<ControlResponse>,
        session: &EditSession,
    ) {
        let live = self.is_enabled() && self.host.as_ref().is_some_and(|host| host.id() == host_id);
        if !live {
            let _ = response.send(ControlResponse::error(
                "session_revoked",
                "the operator stopped AI control",
            ));
            return;
        }
        let Some(state) = session.attached_project_state() else {
            let _ = response.send(ControlResponse::error(
                "project_not_saved",
                "save the live project before using the full MCP attach surface",
            ));
            return;
        };
        if let Err(error) = self.mcp.dispatch(AttachedCall {
            host_id,
            client_id,
            request_value: message,
            limits,
            state,
            summary: session.project_summary(),
            tier: self.tier,
            response: response.clone(),
        }) {
            let _ = response.send(ControlResponse::error("dispatch_failed", error));
        }
    }

    fn handle_mcp_completion(
        &mut self,
        completion: AttachedCompletion,
        session: &mut EditSession,
    ) -> Option<AiEvent> {
        let AttachedCompletion {
            base_revision,
            tier,
            host_id,
            client_id,
            request,
            mut response,
            transaction,
            original_response,
            queued,
        } = completion;
        let operation = live::tool_name(&request)
            .unwrap_or(request.method.as_str())
            .to_owned();
        let live = self.is_enabled()
            && self.tier == tier
            && self.host.as_ref().is_some_and(|host| host.id() == host_id);
        if !live {
            let _ = original_response.send(ControlResponse::error(
                "session_revoked",
                "the operator stopped AI control or changed its permission tier",
            ));
            return None;
        }

        let mut committed = false;
        let mut failed = response_failed(&response);
        if !failed && transaction.is_none() && base_revision != session.revision() {
            let live_revision = session.revision();
            let message = format!(
                "revision conflict: this call read project snapshot {base_revision}, but the live project is {live_revision}; the result is stale. Call project_info and retry"
            );
            if request.method == "tools/call" {
                live::fail_commit(&mut response, message);
            } else {
                response = JsonRpcResponse::error(
                    request.id.clone().unwrap_or(Value::Null),
                    -32000,
                    message,
                    Some(serde_json::json!({
                        "code": "revision_conflict",
                        "snapshotRevision": base_revision,
                        "liveRevision": live_revision
                    })),
                );
            }
            failed = true;
        }

        if !failed
            && let Some(tool) = live::tool_name(&request)
            && matches!(tool, "history_undo" | "history_redo" | "history_list")
        {
            match tool {
                "history_undo" | "history_redo" => {
                    let expected = request.params["arguments"]["expected_revision"].as_u64();
                    if expected.is_some_and(|revision| revision != session.revision()) {
                        live::fail_commit(
                            &mut response,
                            format!(
                                "revision conflict: expected {}, live project is {}; call project_info and retry",
                                expected.unwrap_or_default(),
                                session.revision()
                            ),
                        );
                        failed = true;
                    } else {
                        let before = if tool == "history_undo" {
                            session.undo_steps()
                        } else {
                            session.redo_steps()
                        };
                        if tool == "history_undo" {
                            session.undo();
                        } else {
                            session.redo();
                        }
                        let changed = if tool == "history_undo" {
                            session.undo_steps() < before
                        } else {
                            session.redo_steps() < before
                        };
                        let result = if tool == "history_undo" {
                            live::history_undo_result(changed)
                        } else {
                            live::history_redo_result(changed)
                        };
                        response = JsonRpcResponse::success(
                            request.id.clone().unwrap_or(Value::Null),
                            serde_json::to_value(result).unwrap_or_default(),
                        );
                        if changed {
                            committed = true;
                        }
                    }
                }
                "history_list" => {
                    let arguments = request.params["arguments"].clone();
                    let result = live::history_list_result(session.history_entries(), &arguments);
                    response = JsonRpcResponse::success(
                        request.id.clone().unwrap_or(Value::Null),
                        serde_json::to_value(result).unwrap_or_default(),
                    );
                }
                _ => unreachable!(),
            }
        } else if let Some(transaction) = transaction {
            match transaction {
                PreparedTransaction::Saved {
                    base_revision: base,
                } => {
                    if base == session.revision() {
                        session.mark_attached_saved();
                    } else {
                        live::fail_commit(
                            &mut response,
                            format!(
                                "save raced a live edit: the worker wrote a revision {base} snapshot, but the GUI is now revision {}; no GUI state was replaced. Re-read project_info and save again",
                                session.revision()
                            ),
                        );
                        failed = true;
                    }
                }
                PreparedTransaction::Edit {
                    base_revision: base,
                    state,
                    audio,
                } => match session.commit_attached_project(base, *state, audio, &operation) {
                    Ok(()) => {
                        committed = true;
                    }
                    Err(error) => {
                        live::fail_commit(
                            &mut response,
                            format!(
                                "revision conflict or invalid attached edit: {error}; no live project edit was committed. Call project_info and retry"
                            ),
                        );
                        failed = true;
                    }
                },
            }
        }

        let elapsed = elapsed_ms(queued);
        if committed {
            self.mcp.last_commit_ms = Some(elapsed);
            self.mcp.max_commit_ms = self.mcp.max_commit_ms.max(elapsed);
            self.mcp.commits += 1;
            self.publish_change(&client_id, &operation, Some(elapsed), session);
        } else {
            failed |= response_failed(&response);
        }
        add_attach_meta(
            &mut response,
            base_revision,
            session.revision(),
            elapsed,
            committed,
        );
        let entry = AiActivityEntry {
            timestamp: current_timestamp(),
            client_id: client_id.clone(),
            operation,
            outcome: if committed {
                format!("committed ({elapsed} ms)")
            } else if failed {
                "failed".into()
            } else {
                "ok".into()
            },
            queue_to_commit_ms: if committed { Some(elapsed) } else { None },
            committed,
        };
        self.activity_log.push(entry.clone());
        send_mcp_response(&original_response, response);

        if committed {
            Some(AiEvent::RevisionChanged(session.revision()))
        } else {
            Some(AiEvent::Activity(entry))
        }
    }

    fn publish_change(
        &mut self,
        origin: &str,
        operation: &str,
        queue_to_commit_ms: Option<u64>,
        session: &EditSession,
    ) {
        let summary = session.project_summary();
        self.last_publication = Some(summary_key(&summary));
        if let Some(host) = &self.host {
            host.publish(ProjectChange {
                origin: origin.into(),
                operation: operation.into(),
                summary,
                committed_at_ms: current_timestamp_ms(),
                queue_to_commit_ms,
            });
        }
    }
}

fn response_failed(response: &JsonRpcResponse) -> bool {
    response.error.is_some()
        || response
            .result
            .as_ref()
            .is_some_and(|result| result.get("isError").and_then(Value::as_bool) == Some(true))
}

/// Dispatch native typed requests directly into the live `EditSession`.
fn dispatch_control(
    session: &mut EditSession,
    tier: PermissionTier,
    request: &ControlRequest,
) -> ControlResponse {
    if !tier.allows(request.required_tier()) {
        return ControlResponse::error(
            "permission_denied",
            format!(
                "{} requires {:?} tier; call project_info, then ask the operator to raise the UI tier",
                request.operation_name(),
                request.required_tier()
            ),
        );
    }
    if let Some(expected) = request.expected_revision()
        && expected != session.revision()
    {
        return ControlResponse::error(
            "revision_conflict",
            format!(
                "expected revision {expected}, but the live project is {}; call project_info and retry with its revision",
                session.revision()
            ),
        );
    }

    match request {
        ControlRequest::ProjectInfo => ControlResponse::ProjectInfo {
            summary: Box::new(session.project_summary()),
        },
        ControlRequest::TrackList => {
            let tracks = session
                .tracks()
                .iter()
                .enumerate()
                .map(|(index, track)| TrackSummary {
                    id: track.id.to_string(),
                    name: track.name.clone(),
                    index,
                    gain: track.gain,
                    pan: track.pan,
                    muted: track.muted,
                    solo: track.solo,
                    armed: track.armed,
                    clip_count: track.clips.len(),
                })
                .collect();
            ControlResponse::TrackList { tracks }
        }
        ControlRequest::TrackAdd { name, .. } => match session.add_track(name) {
            Ok(track) => ControlResponse::TrackAdded {
                track: Box::new(track),
                revision: session.revision(),
            },
            Err(message) => ControlResponse::error("validation", message),
        },
        ControlRequest::ClipSplit {
            clip_id,
            split_secs,
            ..
        } => match session.split_clip_at(clip_id, *split_secs) {
            Ok(()) => ControlResponse::ClipSplit {
                revision: session.revision(),
            },
            Err(message) => ControlResponse::error("validation", message),
        },
        ControlRequest::Authenticate { .. } | ControlRequest::Mcp { .. } => ControlResponse::error(
            "bad_request",
            "authentication and MCP are handled outside the native project dispatcher",
        ),
        ControlRequest::Subscribe => ControlResponse::error(
            "bad_request",
            "change subscriptions are handled by the socket host",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alp_api::control::{CONTROL_PROTOCOL_VERSION, read_response, write_request};
    use std::io::BufReader;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    #[test]
    fn test_ai_control_observe_tier_blocks_edits() {
        let mut session = EditSession::demo();
        let initial_tracks = session.tracks().len();
        let response = dispatch_control(
            &mut session,
            PermissionTier::Observe,
            &ControlRequest::TrackAdd {
                name: "BlockedTrack".into(),
                expected_revision: None,
            },
        );
        assert!(matches!(
            response,
            ControlResponse::Error { ref code, .. } if code == "permission_denied"
        ));
        assert_eq!(session.tracks().len(), initial_tracks);
    }

    #[test]
    fn test_ai_control_edit_tier_adds_track_and_is_undoable() {
        let mut session = EditSession::demo();
        let initial_tracks = session.tracks().len();
        let revision = session.revision();
        let response = dispatch_control(
            &mut session,
            PermissionTier::Edit,
            &ControlRequest::TrackAdd {
                name: "AgentTrack".into(),
                expected_revision: Some(revision),
            },
        );
        assert!(matches!(response, ControlResponse::TrackAdded { .. }));
        assert_eq!(session.tracks().len(), initial_tracks + 1);
        assert_eq!(session.revision(), revision + 1);

        // Stale revision is rejected
        let stale = dispatch_control(
            &mut session,
            PermissionTier::Edit,
            &ControlRequest::TrackAdd {
                name: "StaleTrack".into(),
                expected_revision: Some(revision),
            },
        );
        assert!(matches!(
            stale,
            ControlResponse::Error { ref code, .. } if code == "revision_conflict"
        ));
        assert_eq!(session.tracks().len(), initial_tracks + 1);

        // Human/operator undo removes the AI track
        session.undo();
        assert_eq!(session.tracks().len(), initial_tracks);
    }

    #[test]
    fn test_ai_control_manager_socket_lifecycle_and_tick() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let mut manager = AiControlManager::default();
        let status = manager
            .start(temp_dir.path(), PermissionTier::Edit)
            .expect("start host");
        assert!(status.enabled);
        assert!(status.token.is_some());
        assert!(status.socket_path.is_some());

        let mut stream = UnixStream::connect(status.socket_path.unwrap()).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));

        write_request(
            &mut stream,
            &ControlRequest::Authenticate {
                protocol_version: CONTROL_PROTOCOL_VERSION,
                token: status.token.unwrap(),
                client_id: "test-client".into(),
            },
        )
        .expect("write auth");

        // First response is approval required
        assert!(matches!(
            read_response(&mut reader).expect("approval"),
            Some(ControlResponse::ApprovalRequired { .. })
        ));

        let mut session = EditSession::demo();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut approved = false;
        while std::time::Instant::now() < deadline {
            let events = manager.tick(&mut session);
            if events.iter().any(|e| matches!(e, AiEvent::ApprovalRequired { client_id } if client_id == "test-client")) {
                approved = manager.respond_approval("test-client", ApprovalDecision::Session, true);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(approved, "client should be approved");

        // Read authenticated response
        assert!(matches!(
            read_response(&mut reader).expect("authenticated"),
            Some(ControlResponse::Authenticated { .. })
        ));

        // Send a ProjectInfo request
        write_request(&mut stream, &ControlRequest::ProjectInfo).expect("write info");
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut got_info = false;
        while std::time::Instant::now() < deadline {
            let _ = manager.tick(&mut session);
            if let Ok(Some(ControlResponse::ProjectInfo { .. })) = read_response(&mut reader) {
                got_info = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(got_info, "should receive project info response");

        manager.stop("test complete");
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_attached_mcp_full_catalog_and_tool_commit() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let project_dir = temp_dir.path().join("saved_proj");
        let mut session = EditSession::open_or_demo(project_dir);
        let initial_tracks = session.tracks().len();
        let initial_undo = session.undo_steps();

        let mut trusted = BTreeSet::new();
        trusted.insert("mcp-agent".into());
        let mut manager = AiControlManager::new(trusted);
        let status = manager
            .start(temp_dir.path(), PermissionTier::Edit)
            .expect("start host");

        let mut stream = UnixStream::connect(status.socket_path.unwrap()).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));

        // Authenticate trusted client (no interactive approval required)
        write_request(
            &mut stream,
            &ControlRequest::Authenticate {
                protocol_version: CONTROL_PROTOCOL_VERSION,
                token: status.token.unwrap(),
                client_id: "mcp-agent".into(),
            },
        )
        .expect("write auth");

        assert!(matches!(
            read_response(&mut reader).expect("auth response"),
            Some(ControlResponse::Authenticated { .. })
        ));

        let (responses_tx, responses_rx) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(Some(resp)) = read_response(&mut reader) {
                if responses_tx.send(resp).is_err() {
                    break;
                }
            }
        });

        fn call_mcp(
            stream: &mut UnixStream,
            rx: &Receiver<ControlResponse>,
            manager: &mut AiControlManager,
            session: &mut EditSession,
            req: JsonRpcRequest,
        ) -> JsonRpcResponse {
            write_request(
                stream,
                &ControlRequest::Mcp {
                    message: serde_json::to_value(req).unwrap(),
                    limits: Default::default(),
                },
            )
            .expect("write mcp");

            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let _ = manager.tick(session);
                match rx.try_recv() {
                    Ok(ControlResponse::Mcp { response }) => {
                        return serde_json::from_value(response).expect("decode jsonrpc");
                    }
                    Ok(other) => panic!("unexpected response: {other:?}"),
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("reader disconnected"),
                }
                assert!(Instant::now() < deadline, "call timed out");
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        // 1. Test tools/list
        let list_resp = call_mcp(
            &mut stream,
            &responses_rx,
            &mut manager,
            &mut session,
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(1)),
                method: "tools/list".into(),
                params: serde_json::json!({}),
            },
        );
        let tools = list_resp.result.unwrap()["tools"]
            .as_array()
            .unwrap()
            .clone();
        assert!(tools.len() > 4, "expected full MCP tool catalog");

        // 2. Test tools/call track_add
        let call_resp = call_mcp(
            &mut stream,
            &responses_rx,
            &mut manager,
            &mut session,
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(2)),
                method: "tools/call".into(),
                params: serde_json::json!({
                    "name": "track_add",
                    "arguments": { "name": "AI MCP Synth Lead" }
                }),
            },
        );
        let result = call_resp.result.unwrap();
        assert_eq!(result["_meta"]["committed"], true);
        assert_eq!(session.tracks().len(), initial_tracks + 1);
        assert_eq!(session.undo_steps(), initial_undo + 1);
        assert_eq!(manager.status().commits, 1);

        // 3. Test history_undo via MCP
        let undo_resp = call_mcp(
            &mut stream,
            &responses_rx,
            &mut manager,
            &mut session,
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(3)),
                method: "tools/call".into(),
                params: serde_json::json!({
                    "name": "history_undo",
                    "arguments": {}
                }),
            },
        );
        let undo_res = undo_resp.result.unwrap();
        assert_eq!(undo_res["structuredContent"]["undone"], true);
        assert_eq!(session.tracks().len(), initial_tracks);
        assert_eq!(session.undo_steps(), initial_undo);

        manager.stop("done");
    }
}
