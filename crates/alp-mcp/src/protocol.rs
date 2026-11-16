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
///
/// Serialization adds `title` and `annotations` from
/// [`crate::tool_defs::metadata_for`] so the behavioural hints live in one
/// table instead of being restated at each of the ~58 definition sites.
#[derive(Debug, Clone, Deserialize)]
pub struct McpTool {
    /// Stable tool name.
    pub name: String,
    /// Plain-text description for AI agents.
    pub description: String,
    /// Input parameters schema (JSON Schema 2020-12).
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

impl serde::Serialize for McpTool {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;

        let metadata = crate::tool_defs::metadata_for(&self.name);
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("title", &metadata.title)?;
        map.serialize_entry("description", &self.description)?;
        map.serialize_entry("inputSchema", &self.input_schema)?;
        map.serialize_entry("annotations", &metadata.annotations.to_json())?;
        map.end()
    }
}

/// Advisory hints about what a tool does, per the MCP `annotations` field.
///
/// Clients use these to decide whether a call needs confirmation or can be
/// retried. They describe the tool's contract, not a guarantee about any one
/// invocation, so `alp-mcp` derives them from a static table rather than from
/// runtime behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolAnnotations {
    /// Whether the tool only reads state.
    pub read_only: bool,
    /// Whether the tool may remove or overwrite data (all edits are undoable
    /// unless the tool writes a file).
    pub destructive: bool,
    /// Whether repeating the call with the same arguments leaves the same state.
    pub idempotent: bool,
    /// Whether the tool reaches outside the workspace (e.g. host-wide plugin
    /// discovery).
    pub open_world: bool,
}

impl ToolAnnotations {
    /// Annotations for a read-only inspection tool.
    #[must_use]
    pub const fn read_only() -> Self {
        Self {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        }
    }

    /// Annotations for a non-destructive edit that adds something each call.
    #[must_use]
    pub const fn additive() -> Self {
        Self {
            read_only: false,
            destructive: false,
            idempotent: false,
            open_world: false,
        }
    }

    /// Annotations for an edit that removes or overwrites something.
    #[must_use]
    pub const fn destructive() -> Self {
        Self {
            read_only: false,
            destructive: true,
            idempotent: false,
            open_world: false,
        }
    }

    /// Serialize as the MCP `annotations` object.
    #[must_use]
    pub fn to_json(self) -> Value {
        serde_json::json!({
            "readOnlyHint": self.read_only,
            "destructiveHint": self.destructive,
            "idempotentHint": self.idempotent,
            "openWorldHint": self.open_world,
        })
    }
}

/// Static metadata attached to a tool at `tools/list` time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolMetadata {
    /// Short human-facing title for pickers and approval prompts.
    pub title: String,
    /// Behaviour hints.
    pub annotations: ToolAnnotations,
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

    /// Append a note to the text summary, keeping the structured payload.
    ///
    /// Used to report a boundary condition (such as a call that exceeded its
    /// advisory time budget) without discarding the real result.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        if let Some(McpContent::Text { text }) = self.content.first_mut() {
            text.push_str("\n\n");
            text.push_str(&note.into());
        }
        self
    }
}
