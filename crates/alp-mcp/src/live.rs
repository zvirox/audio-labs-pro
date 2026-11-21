//! Small live-session adaptations around the shared MCP response contract.

use alp_api::{HistoryEntry, ProjectSummary};
use serde_json::{Value, json};

use crate::protocol::{JsonRpcRequest, JsonRpcResponse, ToolCallResult};

/// Name of a tool call, if this is a `tools/call` request.
#[must_use]
pub fn tool_name(request: &JsonRpcRequest) -> Option<&str> {
    (request.method == "tools/call")
        .then(|| request.params.get("name").and_then(Value::as_str))
        .flatten()
}

/// Replace summary fields that belong to the live session rather than the
/// worker's intentionally empty scratch undo stack.
pub fn patch_project_summary(
    request: &JsonRpcRequest,
    response: &mut JsonRpcResponse,
    summary: &ProjectSummary,
) {
    if tool_name(request) == Some("project_info")
        && let Some(result) = &mut response.result
    {
        result["structuredContent"] = serde_json::to_value(summary).unwrap_or(Value::Null);
        result["content"][0]["text"] = Value::String(format!(
            "Project '{}' (rev {}): {:.2}s, {} tracks, {} clips",
            summary.name,
            summary.revision,
            summary.duration_secs,
            summary.track_count,
            summary.clip_count
        ));
    }
    if request.method == "resources/read"
        && request.params.get("uri").and_then(Value::as_str) == Some("alp://project/active")
        && let Some(contents) = response
            .result
            .as_mut()
            .and_then(|result| result["contents"].as_array_mut())
        && let Some(content) = contents.first_mut()
    {
        content["text"] = Value::String(serde_json::to_string(summary).unwrap_or_default());
    }
}

/// Turn a successful scratch result into an atomic attached commit refusal.
pub fn fail_commit(response: &mut JsonRpcResponse, message: impl Into<String>) {
    response.result = Some(serde_json::to_value(ToolCallResult::failure(message)).unwrap());
    response.error = None;
}

/// Shared `history_undo` result, after operating on the GUI's actual stack.
#[must_use]
pub fn history_undo_result(undone: bool) -> ToolCallResult {
    ToolCallResult::success(
        if undone {
            "Undone last command"
        } else {
            "Nothing to undo"
        },
        json!({ "undone": undone }),
    )
}

/// Shared `history_redo` result, after operating on the GUI's actual stack.
#[must_use]
pub fn history_redo_result(redone: bool) -> ToolCallResult {
    ToolCallResult::success(
        if redone {
            "Redone last command"
        } else {
            "Nothing to redo"
        },
        json!({ "redone": redone }),
    )
}

/// Page and format the existing live stack using the headless tool's contract.
#[must_use]
pub fn history_list_result(entries: Vec<HistoryEntry>, arguments: &Value) -> ToolCallResult {
    let request = match alp_api::PageRequest::resolve(
        arguments.get("limit").and_then(Value::as_u64),
        arguments.get("cursor").and_then(Value::as_str),
    ) {
        Ok(request) => request,
        Err(error) => return ToolCallResult::failure(error.to_string()),
    };
    let page = alp_api::paging::paginate(entries, request);
    let undoable = page.items.iter().filter(|entry| entry.is_undo).count();
    let summary = format!(
        "{} history entr{} on this page ({} undoable, {} total){}",
        page.items.len(),
        if page.items.len() == 1 { "y" } else { "ies" },
        undoable,
        page.total,
        if page.is_last() {
            String::new()
        } else {
            format!(
                "; pass cursor '{}' for the next page",
                page.next_cursor.clone().unwrap_or_default()
            )
        }
    );
    ToolCallResult::success(summary, serde_json::to_value(page).unwrap())
}
