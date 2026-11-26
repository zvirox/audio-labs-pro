//! Attached MCP round-trip test for live transport control.
//!
//! Uses the real Unix control socket and `AiControlManager::tick`, so it proves
//! that `transport_*` tools drive the injected controller on the UI thread
//! without touching the project snapshot.

use std::collections::BTreeSet;
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use alp_api::control::{
    CONTROL_PROTOCOL_VERSION, ControlRequest, ControlResponse, read_response, write_request,
};
use alp_mcp::{JsonRpcRequest, JsonRpcResponse};
use alp_session::{AiControlManager, EditSession, PermissionTier, TransportController};

fn call_mcp(
    stream: &mut UnixStream,
    rx: &Receiver<ControlResponse>,
    manager: &mut AiControlManager,
    session: &mut EditSession,
    id: i64,
    name: &str,
    arguments: serde_json::Value,
) -> JsonRpcResponse {
    write_request(
        stream,
        &ControlRequest::Mcp {
            message: serde_json::to_value(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(id)),
                method: "tools/call".into(),
                params: serde_json::json!({ "name": name, "arguments": arguments }),
            })
            .expect("encode request"),
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

fn structured(response: &JsonRpcResponse) -> serde_json::Value {
    let result = response.result.as_ref().expect("tool result");
    assert_ne!(
        result.get("isError").and_then(serde_json::Value::as_bool),
        Some(true),
        "tool returned an error: {result}"
    );
    result
        .get("structuredContent")
        .cloned()
        .expect("structured content")
}

#[test]
fn attached_transport_tools_drive_the_live_controller() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut session = EditSession::open_or_demo(temp.path().join("project"));
    let initial_undo = session.undo_steps();
    let transport = Arc::new(Mutex::new(TransportController::new(60.0, 48_000)));

    let mut trusted = BTreeSet::new();
    trusted.insert("transport-agent".to_owned());
    let mut manager = AiControlManager::new(trusted);
    manager.set_transport(Arc::clone(&transport));
    let status = manager
        .start(temp.path(), PermissionTier::Edit)
        .expect("start control host");

    let mut stream = UnixStream::connect(status.socket_path.expect("socket")).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("read timeout");
    let mut reader = BufReader::new(stream.try_clone().expect("clone"));
    write_request(
        &mut stream,
        &ControlRequest::Authenticate {
            protocol_version: CONTROL_PROTOCOL_VERSION,
            token: status.token.expect("token"),
            client_id: "transport-agent".into(),
        },
    )
    .expect("auth");
    assert!(matches!(
        read_response(&mut reader).expect("auth response"),
        Some(ControlResponse::Authenticated { .. })
    ));

    let (tx, rx): (SyncSender<ControlResponse>, Receiver<ControlResponse>) = mpsc::sync_channel(16);
    std::thread::spawn(move || {
        while let Ok(Some(response)) = read_response(&mut reader) {
            if tx.send(response).is_err() {
                break;
            }
        }
    });

    let played = structured(&call_mcp(
        &mut stream,
        &rx,
        &mut manager,
        &mut session,
        1,
        "transport_play",
        serde_json::json!({}),
    ));
    assert_eq!(played["playing"], true);

    let state = structured(&call_mcp(
        &mut stream,
        &rx,
        &mut manager,
        &mut session,
        2,
        "transport_state",
        serde_json::json!({}),
    ));
    assert_eq!(state["playing"], true);

    let seeked = structured(&call_mcp(
        &mut stream,
        &rx,
        &mut manager,
        &mut session,
        3,
        "transport_seek",
        serde_json::json!({ "position_secs": 5.0 }),
    ));
    assert!(seeked["position_secs"].as_f64().unwrap_or(0.0) >= 5.0);

    let looped = structured(&call_mcp(
        &mut stream,
        &rx,
        &mut manager,
        &mut session,
        4,
        "transport_set_loop",
        serde_json::json!({ "enabled": true, "start_secs": 1.0, "end_secs": 3.0 }),
    ));
    assert_eq!(looped["loop_enabled"], true);

    let paused = structured(&call_mcp(
        &mut stream,
        &rx,
        &mut manager,
        &mut session,
        5,
        "transport_pause",
        serde_json::json!({}),
    ));
    assert_eq!(paused["playing"], false);

    assert_eq!(
        session.undo_steps(),
        initial_undo,
        "transport tools must not create project undo entries"
    );
    manager.stop("done");
}
