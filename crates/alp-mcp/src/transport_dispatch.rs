//! Headless fallback for live transport tools.
//!
//! A headless `alp-mcp` has no audio transport, no control socket and no
//! desktop token. These arms therefore fail explicitly instead of inventing a
//! `TransportState` or silently doing nothing. In attached mode the desktop
//! host intercepts the same tool names before dispatch (see
//! `alp_session::ai::transport_control`).

use crate::protocol::ToolCallResult;

const ATTACH_REQUIRED: &str = "transport_control requires a live desktop attach: start alp-mcp --attach and enable AI control in the running app. Headless sessions have no audio transport.";

/// Return the honest headless error for any live transport tool.
pub(crate) fn dispatch_transport(tool_name: &str) -> ToolCallResult {
    ToolCallResult::failure(format!("{ATTACH_REQUIRED} (tool: {tool_name})"))
}
