//! Live AI control admission and UI-thread dispatch.
//!
//! Socket threads enqueue typed work and wait off the UI thread. The dispatcher
//! Legacy cheap calls execute immediately. The shared MCP catalog executes on
//! `ai_mcp` workers against revision-tagged scratch state; only validation,
//! revision-checked commits and view synchronization run on the UI thread.

use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};

use alp_api::control::{
    ApprovalDecision, ControlEvent, ControlHost, ControlRequest, ControlResponse, ProjectChange,
};
use alp_api::{PermissionTier, ProjectSummary, TrackSummary};
use alp_ui_kit::components;
use alp_ui_kit::overlays;
use alp_ui_kit::{ButtonKind, ThemeColors};
use iced::widget::{Space, column, row, text};
use iced::{Element, Length};

use super::ai_mcp::{
    AttachedCall, AttachedCompletion, AttachedMcpState, PreparedTransaction, elapsed_ms,
};
use super::{App, Message};
use crate::session::EditSession;

/// UI-owned endpoint state. The token lives here only while the host is live.
pub(super) struct AiControlState {
    sender: Sender<ControlEvent>,
    receiver: Receiver<ControlEvent>,
    host: Option<ControlHost>,
    pending: VecDeque<PendingApproval>,
    trusted: Arc<Mutex<BTreeSet<String>>>,
    pub(super) mcp: AttachedMcpState,
    last_publication: Option<String>,
    pub(super) socket_path: Option<PathBuf>,
    pub(super) token: Option<String>,
}

struct PendingApproval {
    client_id: String,
    response: SyncSender<ApprovalDecision>,
}

impl AiControlState {
    fn new(trusted: BTreeSet<String>) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            host: None,
            pending: VecDeque::new(),
            trusted: Arc::new(Mutex::new(trusted)),
            mcp: AttachedMcpState::default(),
            last_publication: None,
            socket_path: None,
            token: None,
        }
    }

    pub(super) fn pending_client(&self) -> Option<&str> {
        self.pending
            .front()
            .map(|pending| pending.client_id.as_str())
    }

    fn trusted_clients(&self) -> BTreeSet<String> {
        self.trusted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl App {
    pub(super) fn ai_control_state(trusted: BTreeSet<String>) -> AiControlState {
        AiControlState::new(trusted)
    }

    pub(super) fn trusted_ai_clients(&self) -> BTreeSet<String> {
        self.ai_control.trusted_clients()
    }

    pub(super) fn toggle_ai_control(&mut self) {
        if self.ai_enabled {
            self.stop_ai_control("AI attach stopped; every client was revoked");
            self.show_toast("AI attach stopped and clients revoked");
            return;
        }
        let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
            self.ai_status = "AI attach unavailable: XDG_RUNTIME_DIR is not set".into();
            self.show_toast("Cannot start AI attach without XDG_RUNTIME_DIR");
            return;
        };
        match ControlHost::bind(
            std::path::Path::new(&runtime),
            Arc::clone(&self.ai_control.trusted),
            self.ai_control.sender.clone(),
        ) {
            Ok(host) => {
                self.ai_control.socket_path = Some(host.socket_path().to_path_buf());
                self.ai_control.token = Some(host.token().to_owned());
                self.ai_control.host = Some(host);
                self.ai_control.last_publication =
                    Some(summary_key(&self.session.project_summary()));
                self.ai_enabled = true;
                self.ai_status = format!(
                    "Live control socket · {} · token required",
                    self.ai_control
                        .socket_path
                        .as_ref()
                        .map_or_else(|| "unknown path".into(), |path| path.display().to_string())
                );
                self.ai_activity_log.push(format!(
                    "Control host started with {:?} permission; new clients need approval",
                    self.ai_tier
                ));
                self.show_toast("AI control socket enabled; copy the session token to attach");
            }
            Err(error) => {
                self.ai_enabled = false;
                self.ai_status = format!("AI control socket failed: {error}");
                self.show_toast("Could not start AI control socket");
            }
        }
    }

    pub(super) fn stop_ai_control(&mut self, status: &str) {
        self.ai_enabled = false;
        self.ai_control.host.take();
        self.ai_control.mcp.clear_clients();
        self.ai_control.last_publication = None;
        self.ai_control.socket_path = None;
        self.ai_control.token = None;
        for pending in self.ai_control.pending.drain(..) {
            let _ = pending.response.send(ApprovalDecision::Deny);
        }
        self.ai_status = status.into();
    }

    /// Drain bounded work per frame; a client cannot make the UI wait on I/O.
    pub(super) fn poll_ai_control(&mut self) {
        for _ in 0..64 {
            match self.ai_control.receiver.try_recv() {
                Ok(event) => drop(self.handle_message(Message::AiControl(event))),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        for completion in self.ai_control.mcp.drain() {
            self.handle_attached_mcp_completion(completion);
        }
        if self.ai_enabled {
            self.publish_ai_change("operator", "GUI edit", None);
        }
    }

    pub(super) fn handle_ai_control_event(&mut self, event: ControlEvent) {
        match event {
            ControlEvent::ApprovalRequest {
                host_id,
                client_id,
                response,
            } => {
                if !self.ai_enabled
                    || !self
                        .ai_control
                        .host
                        .as_ref()
                        .is_some_and(|host| host.id() == host_id)
                {
                    let _ = response.send(ApprovalDecision::Deny);
                    return;
                }
                self.ai_activity_log
                    .push(format!("Approval requested by {client_id}"));
                self.ai_control.pending.push_back(PendingApproval {
                    client_id,
                    response,
                });
            }
            ControlEvent::Request {
                host_id,
                client_id,
                request,
                response,
            } => {
                if let ControlRequest::Mcp { message, limits } = request {
                    self.handle_attached_mcp_request(host_id, client_id, message, limits, response);
                    return;
                }
                let operation = request.operation_name();
                let live = self.ai_enabled
                    && self
                        .ai_control
                        .host
                        .as_ref()
                        .is_some_and(|host| host.id() == host_id);
                let before_revision = self.session.revision();
                let result = if live {
                    dispatch_control(&mut self.session, self.ai_tier, &request)
                } else {
                    ControlResponse::error("session_revoked", "the operator stopped AI control")
                };
                let outcome = match &result {
                    ControlResponse::Error { code, .. } => format!("refused ({code})"),
                    _ => "ok".to_owned(),
                };
                self.ai_activity_log
                    .push(format!("{client_id}: {operation} · {outcome}"));
                let _ = response.send(result);
                if matches!(
                    request,
                    ControlRequest::TrackAdd { .. } | ControlRequest::ClipSplit { .. }
                ) {
                    self.sync_edit_fields();
                }
                if self.session.revision() != before_revision {
                    self.publish_ai_change(&client_id, operation, None);
                }
            }
        }
    }

    fn handle_attached_mcp_request(
        &mut self,
        host_id: u64,
        client_id: String,
        message: serde_json::Value,
        limits: alp_api::control::ControlLimits,
        response: SyncSender<ControlResponse>,
    ) {
        let live = self.ai_enabled
            && self
                .ai_control
                .host
                .as_ref()
                .is_some_and(|host| host.id() == host_id);
        if !live {
            let _ = response.send(ControlResponse::error(
                "session_revoked",
                "the operator stopped AI control",
            ));
            return;
        }
        let Some(state) = self.session.attached_project_state() else {
            let _ = response.send(ControlResponse::error(
                "project_not_saved",
                "save the live project before using the full MCP attach surface",
            ));
            return;
        };
        if let Err(error) = self.ai_control.mcp.dispatch(AttachedCall {
            host_id,
            client_id,
            request_value: message,
            limits,
            state,
            summary: self.session.project_summary(),
            tier: self.ai_tier,
            response: response.clone(),
        }) {
            let _ = response.send(ControlResponse::error("dispatch_failed", error));
        }
    }

    fn handle_attached_mcp_completion(&mut self, completion: AttachedCompletion) {
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
        let operation = alp_mcp::live::tool_name(&request)
            .unwrap_or(request.method.as_str())
            .to_owned();
        let live = self.ai_enabled
            && self.ai_tier == tier
            && self
                .ai_control
                .host
                .as_ref()
                .is_some_and(|host| host.id() == host_id);
        if !live {
            let _ = original_response.send(ControlResponse::error(
                "session_revoked",
                "the operator stopped AI control or changed its permission tier",
            ));
            return;
        }

        let mut committed = false;
        let mut failed = response_failed(&response);
        if !failed && transaction.is_none() && base_revision != self.session.revision() {
            let live_revision = self.session.revision();
            let message = format!(
                "revision conflict: this call read project snapshot {base_revision}, but the live project is {live_revision}; the result is stale. Call project_info and retry"
            );
            if request.method == "tools/call" {
                alp_mcp::live::fail_commit(&mut response, message);
            } else {
                response = alp_mcp::JsonRpcResponse::error(
                    request.id.clone().unwrap_or(serde_json::Value::Null),
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
            && let Some(tool) = alp_mcp::live::tool_name(&request)
            && matches!(tool, "history_undo" | "history_redo" | "history_list")
        {
            match tool {
                "history_undo" | "history_redo" => {
                    let expected = request.params["arguments"]["expected_revision"].as_u64();
                    if expected.is_some_and(|revision| revision != self.session.revision()) {
                        alp_mcp::live::fail_commit(
                            &mut response,
                            format!(
                                "revision conflict: expected {}, live project is {}; call project_info and retry",
                                expected.unwrap_or_default(),
                                self.session.revision()
                            ),
                        );
                        failed = true;
                    } else {
                        let before = if tool == "history_undo" {
                            self.session.undo_steps()
                        } else {
                            self.session.redo_steps()
                        };
                        if tool == "history_undo" {
                            self.session.undo();
                        } else {
                            self.session.redo();
                        }
                        let changed = if tool == "history_undo" {
                            self.session.undo_steps() < before
                        } else {
                            self.session.redo_steps() < before
                        };
                        let result = if tool == "history_undo" {
                            alp_mcp::live::history_undo_result(changed)
                        } else {
                            alp_mcp::live::history_redo_result(changed)
                        };
                        response = alp_mcp_json_response(
                            &request,
                            serde_json::to_value(result).unwrap_or_default(),
                        );
                        if changed {
                            committed = true;
                            self.sync_edit_fields();
                        }
                    }
                }
                "history_list" => {
                    let arguments = request.params["arguments"].clone();
                    let result = alp_mcp::live::history_list_result(
                        self.session.history_entries(),
                        &arguments,
                    );
                    response = alp_mcp_json_response(
                        &request,
                        serde_json::to_value(result).unwrap_or_default(),
                    );
                }
                _ => unreachable!(),
            }
        } else if let Some(transaction) = transaction {
            match transaction {
                PreparedTransaction::Saved { base_revision } => {
                    if base_revision == self.session.revision() {
                        self.session.mark_attached_saved();
                    } else {
                        alp_mcp::live::fail_commit(
                            &mut response,
                            format!(
                                "save raced a live edit: the worker wrote a revision {base_revision} snapshot, but the GUI is now revision {}; no GUI state was replaced. Re-read project_info and save again",
                                self.session.revision()
                            ),
                        );
                        failed = true;
                    }
                }
                PreparedTransaction::Edit {
                    base_revision,
                    state,
                    audio,
                } => match self.session.commit_attached_project(
                    base_revision,
                    *state,
                    audio,
                    &operation,
                ) {
                    Ok(()) => {
                        committed = true;
                        self.sync_edit_fields();
                    }
                    Err(error) => {
                        alp_mcp::live::fail_commit(
                            &mut response,
                            format!(
                                "revision conflict or invalid attached edit: {error}; no live project edit was committed. Any worker-generated media remains in the workspace rather than being silently deleted. Call project_info and retry"
                            ),
                        );
                        failed = true;
                    }
                },
            }
        }

        let elapsed = elapsed_ms(queued);
        if committed {
            self.ai_control.mcp.last_commit_ms = Some(elapsed);
            self.ai_control.mcp.max_commit_ms = self.ai_control.mcp.max_commit_ms.max(elapsed);
            self.ai_control.mcp.commits += 1;
            self.publish_ai_change(&client_id, &operation, Some(elapsed));
        } else {
            failed |= response_failed(&response);
        }
        add_attach_meta(
            &mut response,
            base_revision,
            self.session.revision(),
            elapsed,
            committed,
        );
        self.ai_activity_log.push(format!(
            "{client_id}: {operation} · {}{}",
            if committed {
                "committed"
            } else if failed {
                "failed"
            } else {
                "ok"
            },
            if committed {
                format!(" · {elapsed} ms · one live undo step")
            } else {
                String::new()
            }
        ));
        send_mcp_response(&original_response, response);
    }

    fn publish_ai_change(&mut self, origin: &str, operation: &str, latency_ms: Option<u64>) {
        let summary = self.session.project_summary();
        let key = summary_key(&summary);
        if self.ai_control.last_publication.as_deref() == Some(key.as_str()) {
            return;
        }
        self.ai_control.last_publication = Some(key);
        if let Some(host) = &self.ai_control.host {
            host.publish(ProjectChange {
                summary,
                origin: origin.to_owned(),
                operation: operation.to_owned(),
                committed_at_ms: alp_core::now_ms(),
                queue_to_commit_ms: latency_ms,
            });
        }
    }

    pub(super) fn resolve_ai_approval(&mut self, decision: ApprovalDecision) {
        let Some(pending) = self.ai_control.pending.pop_front() else {
            return;
        };
        if decision == ApprovalDecision::Always {
            self.ai_control
                .trusted
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert(pending.client_id.clone());
            self.save_workspace();
        }
        self.ai_activity_log.push(format!(
            "{} {}{}",
            match decision {
                ApprovalDecision::Deny => "Denied",
                ApprovalDecision::Session | ApprovalDecision::Always => "Approved",
            },
            pending.client_id,
            match decision {
                ApprovalDecision::Always => " for future sessions",
                ApprovalDecision::Session => " for this control session",
                ApprovalDecision::Deny => "",
            }
        ));
        let _ = pending.response.send(decision);
    }

    pub(super) fn ai_approval_modal_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let client = self.ai_control.pending_client().unwrap_or("unknown-client");
        let body = column![
            text(client)
                .size(18.0)
                .font(alp_ui_kit::fonts::mono())
                .color(colors.text),
            components::muted(
                "The socket excludes other users and a new token is required for every control session. Approval permits only typed operations within the selected tier; cloning, model installation, settings changes and shell execution are not part of this protocol.",
                colors,
            ),
        ]
        .spacing(12.0);
        let footer = row![
            components::button(
                "Deny",
                ButtonKind::Danger,
                Some(Message::ResolveAiApproval(ApprovalDecision::Deny)),
            ),
            Space::new().width(Length::Fill),
            components::button(
                "This session",
                ButtonKind::Secondary,
                Some(Message::ResolveAiApproval(ApprovalDecision::Session)),
            ),
            components::button(
                "Always allow client",
                ButtonKind::Primary,
                Some(Message::ResolveAiApproval(ApprovalDecision::Always)),
            ),
        ]
        .spacing(10.0);
        overlays::dialog(
            "Approve AI client?",
            "A new client authenticated to this window's live control socket.",
            body,
            footer,
            620.0,
            colors,
            Message::ResolveAiApproval(ApprovalDecision::Deny),
        )
    }
}

fn summary_key(summary: &ProjectSummary) -> String {
    serde_json::to_string(summary)
        .unwrap_or_else(|_| format!("{}:{}", summary.id, summary.revision))
}

fn response_failed(response: &alp_mcp::JsonRpcResponse) -> bool {
    response.error.is_some()
        || response.result.as_ref().is_some_and(|result| {
            result.get("isError").and_then(serde_json::Value::as_bool) == Some(true)
        })
}

fn alp_mcp_json_response(
    request: &alp_mcp::JsonRpcRequest,
    result: serde_json::Value,
) -> alp_mcp::JsonRpcResponse {
    alp_mcp::JsonRpcResponse::success(
        request.id.clone().unwrap_or(serde_json::Value::Null),
        result,
    )
}

fn add_attach_meta(
    response: &mut alp_mcp::JsonRpcResponse,
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

fn send_mcp_response(response: &SyncSender<ControlResponse>, value: alp_mcp::JsonRpcResponse) {
    match serde_json::to_vec(&value) {
        Ok(bytes) if bytes.len() > alp_api::control::MAX_CONTROL_MESSAGE_BYTES => {
            let _ = response.send(ControlResponse::error(
                "response_too_large",
                format!(
                    "MCP response is {} bytes, over the {}-byte frame cap; use pagination or narrower arguments",
                    bytes.len(),
                    alp_api::control::MAX_CONTROL_MESSAGE_BYTES
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

/// Execute one call against the GUI's only live project and undo stack.
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
        ControlRequest::ProjectInfo => {
            let project = session.project();
            ControlResponse::ProjectInfo {
                summary: Box::new(ProjectSummary {
                    id: project.id.to_string(),
                    name: project.name.clone(),
                    revision: session.revision(),
                    sample_rate: project.sample_rate,
                    duration_secs: session.duration_secs(),
                    track_count: project.tracks.len(),
                    clip_count: project.tracks.iter().map(|track| track.clips.len()).sum(),
                    undo_steps: session.undo_steps(),
                    redo_steps: session.redo_steps(),
                    is_dirty: session.is_dirty(),
                }),
            }
        }
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

    #[test]
    fn observe_reads_the_live_project_but_cannot_edit_it() {
        let mut session = EditSession::demo();
        let tracks = session.tracks().len();
        let ControlResponse::ProjectInfo { summary } = dispatch_control(
            &mut session,
            PermissionTier::Observe,
            &ControlRequest::ProjectInfo,
        ) else {
            panic!("project info should be readable");
        };
        assert_eq!(summary.id, session.project().id.to_string());
        assert_eq!(summary.track_count, tracks);

        let result = dispatch_control(
            &mut session,
            PermissionTier::Observe,
            &ControlRequest::TrackAdd {
                name: "Narration".into(),
                expected_revision: None,
            },
        );
        assert!(matches!(
            result,
            ControlResponse::Error { ref code, .. } if code == "permission_denied"
        ));
        assert_eq!(session.tracks().len(), tracks);
    }

    #[test]
    fn an_attached_track_edit_is_undoable_and_stale_writes_are_atomic() {
        let mut session = EditSession::demo();
        let tracks = session.tracks().len();
        let revision = session.revision();
        let result = dispatch_control(
            &mut session,
            PermissionTier::Edit,
            &ControlRequest::TrackAdd {
                name: "Narration".into(),
                expected_revision: Some(revision),
            },
        );
        assert!(matches!(result, ControlResponse::TrackAdded { .. }));
        assert_eq!(session.tracks().len(), tracks + 1);
        assert!(session.can_undo());

        let after_edit = session.revision();
        let stale = dispatch_control(
            &mut session,
            PermissionTier::Edit,
            &ControlRequest::TrackAdd {
                name: "Stale".into(),
                expected_revision: Some(revision),
            },
        );
        assert!(matches!(
            stale,
            ControlResponse::Error { ref code, .. } if code == "revision_conflict"
        ));
        assert_eq!(session.tracks().len(), tracks + 1);
        assert_eq!(session.revision(), after_edit);

        session.undo();
        assert_eq!(session.tracks().len(), tracks);
    }

    #[test]
    fn an_attached_split_uses_the_same_live_undo_stack() {
        let mut session = EditSession::demo();
        let clip = session
            .project()
            .tracks
            .first()
            .and_then(|track| track.clips.first())
            .expect("demo clip");
        let id = clip.id.to_string();
        let before = session.clip_count();
        let revision = session.revision();
        let result = dispatch_control(
            &mut session,
            PermissionTier::Edit,
            &ControlRequest::ClipSplit {
                clip_id: id,
                split_secs: 1.0,
                expected_revision: Some(revision),
            },
        );
        assert!(matches!(result, ControlResponse::ClipSplit { .. }));
        assert_eq!(session.clip_count(), before + 1);
        session.undo();
        assert_eq!(session.clip_count(), before);
    }

    #[test]
    fn production_socket_track_add_commits_to_the_live_undoable_project() {
        use std::io::BufReader;
        use std::os::unix::net::UnixStream;
        use std::time::Duration;

        use alp_api::control::{CONTROL_PROTOCOL_VERSION, read_response, write_request};

        let runtime = tempfile::tempdir().expect("tempdir");
        let (events, receiver) = mpsc::channel();
        let host = ControlHost::bind(
            runtime.path(),
            Arc::new(Mutex::new(BTreeSet::new())),
            events,
        )
        .expect("bind production host");
        let mut stream = UnixStream::connect(host.socket_path()).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        write_request(
            &mut stream,
            &ControlRequest::Authenticate {
                protocol_version: CONTROL_PROTOCOL_VERSION,
                token: host.token().into(),
                client_id: "socket-integration".into(),
            },
        )
        .expect("authenticate");
        assert!(matches!(
            read_response(&mut reader).expect("approval response"),
            Some(ControlResponse::ApprovalRequired { .. })
        ));
        let ControlEvent::ApprovalRequest { response, .. } = receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("approval event")
        else {
            panic!("expected approval");
        };
        response.send(ApprovalDecision::Session).expect("approve");
        assert!(matches!(
            read_response(&mut reader).expect("authenticated response"),
            Some(ControlResponse::Authenticated { .. })
        ));

        let mut session = EditSession::demo();
        let before = session.tracks().len();
        let revision = session.revision();
        write_request(
            &mut stream,
            &ControlRequest::TrackAdd {
                name: "Socket-created track".into(),
                expected_revision: Some(revision),
            },
        )
        .expect("track call");
        let ControlEvent::Request {
            request, response, ..
        } = receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("operation event")
        else {
            panic!("expected operation");
        };
        response
            .send(dispatch_control(
                &mut session,
                PermissionTier::Edit,
                &request,
            ))
            .expect("dispatch answer");
        assert!(matches!(
            read_response(&mut reader).expect("edit response"),
            Some(ControlResponse::TrackAdded { .. })
        ));
        assert_eq!(session.tracks().len(), before + 1);
        assert_eq!(
            session.tracks().last().map(|track| track.name.as_str()),
            Some("Socket-created track")
        );
        session.undo();
        assert_eq!(session.tracks().len(), before);
    }

    struct AttachedApp {
        _temporary: tempfile::TempDir,
        app: App,
        stream: std::os::unix::net::UnixStream,
        responses: Receiver<ControlResponse>,
        next_id: u64,
    }

    impl AttachedApp {
        fn new() -> Self {
            use std::io::BufReader;
            use std::os::unix::net::UnixStream;

            use alp_api::control::{CONTROL_PROTOCOL_VERSION, read_response, write_request};

            let temporary = tempfile::tempdir().expect("integration tempdir");
            let (mut app, _) = App::boot();
            app.session = EditSession::open_or_demo(temporary.path().join("project"));
            app.ai_control
                .trusted
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert("full-mcp-test".into());
            let host = ControlHost::bind(
                temporary.path(),
                Arc::clone(&app.ai_control.trusted),
                app.ai_control.sender.clone(),
            )
            .expect("production attached host");
            app.ai_enabled = true;
            app.ai_control.last_publication = Some(summary_key(&app.session.project_summary()));
            let mut stream = UnixStream::connect(host.socket_path()).expect("connect MCP");
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .expect("authentication timeout");
            let mut reader = BufReader::new(stream.try_clone().expect("clone socket"));
            write_request(
                &mut stream,
                &ControlRequest::Authenticate {
                    protocol_version: CONTROL_PROTOCOL_VERSION,
                    token: host.token().into(),
                    client_id: "full-mcp-test".into(),
                },
            )
            .expect("authenticate MCP");
            assert!(matches!(
                read_response(&mut reader).expect("authenticated response"),
                Some(ControlResponse::Authenticated { .. })
            ));
            app.ai_control.host = Some(host);
            let (responses, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                while let Ok(Some(response)) = read_response(&mut reader) {
                    if responses.send(response).is_err() {
                        break;
                    }
                }
            });
            Self {
                _temporary: temporary,
                app,
                stream,
                responses: receiver,
                next_id: 1,
            }
        }

        fn call(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
            use alp_api::control::write_request;

            let id = self.next_id;
            self.next_id += 1;
            let request = alp_mcp::JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(id)),
                method: method.into(),
                params,
            };
            write_request(
                &mut self.stream,
                &ControlRequest::Mcp {
                    message: serde_json::to_value(request).expect("MCP request"),
                    limits: Default::default(),
                },
            )
            .expect("write MCP call");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                self.app.poll_ai_control();
                match self.responses.try_recv() {
                    Ok(ControlResponse::Mcp { response }) => return response,
                    Ok(other) => panic!("unexpected native response: {other:?}"),
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("MCP reader disconnected"),
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "attached MCP call timed out"
                );
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }

        fn subscribe(&self) -> std::io::BufReader<std::os::unix::net::UnixStream> {
            use std::io::BufReader;
            use std::os::unix::net::UnixStream;

            use alp_api::control::{CONTROL_PROTOCOL_VERSION, read_response, write_request};

            let host = self.app.ai_control.host.as_ref().expect("live host");
            let mut stream = UnixStream::connect(host.socket_path()).expect("subscribe connection");
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .expect("subscription timeout");
            let mut reader = BufReader::new(stream.try_clone().expect("clone subscription"));
            write_request(
                &mut stream,
                &ControlRequest::Authenticate {
                    protocol_version: CONTROL_PROTOCOL_VERSION,
                    token: host.token().into(),
                    client_id: "full-mcp-test".into(),
                },
            )
            .expect("authorize subscription");
            assert!(matches!(
                read_response(&mut reader).expect("subscription auth"),
                Some(ControlResponse::Authenticated { .. })
            ));
            write_request(&mut stream, &ControlRequest::Subscribe).expect("subscribe");
            assert!(matches!(
                read_response(&mut reader).expect("subscription response"),
                Some(ControlResponse::Subscribed)
            ));
            reader
        }
    }

    #[test]
    fn complete_mcp_catalog_edits_live_history_and_publishes_measured_changes() {
        use alp_api::control::read_response;

        let mut attached = AttachedApp::new();
        let catalog = attached.call("tools/list", serde_json::json!({}));
        let tools = catalog["result"]["tools"].as_array().expect("tool catalog");
        assert_eq!(
            tools.len(),
            alp_mcp::tools_for_tier(PermissionTier::Edit).len()
        );
        assert!(tools.len() > 4);
        assert!(tools.iter().any(|tool| tool["name"] == "media_import"));

        let mut subscription = attached.subscribe();
        let before = attached.app.session.tracks().len();
        let history_before = attached.app.session.undo_steps();
        let response = attached.call(
            "tools/call",
            serde_json::json!({
                "name": "track_add",
                "arguments": { "name": "Attached full catalog" }
            }),
        );
        assert_ne!(response["result"]["isError"], true, "{response}");
        assert_eq!(response["result"]["_meta"]["committed"], true);
        assert!(response["result"]["_meta"]["queueToCommitMs"].is_number());
        assert_eq!(attached.app.session.tracks().len(), before + 1);
        assert_eq!(attached.app.session.undo_steps(), history_before + 1);
        assert_eq!(attached.app.ai_control.mcp.commits, 1);
        let latency = attached
            .app
            .ai_control
            .mcp
            .last_commit_ms
            .expect("measured edit");
        assert!(
            latency <= 200,
            "cache-ready attached edit took {latency} ms through request, worker and commit"
        );
        let Some(ControlResponse::ProjectChanged { change }) =
            read_response(&mut subscription).expect("AI change event")
        else {
            panic!("expected committed change");
        };
        assert_eq!(change.origin, "full-mcp-test");
        assert_eq!(change.operation, "track_add");
        assert_eq!(change.queue_to_commit_ms, Some(latency));

        let history = attached.call(
            "tools/call",
            serde_json::json!({ "name": "history_list", "arguments": {} }),
        );
        assert!(
            history["result"]["structuredContent"]["items"]
                .as_array()
                .is_some_and(|items| !items.is_empty()),
            "{history}"
        );
        let undone = attached.call(
            "tools/call",
            serde_json::json!({ "name": "history_undo", "arguments": {} }),
        );
        assert_eq!(undone["result"]["structuredContent"]["undone"], true);
        assert_eq!(attached.app.session.tracks().len(), before);
        let redone = attached.call(
            "tools/call",
            serde_json::json!({ "name": "history_redo", "arguments": {} }),
        );
        assert_eq!(redone["result"]["structuredContent"]["redone"], true);
        assert_eq!(attached.app.session.tracks().len(), before + 1);
        for operation in ["history_undo", "history_redo"] {
            let Some(ControlResponse::ProjectChanged { change }) =
                read_response(&mut subscription).expect("shared-history event")
            else {
                panic!("expected history change");
            };
            assert_eq!(change.operation, operation);
        }

        let operator_before = attached.app.session.tracks().len();
        attached
            .app
            .session
            .add_track("Operator track")
            .expect("operator edit");
        attached.app.poll_ai_control();
        let Some(ControlResponse::ProjectChanged { change }) =
            read_response(&mut subscription).expect("operator change event")
        else {
            panic!("expected operator change");
        };
        assert_eq!(change.origin, "operator");
        assert_eq!(change.summary.track_count, operator_before + 1);
        assert_eq!(change.queue_to_commit_ms, None);
    }

    #[test]
    fn a_read_started_before_a_human_edit_is_not_returned_as_current() {
        let mut attached = AttachedApp::new();
        let host_id = attached.app.ai_control.host.as_ref().expect("host").id();
        let state = attached
            .app
            .session
            .attached_project_state()
            .expect("saved project");
        let base_revision = state.revision;
        let track_count = state.project.tracks.len();
        let (response, receive) = mpsc::sync_channel(1);
        attached
            .app
            .ai_control
            .mcp
            .dispatch(AttachedCall {
                host_id,
                client_id: "full-mcp-test".into(),
                request_value: serde_json::json!({
                    "jsonrpc": "2.0", "id": "stale-read", "method": "tools/call",
                    "params": { "name": "track_list", "arguments": {} }
                }),
                limits: Default::default(),
                state,
                summary: attached.app.session.project_summary(),
                tier: PermissionTier::Edit,
                response,
            })
            .expect("dispatch worker");
        attached
            .app
            .session
            .add_track("Human edit")
            .expect("human edit");
        let response = {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                attached.app.poll_ai_control();
                match receive.try_recv() {
                    Ok(response) => break response,
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("worker response disconnected"),
                }
                assert!(std::time::Instant::now() < deadline, "stale read timed out");
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        };
        let ControlResponse::Mcp { response } = response else {
            panic!("expected MCP response");
        };
        assert_eq!(response["result"]["isError"], true, "{response}");
        assert!(
            response["result"]["content"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("revision conflict"))
        );
        let meta = &response["result"]["_meta"];
        assert_eq!(meta["snapshotRevision"], base_revision, "{response}");
        assert_eq!(meta["liveRevision"], base_revision + 1, "{response}");
        assert_eq!(attached.app.session.tracks().len(), track_count + 1);
    }

    #[test]
    fn a_worker_completion_cannot_overwrite_a_concurrent_operator_edit() {
        let mut attached = AttachedApp::new();
        let host_id = attached.app.ai_control.host.as_ref().expect("host").id();
        let state = attached
            .app
            .session
            .attached_project_state()
            .expect("saved project");
        let base_revision = state.revision;
        let track_count = state.project.tracks.len();
        let (response, receive) = mpsc::sync_channel(1);
        let request = alp_mcp::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!("stale-worker")),
            method: "tools/call".into(),
            params: serde_json::json!({
                "name": "track_add",
                "arguments": { "name": "Stale worker" }
            }),
        };
        attached
            .app
            .ai_control
            .mcp
            .dispatch(AttachedCall {
                host_id,
                client_id: "full-mcp-test".into(),
                request_value: serde_json::to_value(request).expect("request"),
                limits: Default::default(),
                state,
                summary: attached.app.session.project_summary(),
                tier: PermissionTier::Edit,
                response,
            })
            .expect("dispatch worker");
        attached
            .app
            .session
            .add_track("Human wins")
            .expect("human edit");
        let response = {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                attached.app.poll_ai_control();
                match receive.try_recv() {
                    Ok(response) => break response,
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("worker response disconnected"),
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "stale completion timed out"
                );
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        };
        let ControlResponse::Mcp { response } = response else {
            panic!("expected MCP response");
        };
        assert_eq!(response["result"]["isError"], true, "{response}");
        assert!(
            response["result"]["content"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("revision conflict"))
        );
        assert_eq!(attached.app.session.tracks().len(), track_count + 1);
        assert!(
            attached
                .app
                .session
                .tracks()
                .iter()
                .all(|track| track.name != "Stale worker")
        );
        assert_eq!(
            attached
                .app
                .session
                .tracks()
                .last()
                .map(|track| track.name.as_str()),
            Some("Human wins")
        );
        assert_eq!(attached.app.session.revision(), base_revision + 1);
    }

    #[test]
    fn revocation_discards_a_late_worker_edit_without_touching_live_history() {
        let mut attached = AttachedApp::new();
        let host_id = attached.app.ai_control.host.as_ref().expect("host").id();
        let state = attached
            .app
            .session
            .attached_project_state()
            .expect("saved project");
        let before = attached.app.session.project_summary();
        let (response, receive) = mpsc::sync_channel(1);
        attached
            .app
            .ai_control
            .mcp
            .dispatch(AttachedCall {
                host_id,
                client_id: "full-mcp-test".into(),
                request_value: serde_json::json!({
                    "jsonrpc": "2.0", "id": "late-edit", "method": "tools/call",
                    "params": {
                        "name": "track_add",
                        "arguments": { "name": "Must not commit" }
                    }
                }),
                limits: Default::default(),
                state,
                summary: before.clone(),
                tier: PermissionTier::Edit,
                response,
            })
            .expect("dispatch worker");
        attached.app.stop_ai_control("test revocation");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let response = loop {
            attached.app.poll_ai_control();
            match receive.try_recv() {
                Ok(response) => break response,
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => panic!("worker response disconnected"),
            }
            assert!(std::time::Instant::now() < deadline, "revocation timed out");
            std::thread::sleep(std::time::Duration::from_millis(2));
        };
        assert!(matches!(
            response,
            ControlResponse::Error { ref code, .. } if code == "session_revoked"
        ));
        let after = attached.app.session.project_summary();
        assert_eq!(after.id, before.id);
        assert_eq!(after.revision, before.revision);
        assert_eq!(after.track_count, before.track_count);
        assert_eq!(after.undo_steps, before.undo_steps);
    }

    #[test]
    fn attached_media_import_reconciles_audio_needed_by_live_undo_and_redo() {
        let mut attached = AttachedApp::new();
        let state = attached
            .app
            .session
            .attached_project_state()
            .expect("saved project");
        let existing = state.project.sources.first().expect("demo media");
        let input = state
            .paths
            .resolve_relative(&existing.relative_path)
            .expect("existing demo source");
        let before = attached.app.session.project().sources.len();
        let history_before = attached.app.session.undo_steps();
        let response = attached.call(
            "tools/call",
            serde_json::json!({
                "name": "media_import",
                "arguments": {
                    "path": input,
                    "name": "Attached source cache check",
                    "expected_revision": state.revision
                }
            }),
        );
        assert_ne!(response["result"]["isError"], true, "{response}");
        assert_eq!(attached.app.session.project().sources.len(), before + 1);
        assert_eq!(attached.app.session.undo_steps(), history_before + 1);
        attached
            .app
            .session
            .scene()
            .expect("all attached sources have decoded audio");
        attached.app.session.undo();
        assert_eq!(attached.app.session.project().sources.len(), before);
        attached
            .app
            .session
            .scene()
            .expect("undo scene remains coherent");
        attached.app.session.redo();
        assert_eq!(attached.app.session.project().sources.len(), before + 1);
        attached
            .app
            .session
            .scene()
            .expect("redo retains imported source audio");
    }
}
