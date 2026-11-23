//! Reusable Model Context Protocol server for Audio Labs Pro.
//!
//! Both the headless binary and the desktop attach bridge use this dispatcher;
//! attached operation never gets a reduced or separately maintained tool list.

mod dispatch;
pub mod limits;
pub mod live;
pub mod protocol;
pub mod server;
pub mod starship;
pub mod tool_defs;

pub use protocol::{JsonRpcRequest, JsonRpcResponse, ToolCallResult};
pub use server::McpServer;
pub use starship::build_alp_bridge;
pub use tool_defs::tools_for_tier;
