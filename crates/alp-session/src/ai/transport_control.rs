//! Live transport tool interception for attached MCP clients.
//!
//! Transport tools touch the playback controller, not the project snapshot, so
//! they are handled on the UI thread before the scratch worker and revision
//! commit path. Headless mode never reaches this module.

use std::sync::mpsc::SyncSender;

use alp_api::control::ControlResponse;
use alp_api::PermissionTier;
use alp_mcp::{JsonRpcRequest, JsonRpcResponse, ToolCallResult, live};
use serde_json::Value;

use super::{AiControlManager, send_mcp_response};
use crate::{EditSession, TransportState};

fn transport_failure(
    response: &SyncSender<ControlResponse>,
    request: &JsonRpcRequest,
    message: impl Into<String>,
) {
    let result = ToolCallResult::failure(message);
    let value = serde_json::to_value(result).unwrap_or(Value::Null);
    send_mcp_response(
        response,
        JsonRpcResponse::success(request.id.clone().unwrap_or(Value::Null), value),
    );
}

/// Handle a live `transport_*` tool call against the attached transport.
pub(crate) fn handle_transport_call(
    manager: &AiControlManager,
    request: &JsonRpcRequest,
    response: &SyncSender<ControlResponse>,
    session: &EditSession,
) {
    let Some(tool) = live::tool_name(request) else {
        return;
    };
    let Some(transport) = &manager.transport else {
        transport_failure(
            response,
            request,
            "transport control is not attached to this desktop session",
        );
        return;
    };
    if tool != "transport_state" && !manager.tier.allows(PermissionTier::Edit) {
        transport_failure(
            response,
            request,
            "transport control requires the Edit permission tier",
        );
        return;
    }
    let arguments = request
        .params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let mut controller = match transport.lock() {
        Ok(controller) => controller,
        Err(error) => {
            transport_failure(
                response,
                request,
                format!("transport lock poisoned: {error}"),
            );
            return;
        }
    };

    let outcome: Result<TransportState, String> = match tool {
        "transport_state" => Ok(controller.state()),
        "transport_play" => controller.play(session),
        "transport_pause" => controller.pause(),
        "transport_stop" => controller.stop(),
        "transport_seek" => {
            let Some(secs) = arguments.get("position_secs").and_then(Value::as_f64) else {
                transport_failure(response, request, "position_secs is required");
                return;
            };
            controller.seek(secs)
        }
        "transport_set_volume" => {
            let Some(volume) = arguments.get("volume").and_then(Value::as_f64) else {
                transport_failure(response, request, "volume is required");
                return;
            };
            controller.set_volume(volume as f32);
            Ok(controller.state())
        }
        "transport_set_loop" => {
            let Some(enabled) = arguments.get("enabled").and_then(Value::as_bool) else {
                transport_failure(response, request, "enabled is required");
                return;
            };
            let Some(start) = arguments.get("start_secs").and_then(Value::as_f64) else {
                transport_failure(response, request, "start_secs is required");
                return;
            };
            let Some(end) = arguments.get("end_secs").and_then(Value::as_f64) else {
                transport_failure(response, request, "end_secs is required");
                return;
            };
            controller.set_loop(enabled, start, end);
            Ok(controller.state())
        }
        other => {
            transport_failure(
                response,
                request,
                format!("unknown transport tool '{other}'"),
            );
            return;
        }
    };

    match outcome {
        Ok(state) => {
            let summary = format!(
                "transport {tool}: playing={}, position={:.3}s",
                state.playing, state.position_secs
            );
            match serde_json::to_value(state) {
                Ok(state_value) => {
                    let result = ToolCallResult::success(summary, state_value);
                    match serde_json::to_value(result) {
                        Ok(value) => send_mcp_response(
                            response,
                            JsonRpcResponse::success(request.id.clone().unwrap_or(Value::Null), value),
                        ),
                        Err(error) => transport_failure(response, request, error.to_string()),
                    }
                }
                Err(error) => transport_failure(response, request, error.to_string()),
            }
        }
        Err(error) => transport_failure(
            response,
            request,
            format!("transport {tool} failed: {error}"),
        ),
    }
}
