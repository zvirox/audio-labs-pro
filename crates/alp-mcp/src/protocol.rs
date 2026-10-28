//! Model Context Protocol (MCP) JSON-RPC 2.0 messages and data structures.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standard JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version string, must be "2.0".
    pub jsonrpc: String,
    /// Request identifier (null for notifications).
    pub id: Option<Value>,
    /// Protocol method name.
    pub method: String,
    /// Optional method parameters.
    #[serde(default)]
    pub params: Value,
}

/// Standard JSON-RPC 2.0 response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version string ("2.0").
    pub jsonrpc: String,
    /// Correlated request identifier.
    pub id: Value,
    /// Successful result payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error payload if call failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a success response.
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: Value, code: i64, message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data,
            }),
        }
    }
}

/// JSON-RPC 2.0 error body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i64,
    /// Plain-language description.
    pub message: String,
    /// Optional additional error data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP tool descriptor for `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Stable tool name.
    pub name: String,
    /// Plain-text description for AI agents.
    pub description: String,
    /// Input parameters schema (JSON Schema 2020-12).
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// MCP content element in tool execution results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpContent {
    /// Text response content.
    Text { text: String },
}

/// Result returned from `tools/call`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// Content items (usually text summaries for models).
    pub content: Vec<McpContent>,
    /// Structured JSON data for downstream parsing.
    #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
    /// Whether the tool execution resulted in a recoverable domain error.
    #[serde(rename = "isError")]
    pub is_error: bool,
}

impl ToolCallResult {
    /// Create a successful result with text and structured content.
    pub fn success(summary: impl Into<String>, structured: Value) -> Self {
        Self {
            content: vec![McpContent::Text {
                text: summary.into(),
            }],
            structured_content: Some(structured),
            is_error: false,
        }
    }

    /// Create an error result with clear recovery guidance.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            content: vec![McpContent::Text {
                text: message.into(),
            }],
            structured_content: None,
            is_error: true,
        }
    }
}
