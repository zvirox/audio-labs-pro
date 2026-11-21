//! Owner-only live control transport for the desktop application.
//!
//! The host owns only transport and admission: it never opens a project or
//! edits one on a socket thread. Every approved call is sent to the desktop's
//! event queue and answered there, so human and AI edits use the same command
//! authority. This is a small native attach protocol, not a second MCP server.

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use nix::unistd::Uid;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::types::{PermissionTier, ProjectSummary, TrackSummary};

/// Version of the newline-delimited JSON control protocol.
pub const CONTROL_PROTOCOL_VERSION: u32 = 2;
/// Maximum serialized request or response size. The native envelope is larger
/// than the MCP line limit so wrapping cannot reject an otherwise valid call.
pub const MAX_CONTROL_MESSAGE_BYTES: usize = 2 * 1024 * 1024;
/// Maximum committed changes buffered for a slow subscriber before it is dropped.
const CHANGE_FEED_QUEUE: usize = 64;
/// Bounds socket writes so a stalled subscriber cannot block revocation.
const SOCKET_WRITE_TIMEOUT: Duration = Duration::from_millis(500);
static NEXT_HOST_ID: AtomicU64 = AtomicU64::new(1);

/// Per-client MCP limits, replicated at the UI dispatcher boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlLimits {
    /// Largest JSON-RPC input line accepted by the attach proxy.
    pub max_payload_bytes: usize,
    /// Advisory tool-call budget in milliseconds.
    pub call_timeout_ms: u64,
    /// Calls admitted per rate window.
    pub requests_per_window: u32,
    /// Rate window in seconds.
    pub rate_window_secs: u64,
}

impl Default for ControlLimits {
    fn default() -> Self {
        Self {
            max_payload_bytes: 1024 * 1024,
            call_timeout_ms: 120_000,
            requests_per_window: 600,
            rate_window_secs: 60,
        }
    }
}

impl ControlLimits {
    /// Convert to the `alp-mcp` boundary type without introducing a dependency
    /// from the transport crate back into the MCP crate.
    #[must_use]
    pub fn durations(self) -> (std::time::Duration, std::time::Duration) {
        (
            std::time::Duration::from_millis(self.call_timeout_ms),
            std::time::Duration::from_secs(self.rate_window_secs.max(1)),
        )
    }
}

/// One authenticated call, or the initial per-connection authentication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlRequest {
    /// Authenticate and identify a client. The token is valid for one UI host.
    Authenticate {
        /// Must equal [`CONTROL_PROTOCOL_VERSION`].
        protocol_version: u32,
        /// Random token displayed by the GUI, never persisted.
        token: String,
        /// Stable, human-readable client identity used for approval.
        client_id: String,
    },
    /// Read the live project summary.
    ProjectInfo,
    /// Read the live track list.
    TrackList,
    /// Append an undoable track.
    TrackAdd {
        /// Display name for the new track.
        name: String,
        /// Refuse the write if a human or another client changed the project.
        expected_revision: Option<u64>,
    },
    /// Split a clip at an absolute timeline position.
    ClipSplit {
        /// Existing clip UUID.
        clip_id: String,
        /// Absolute timeline position in seconds.
        split_secs: f64,
        /// Refuse the write if a human or another client changed the project.
        expected_revision: Option<u64>,
    },
    /// One JSON-RPC message for the shared MCP dispatcher.
    Mcp {
        /// Complete JSON-RPC request. The UI enforces the same tier and catalog
        /// as headless `alp-mcp`; this transport does not interpret tools.
        message: serde_json::Value,
        /// Boundary limits advertised by this attached client.
        limits: ControlLimits,
    },
    /// Receive a nonblocking stream of committed live-project changes.
    Subscribe,
}

impl ControlRequest {
    /// Stable operation name for the activity log.
    #[must_use]
    pub fn operation_name(&self) -> &'static str {
        match self {
            Self::Authenticate { .. } => "authenticate",
            Self::ProjectInfo => "project_info",
            Self::TrackList => "track_list",
            Self::TrackAdd { .. } => "track_add",
            Self::ClipSplit { .. } => "clip_split",
            Self::Mcp { .. } => "mcp",
            Self::Subscribe => "subscribe",
        }
    }

    /// Least privilege required by the operation.
    #[must_use]
    pub fn required_tier(&self) -> PermissionTier {
        match self {
            Self::ProjectInfo | Self::TrackList | Self::Subscribe => PermissionTier::Observe,
            Self::Mcp { .. } => PermissionTier::Observe,
            Self::Authenticate { .. } | Self::TrackAdd { .. } | Self::ClipSplit { .. } => {
                PermissionTier::Edit
            }
        }
    }

    /// Expected revision carried by a write, if any.
    #[must_use]
    pub fn expected_revision(&self) -> Option<u64> {
        match self {
            Self::TrackAdd {
                expected_revision, ..
            }
            | Self::ClipSplit {
                expected_revision, ..
            } => *expected_revision,
            Self::Authenticate { .. }
            | Self::ProjectInfo
            | Self::TrackList
            | Self::Mcp { .. }
            | Self::Subscribe => None,
        }
    }
}

/// One committed project revision, suitable for MCP resource/update feeds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectChange {
    /// Summary captured after the live edit was committed.
    pub summary: ProjectSummary,
    /// `operator` for a GUI edit or the approved attached client ID.
    pub origin: String,
    /// Human-readable operation or logical MCP tool name.
    pub operation: String,
    /// Unix timestamp of the UI commit.
    pub committed_at_ms: u64,
    /// Measured queue-to-live-view-model latency for attached calls, when known.
    pub queue_to_commit_ms: Option<u64>,
}

/// Result or admission response sent to a control client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlResponse {
    /// The first new client must wait for a human decision.
    ApprovalRequired {
        /// Sanitized identity supplied by the client.
        client_id: String,
    },
    /// Authentication and approval succeeded.
    Authenticated {
        /// Approved client identity.
        client_id: String,
    },
    /// Live project state.
    ProjectInfo {
        /// Summary from the GUI's current session.
        summary: Box<ProjectSummary>,
    },
    /// Live timeline tracks.
    TrackList {
        /// Tracks in timeline order.
        tracks: Vec<TrackSummary>,
    },
    /// An undoable track command completed.
    TrackAdded {
        /// New track summary.
        track: Box<TrackSummary>,
        /// Revision after the edit.
        revision: u64,
    },
    /// An undoable split command completed.
    ClipSplit {
        /// Revision after the edit.
        revision: u64,
    },
    /// A JSON-RPC response produced by the shared MCP dispatcher.
    Mcp {
        /// Correlated JSON-RPC response envelope.
        response: serde_json::Value,
    },
    /// This connection will receive committed changes until it closes or the
    /// operator stops the control host.
    Subscribed,
    /// A live GUI or attached edit has become visible in the view model.
    ProjectChanged {
        /// Committed revision and origin metadata.
        change: ProjectChange,
    },
    /// A refusal that the client can act on.
    Error {
        /// Stable machine-readable category.
        code: String,
        /// Human-readable explanation; credentials are never echoed.
        message: String,
    },
}

impl ControlResponse {
    /// Construct an error response without exposing socket internals.
    #[must_use]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Human answer to a first-connection approval prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// Reject this connection.
    Deny,
    /// Trust this client ID until the control host is stopped.
    Session,
    /// Trust this client ID across restarts; a fresh token is still required.
    Always,
}

/// Work handed from a socket thread to the UI event queue.
#[derive(Debug, Clone)]
pub enum ControlEvent {
    /// A new client authenticated and now needs human approval.
    ApprovalRequest {
        /// Host generation, used to reject work queued before revocation.
        host_id: u64,
        /// Sanitized client identity.
        client_id: String,
        /// One-shot answer channel.
        response: SyncSender<ApprovalDecision>,
    },
    /// An authenticated operation awaiting execution on the UI thread.
    Request {
        /// Host generation, used to reject work queued before revocation.
        host_id: u64,
        /// Approved client identity for the activity log.
        client_id: String,
        /// Typed operation.
        request: ControlRequest,
        /// One-shot result channel.
        response: SyncSender<ControlResponse>,
    },
}

#[derive(Default)]
struct ChangeFeed {
    subscribers: Mutex<Vec<SyncSender<ControlResponse>>>,
}

impl ChangeFeed {
    fn publish(&self, change: ProjectChange) {
        let mut subscribers = lock_recover(&self.subscribers);
        subscribers.retain(|subscriber| {
            matches!(
                subscriber.try_send(ControlResponse::ProjectChanged {
                    change: change.clone()
                }),
                Ok(())
            )
        });
    }
}

/// A running owner-only socket host.
///
/// Dropping or stopping the host revokes every accepted client. No listener or
/// worker thread is joined on the UI thread; workers observe the revocation at
/// their next bounded wait.
pub struct ControlHost {
    id: u64,
    socket_path: PathBuf,
    token: String,
    shutdown: Arc<AtomicBool>,
    changes: Arc<ChangeFeed>,
}

impl ControlHost {
    /// Bind `$XDG_RUNTIME_DIR/alp/control.sock` below the supplied runtime root.
    pub fn bind(
        runtime_dir: &Path,
        always_clients: Arc<Mutex<BTreeSet<String>>>,
        events: Sender<ControlEvent>,
    ) -> io::Result<Self> {
        let socket_path = default_socket_path(runtime_dir);
        let parent = socket_path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "control socket has no parent")
        })?;
        prepare_runtime_directory(parent)?;
        prepare_socket_path(&socket_path)?;

        let listener = UnixListener::bind(&socket_path)?;
        fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;

        let changes = Arc::new(ChangeFeed::default());
        let host = Self {
            id: NEXT_HOST_ID.fetch_add(1, Ordering::Relaxed),
            socket_path,
            token: format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4()),
            shutdown: Arc::new(AtomicBool::new(false)),
            changes: Arc::clone(&changes),
        };
        let config = Arc::new(HostConfig {
            id: host.id,
            token: host.token.clone(),
            shutdown: Arc::clone(&host.shutdown),
            always_clients,
            session_clients: Mutex::new(BTreeSet::new()),
            events,
            changes,
        });
        thread::Builder::new()
            .name("alp-control-listener".into())
            .spawn(move || accept_loop(listener, config))?;
        Ok(host)
    }

    /// Opaque generation used internally to reject stale queued work.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Socket path clients should connect to.
    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Fresh authentication token for this enabled control session.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Publish an already committed edit without waiting on subscriber I/O.
    /// Slow subscribers are dropped when their bounded queue fills or a write
    /// times out, so publishing can never block the UI or grow without bound.
    pub fn publish(&self, change: ProjectChange) {
        self.changes.publish(change);
    }

    /// Revoke new and existing connections and unlink the socket.
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        remove_owned_socket(&self.socket_path);
    }
}

impl Drop for ControlHost {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Conventional control socket path below an XDG runtime directory.
#[must_use]
pub fn default_socket_path(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join("alp").join("control.sock")
}

/// Return the kernel-authenticated UID of a connected peer.
pub fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    getsockopt(stream, PeerCredentials)
        .map(|credentials| credentials.uid())
        .map_err(io::Error::other)
}

/// Serialize one request for a native attach client.
pub fn write_request(stream: &mut UnixStream, request: &ControlRequest) -> io::Result<()> {
    write_json(stream, request)
}

/// Read one response for a native attach client.
pub fn read_response(reader: &mut BufReader<UnixStream>) -> io::Result<Option<ControlResponse>> {
    read_response_buffered(reader, &mut Vec::new())
}

/// Read one response while retaining a partial frame across timeouts.
///
/// Subscription clients set a socket read timeout so shutdown is responsive.
/// Without a caller-owned buffer, the bytes consumed before a timeout would be
/// lost and the next frame would be parsed from its suffix.
pub fn read_response_buffered(
    reader: &mut BufReader<UnixStream>,
    frame: &mut Vec<u8>,
) -> io::Result<Option<ControlResponse>> {
    read_json_buffered(reader, frame)
}

struct HostConfig {
    id: u64,
    token: String,
    shutdown: Arc<AtomicBool>,
    always_clients: Arc<Mutex<BTreeSet<String>>>,
    session_clients: Mutex<BTreeSet<String>>,
    events: Sender<ControlEvent>,
    changes: Arc<ChangeFeed>,
}

impl HostConfig {
    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    fn known_client(&self, client_id: &str) -> bool {
        lock_recover(&self.always_clients).contains(client_id)
            || lock_recover(&self.session_clients).contains(client_id)
    }
}

fn accept_loop(listener: UnixListener, config: Arc<HostConfig>) {
    while !config.is_shutdown() {
        match listener.accept() {
            Ok((stream, _address)) => {
                let config = Arc::clone(&config);
                let spawned = thread::Builder::new()
                    .name("alp-control-client".into())
                    .spawn(move || serve_client(stream, config));
                if spawned.is_err() {
                    // Dropping the just-accepted stream is a safe refusal.
                    continue;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(25));
            }
            Err(_) if config.is_shutdown() => break,
            Err(_) => thread::sleep(Duration::from_millis(25)),
        }
    }
}

fn serve_client(mut stream: UnixStream, config: Arc<HostConfig>) {
    if stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .and_then(|_| stream.set_write_timeout(Some(SOCKET_WRITE_TIMEOUT)))
        .is_err()
    {
        return;
    }
    match peer_uid(&stream) {
        Ok(uid) if is_owner_uid(uid) => {}
        _ => {
            let _ = write_response(
                &mut stream,
                &ControlResponse::error("peer_rejected", "control peer UID does not match the UI"),
            );
            return;
        }
    }

    let mut reader = match stream.try_clone() {
        Ok(clone) => BufReader::new(clone),
        Err(_) => return,
    };
    let Some(request) = read_request_or_refuse(&mut reader, &mut stream) else {
        return;
    };
    let ControlRequest::Authenticate {
        protocol_version,
        token,
        client_id,
    } = request
    else {
        let _ = write_response(
            &mut stream,
            &ControlResponse::error("authentication_required", "authenticate first"),
        );
        return;
    };
    if protocol_version != CONTROL_PROTOCOL_VERSION {
        let _ = write_response(
            &mut stream,
            &ControlResponse::error(
                "protocol_mismatch",
                format!("control protocol version must be {CONTROL_PROTOCOL_VERSION}"),
            ),
        );
        return;
    }
    if !constant_time_eq(config.token.as_bytes(), token.as_bytes()) {
        let _ = write_response(
            &mut stream,
            &ControlResponse::error("authentication_failed", "invalid control token"),
        );
        return;
    }
    if !valid_client_id(&client_id) {
        let _ = write_response(
            &mut stream,
            &ControlResponse::error(
                "invalid_client",
                "client_id must be 1-64 ASCII letters, digits, '.', '_' or '-'",
            ),
        );
        return;
    }

    if !config.known_client(&client_id) {
        if write_response(
            &mut stream,
            &ControlResponse::ApprovalRequired {
                client_id: client_id.clone(),
            },
        )
        .is_err()
        {
            return;
        }
        let (response, answer) = mpsc::sync_channel(1);
        if config
            .events
            .send(ControlEvent::ApprovalRequest {
                host_id: config.id,
                client_id: client_id.clone(),
                response,
            })
            .is_err()
        {
            return;
        }
        let decision = wait_for_answer(&answer, &config);
        match decision {
            Some(ApprovalDecision::Session) => {
                lock_recover(&config.session_clients).insert(client_id.clone());
            }
            Some(ApprovalDecision::Always) => {
                lock_recover(&config.always_clients).insert(client_id.clone());
                lock_recover(&config.session_clients).insert(client_id.clone());
            }
            Some(ApprovalDecision::Deny) => {
                let _ = write_response(
                    &mut stream,
                    &ControlResponse::error("approval_denied", "the operator denied this client"),
                );
                return;
            }
            None => {
                let (code, message) = if config.is_shutdown() {
                    ("session_revoked", "the operator stopped AI control")
                } else {
                    (
                        "approval_denied",
                        "the approval request could not be completed",
                    )
                };
                let _ = write_response(&mut stream, &ControlResponse::error(code, message));
                return;
            }
        }
    }

    if write_response(
        &mut stream,
        &ControlResponse::Authenticated {
            client_id: client_id.clone(),
        },
    )
    .is_err()
    {
        return;
    }

    let mut frame = Vec::new();
    while !config.is_shutdown() {
        let request = match read_json_buffered::<ControlRequest>(&mut reader, &mut frame) {
            Ok(Some(request)) => request,
            Ok(None) => break,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(_) => break,
        };
        if matches!(request, ControlRequest::Authenticate { .. }) {
            let _ = write_response(
                &mut stream,
                &ControlResponse::error("already_authenticated", "connection is authenticated"),
            );
            continue;
        }
        if matches!(request, ControlRequest::Subscribe) {
            serve_change_subscription(stream, &config);
            break;
        }
        let (response, answer) = mpsc::sync_channel(1);
        if config
            .events
            .send(ControlEvent::Request {
                host_id: config.id,
                client_id: client_id.clone(),
                request,
                response,
            })
            .is_err()
        {
            break;
        }
        let Some(result) = wait_for_answer(&answer, &config) else {
            let _ = write_response(
                &mut stream,
                &ControlResponse::error("session_revoked", "the operator stopped AI control"),
            );
            break;
        };
        if write_response(&mut stream, &result).is_err() {
            break;
        }
    }
}

fn serve_change_subscription(mut stream: UnixStream, config: &HostConfig) {
    let (sender, receiver) = mpsc::sync_channel(CHANGE_FEED_QUEUE);
    lock_recover(&config.changes.subscribers).push(sender);
    if write_response(&mut stream, &ControlResponse::Subscribed).is_err() {
        return;
    }
    while !config.is_shutdown() {
        match receiver.recv_timeout(Duration::from_millis(250)) {
            Ok(response) => {
                if write_response(&mut stream, &response).is_err() {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn read_request_or_refuse(
    reader: &mut BufReader<UnixStream>,
    stream: &mut UnixStream,
) -> Option<ControlRequest> {
    match read_json(reader) {
        Ok(Some(request)) => Some(request),
        Ok(None) => None,
        Err(error) => {
            let _ = write_response(
                stream,
                &ControlResponse::error("bad_request", error.to_string()),
            );
            None
        }
    }
}

fn wait_for_answer<T>(receiver: &Receiver<T>, config: &HostConfig) -> Option<T> {
    loop {
        if config.is_shutdown() {
            return None;
        }
        match receiver.recv_timeout(Duration::from_millis(25)) {
            Ok(answer) => return Some(answer),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return None,
        }
    }
}

fn prepare_runtime_directory(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("refusing non-directory control path {}", path.display()),
        ));
    }
    if metadata.uid() != Uid::effective().as_raw() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "refusing control directory owned by another user: {}",
                path.display()
            ),
        ));
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

fn prepare_socket_path(path: &Path) -> io::Result<()> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    if metadata.uid() != Uid::effective().as_raw() || !metadata.file_type().is_socket() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "refusing to replace non-socket or foreign path {}",
                path.display()
            ),
        ));
    }
    match UnixStream::connect(path) {
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            format!("another UI control host is listening at {}", path.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => fs::remove_file(path),
        Err(error) => Err(error),
    }
}

fn remove_owned_socket(path: &Path) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if metadata.file_type().is_socket() && metadata.uid() == Uid::effective().as_raw() {
        let _ = fs::remove_file(path);
    }
}

fn is_owner_uid(uid: u32) -> bool {
    uid == Uid::effective().as_raw()
}

fn valid_client_id(client_id: &str) -> bool {
    !client_id.is_empty()
        && client_id.len() <= 64
        && client_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn constant_time_eq(expected: &[u8], supplied: &[u8]) -> bool {
    let mut difference = expected.len() ^ supplied.len();
    let longest = expected.len().max(supplied.len());
    for index in 0..longest {
        let left = expected.get(index).copied().unwrap_or(0);
        let right = supplied.get(index).copied().unwrap_or(0);
        difference |= usize::from(left ^ right);
    }
    difference == 0
}

fn lock_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn write_response(stream: &mut UnixStream, response: &ControlResponse) -> io::Result<()> {
    match write_json(stream, response) {
        Err(error) if error.kind() == io::ErrorKind::InvalidData => write_json(
            stream,
            &ControlResponse::error(
                "response_too_large",
                "control response exceeds the 2 MiB frame cap; use pagination or narrower arguments",
            ),
        ),
        result => result,
    }
}

fn write_json<T: Serialize>(stream: &mut UnixStream, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    if bytes.len() > MAX_CONTROL_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "control response exceeds the payload limit",
        ));
    }
    stream.write_all(&bytes)?;
    stream.write_all(b"\n")?;
    stream.flush()
}

fn read_json<T: DeserializeOwned>(reader: &mut BufReader<UnixStream>) -> io::Result<Option<T>> {
    read_json_buffered(reader, &mut Vec::new())
}

fn read_json_buffered<T: DeserializeOwned>(
    reader: &mut BufReader<UnixStream>,
    frame: &mut Vec<u8>,
) -> io::Result<Option<T>> {
    if read_frame(reader, frame)?.is_none() {
        return Ok(None);
    }
    let bytes = std::mem::take(frame);
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// Read into a caller-owned frame so a socket timeout cannot discard a partial
/// message. Authenticated clients can therefore idle or fragment their writes.
fn read_frame(reader: &mut BufReader<UnixStream>, frame: &mut Vec<u8>) -> io::Result<Option<()>> {
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "unterminated control message",
                ))
            };
        }
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        let has_delimiter = available.get(consumed.saturating_sub(1)) == Some(&b'\n');
        let payload_bytes = frame.len() + consumed - usize::from(has_delimiter);
        if payload_bytes > MAX_CONTROL_MESSAGE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "control message exceeds the {MAX_CONTROL_MESSAGE_BYTES}-byte payload limit"
                ),
            ));
        }
        frame.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        if frame.last() == Some(&b'\n') {
            frame.pop();
            return Ok(Some(()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(
        runtime: &Path,
    ) -> (
        ControlHost,
        Arc<Mutex<BTreeSet<String>>>,
        Receiver<ControlEvent>,
    ) {
        let trusted = Arc::new(Mutex::new(BTreeSet::new()));
        let (events, receiver) = mpsc::channel();
        let host = ControlHost::bind(runtime, Arc::clone(&trusted), events).expect("bind host");
        (host, trusted, receiver)
    }

    fn authenticate(host: &ControlHost, client_id: &str) -> (UnixStream, BufReader<UnixStream>) {
        let mut stream = UnixStream::connect(host.socket_path()).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout");
        let reader = BufReader::new(stream.try_clone().expect("clone"));
        write_request(
            &mut stream,
            &ControlRequest::Authenticate {
                protocol_version: CONTROL_PROTOCOL_VERSION,
                token: host.token().into(),
                client_id: client_id.into(),
            },
        )
        .expect("authenticate");
        (stream, reader)
    }

    fn next(reader: &mut BufReader<UnixStream>) -> ControlResponse {
        read_response(reader)
            .expect("response")
            .expect("connection remains open")
    }

    #[test]
    fn response_timeouts_preserve_fragmented_frames() {
        let (mut writer, reader) = UnixStream::pair().expect("socket pair");
        reader
            .set_read_timeout(Some(Duration::from_millis(40)))
            .expect("short read timeout");
        let mut encoded =
            serde_json::to_vec(&ControlResponse::Subscribed).expect("encode response");
        encoded.push(b'\n');
        let split = encoded.len() / 2;
        let first_fragment = encoded[..split].to_vec();
        writer
            .write_all(&first_fragment)
            .expect("write first fragment");
        writer.flush().expect("flush first fragment");
        let mut writer_thread = writer.try_clone().expect("clone writer");
        let writing = thread::spawn(move || {
            thread::sleep(Duration::from_millis(150));
            writer_thread
                .write_all(&encoded[split..])
                .expect("write second fragment");
            writer_thread.flush().expect("flush second fragment");
        });

        let mut reader = BufReader::new(reader);
        let mut frame = Vec::new();
        let timeout = read_response_buffered(&mut reader, &mut frame).expect_err("timeout");
        assert!(
            matches!(
                timeout.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ),
            "{timeout}"
        );
        assert!(
            frame.starts_with(&first_fragment),
            "partial response bytes were discarded"
        );
        let mut complete = None;
        for _ in 0..10 {
            match read_response_buffered(&mut reader, &mut frame) {
                Ok(Some(response)) => {
                    complete = Some(response);
                    break;
                }
                Ok(None) => panic!("subscription closed"),
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) => {}
                Err(error) => panic!("unexpected read error: {error}"),
            }
        }
        assert!(
            matches!(complete, Some(ControlResponse::Subscribed)),
            "complete={complete:?}, frame={frame:?}"
        );
        writing.join().expect("fragment writer");
    }

    fn test_change(revision: u64) -> ProjectChange {
        ProjectChange {
            summary: ProjectSummary {
                id: "project".into(),
                name: "Project".into(),
                revision,
                sample_rate: 48_000,
                duration_secs: 1.0,
                track_count: 1,
                clip_count: 1,
                undo_steps: 1,
                redo_steps: 0,
                is_dirty: true,
            },
            origin: "operator".into(),
            operation: "track_add".into(),
            committed_at_ms: revision,
            queue_to_commit_ms: None,
        }
    }

    #[test]
    fn a_slow_subscriber_is_dropped_when_its_bounded_queue_fills() {
        let feed = ChangeFeed::default();
        let (sender, receiver) = mpsc::sync_channel(1);
        lock_recover(&feed.subscribers).push(sender);

        feed.publish(test_change(1));
        feed.publish(test_change(2));

        assert!(lock_recover(&feed.subscribers).is_empty());
        assert!(matches!(
            receiver.try_recv(),
            Ok(ControlResponse::ProjectChanged { change })
                if change.summary.revision == 1
        ));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn exact_frame_boundary_is_readable_and_oversize_writes_are_rejected() {
        let (mut writer, reader) = UnixStream::pair().expect("socket pair");
        let exact = serde_json::Value::String("x".repeat(MAX_CONTROL_MESSAGE_BYTES - 2));
        let writing = thread::spawn(move || write_json(&mut writer, &exact));
        let mut reader = BufReader::new(reader);
        let decoded: serde_json::Value = read_json(&mut reader)
            .expect("exact frame must be readable")
            .expect("one frame");
        assert_eq!(
            decoded.as_str().map(str::len),
            Some(MAX_CONTROL_MESSAGE_BYTES - 2)
        );
        writing.join().expect("writer thread").expect("exact write");

        let (mut writer, _reader) = UnixStream::pair().expect("socket pair");
        let oversized = serde_json::Value::String("x".repeat(MAX_CONTROL_MESSAGE_BYTES - 1));
        let error = write_json(&mut writer, &oversized).expect_err("oversize must fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn socket_is_owner_only_and_kernel_peer_credentials_match() {
        let runtime = tempfile::tempdir().expect("tempdir");
        let (host, _trusted, _events) = host(runtime.path());
        let mode = fs::metadata(host.socket_path())
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(host.token().len(), 72);

        let (left, right) = UnixStream::pair().expect("pair");
        assert_eq!(peer_uid(&left).expect("peer"), Uid::effective().as_raw());
        assert_eq!(peer_uid(&right).expect("peer"), Uid::effective().as_raw());
        let owner = Uid::effective().as_raw();
        let foreign = if owner == u32::MAX {
            owner - 1
        } else {
            owner + 1
        };
        assert!(is_owner_uid(owner));
        assert!(!is_owner_uid(foreign));
    }

    #[test]
    fn bad_token_is_refused_without_creating_ui_work() {
        let runtime = tempfile::tempdir().expect("tempdir");
        let (host, _trusted, events) = host(runtime.path());
        let mut stream = UnixStream::connect(host.socket_path()).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        write_request(
            &mut stream,
            &ControlRequest::Authenticate {
                protocol_version: CONTROL_PROTOCOL_VERSION,
                token: "wrong-token".into(),
                client_id: "test-client".into(),
            },
        )
        .expect("write");
        assert!(matches!(
            next(&mut reader),
            ControlResponse::Error { ref code, .. } if code == "authentication_failed"
        ));
        assert!(
            events.try_recv().is_err(),
            "unauthenticated work reached the UI"
        );
    }

    #[test]
    fn approval_and_call_round_trip_through_the_ui_queue() {
        let runtime = tempfile::tempdir().expect("tempdir");
        let (host, _trusted, events) = host(runtime.path());
        let (mut stream, mut reader) = authenticate(&host, "test-client");
        assert!(matches!(
            next(&mut reader),
            ControlResponse::ApprovalRequired { .. }
        ));
        let event = events
            .recv_timeout(Duration::from_secs(2))
            .expect("approval");
        let ControlEvent::ApprovalRequest {
            client_id,
            response,
            ..
        } = event
        else {
            panic!("expected approval request");
        };
        assert_eq!(client_id, "test-client");
        response.send(ApprovalDecision::Session).expect("approve");
        assert!(matches!(
            next(&mut reader),
            ControlResponse::Authenticated { .. }
        ));

        // Idle timeouts do not revoke a healthy authenticated connection, and
        // a timeout between fragments does not discard the partial frame.
        thread::sleep(Duration::from_millis(300));
        let mut call = serde_json::to_vec(&ControlRequest::TrackList).expect("serialize call");
        call.push(b'\n');
        let split = call.len() / 2;
        stream.write_all(&call[..split]).expect("first fragment");
        stream.flush().expect("flush first fragment");
        thread::sleep(Duration::from_millis(300));
        stream.write_all(&call[split..]).expect("second fragment");
        stream.flush().expect("flush second fragment");
        let event = events
            .recv_timeout(Duration::from_secs(2))
            .expect("request");
        let ControlEvent::Request {
            client_id,
            request,
            response,
            ..
        } = event
        else {
            panic!("expected operation");
        };
        assert_eq!(client_id, "test-client");
        assert_eq!(request.required_tier(), PermissionTier::Observe);
        response
            .send(ControlResponse::TrackList { tracks: Vec::new() })
            .expect("answer");
        assert!(matches!(
            next(&mut reader),
            ControlResponse::TrackList { .. }
        ));

        // Session approval survives a reconnect but still requires the token.
        let (_second, mut second_reader) = authenticate(&host, "test-client");
        assert!(matches!(
            next(&mut second_reader),
            ControlResponse::Authenticated { .. }
        ));
    }

    #[test]
    fn stop_revokes_a_client_and_removes_the_socket() {
        let runtime = tempfile::tempdir().expect("tempdir");
        let (host, _trusted, events) = host(runtime.path());
        let (_stream, mut reader) = authenticate(&host, "test-client");
        assert!(matches!(
            next(&mut reader),
            ControlResponse::ApprovalRequired { .. }
        ));
        let ControlEvent::ApprovalRequest { response, .. } = events
            .recv_timeout(Duration::from_secs(2))
            .expect("approval")
        else {
            panic!("expected approval");
        };
        host.stop();
        drop(response);
        assert!(!host.socket_path().exists());
        assert!(matches!(
            next(&mut reader),
            ControlResponse::Error { ref code, .. } if code == "session_revoked"
        ));
    }

    #[test]
    fn always_approval_survives_restart_but_the_token_rotates() {
        let runtime = tempfile::tempdir().expect("tempdir");
        let trusted = Arc::new(Mutex::new(BTreeSet::new()));
        lock_recover(&trusted).insert("trusted-client".into());
        let (first_events, _first_receiver) = mpsc::channel();
        let first = ControlHost::bind(runtime.path(), Arc::clone(&trusted), first_events)
            .expect("first host");
        let (_stream, mut reader) = authenticate(&first, "trusted-client");
        assert!(matches!(
            next(&mut reader),
            ControlResponse::Authenticated { .. }
        ));
        let first_token = first.token().to_owned();
        first.stop();

        let (second_events, _second_receiver) = mpsc::channel();
        let second =
            ControlHost::bind(runtime.path(), trusted, second_events).expect("second host");
        let (_stream, mut reader) = authenticate(&second, "trusted-client");
        assert!(matches!(
            next(&mut reader),
            ControlResponse::Authenticated { .. }
        ));
        assert_ne!(first_token, second.token());
    }

    #[test]
    fn bounded_frames_reject_oversize_input_and_unknown_operations() {
        let (mut writer, reader) = UnixStream::pair().expect("pair");
        let write =
            thread::spawn(move || writer.write_all(&vec![b'x'; MAX_CONTROL_MESSAGE_BYTES + 1]));
        let mut reader = BufReader::new(reader);
        let error = read_frame(&mut reader, &mut Vec::new()).expect_err("must reject");
        write
            .join()
            .expect("oversize writer")
            .expect("write oversized frame");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(
            serde_json::from_str::<ControlRequest>(
                r#"{"type":"voice_clone","speaker":"untrusted"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn socket_parent_symlink_is_refused() {
        let runtime = tempfile::tempdir().expect("tempdir");
        let target = runtime.path().join("target");
        fs::create_dir(&target).expect("target");
        std::os::unix::fs::symlink(&target, runtime.path().join("alp")).expect("symlink");
        let (events, _receiver) = mpsc::channel();
        let result = ControlHost::bind(
            runtime.path(),
            Arc::new(Mutex::new(BTreeSet::new())),
            events,
        );
        assert!(result.is_err(), "a symlinked socket parent must be refused");
    }

    #[test]
    fn stale_cleanup_refuses_regular_files_and_symlinks() {
        let runtime = tempfile::tempdir().expect("tempdir");
        let path = default_socket_path(runtime.path());
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(&path, b"do not remove").expect("file");
        assert!(prepare_socket_path(&path).is_err());
        assert_eq!(fs::read(&path).expect("read"), b"do not remove");
        fs::remove_file(&path).expect("remove");
        std::os::unix::fs::symlink("elsewhere", &path).expect("symlink");
        assert!(prepare_socket_path(&path).is_err());
        assert!(
            fs::symlink_metadata(&path)
                .expect("metadata")
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn authentication_policy_is_strict() {
        assert!(valid_client_id("client-1.test_name"));
        assert!(!valid_client_id(""));
        assert!(!valid_client_id("client with spaces"));
        assert!(!valid_client_id(&"x".repeat(65)));
        assert!(constant_time_eq(b"same", b"same"));
        assert!(!constant_time_eq(b"same", b"sam"));
        assert!(!constant_time_eq(b"same", b"diff"));

        let runtime = tempfile::tempdir().expect("tempdir");
        let (host, _trusted, events) = host(runtime.path());
        let mut stream = UnixStream::connect(host.socket_path()).expect("connect");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        write_request(&mut stream, &ControlRequest::ProjectInfo).expect("write");
        assert!(matches!(
            next(&mut reader),
            ControlResponse::Error { ref code, .. } if code == "authentication_required"
        ));
        assert!(events.try_recv().is_err());
    }
}
