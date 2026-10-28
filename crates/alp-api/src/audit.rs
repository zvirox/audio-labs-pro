//! Audit logging for automation and MCP actions.

use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::ApiError;
use crate::types::PermissionTier;

/// One audit log entry recorded on automation tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Timestamp in milliseconds since UNIX epoch.
    pub timestamp_ms: u64,
    /// Connected client or agent identifier.
    pub client_id: String,
    /// Tool or API operation name.
    pub tool_name: String,
    /// SHA-256 or truncated digest of the arguments.
    pub arg_digest: String,
    /// Permission tier used.
    pub tier: PermissionTier,
    /// Result status: "ok" or error message.
    pub status: String,
    /// Project revision before mutation.
    pub revision_before: u64,
    /// Project revision after mutation.
    pub revision_after: u64,
}

/// Persistent JSONL audit logger.
#[derive(Debug, Clone)]
pub struct AuditLogger {
    path: PathBuf,
}

impl AuditLogger {
    /// Create or attach to an audit log file.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ApiError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    /// Record an operation event into the audit log.
    pub fn record(&self, entry: &AuditEntry) -> Result<(), ApiError> {
        let json = serde_json::to_string(entry)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{json}")?;
        Ok(())
    }

    /// Path to the audit log.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alp_core::now_ms;
    use tempfile::tempdir;

    #[test]
    fn audit_log_records_and_appends_jsonl() {
        let dir = tempdir().expect("tempdir");
        let log_file = dir.path().join("audit.jsonl");
        let logger = AuditLogger::new(&log_file).expect("logger");

        let entry = AuditEntry {
            timestamp_ms: now_ms(),
            client_id: "mcp-client-1".into(),
            tool_name: "clip_split".into(),
            arg_digest: "sha256_mock_digest".into(),
            tier: PermissionTier::Edit,
            status: "ok".into(),
            revision_before: 1,
            revision_after: 2,
        };

        logger.record(&entry).expect("record");
        let contents = std::fs::read_to_string(&log_file).expect("read");
        assert!(contents.contains("\"tool_name\":\"clip_split\""));
        assert!(contents.contains("\"client_id\":\"mcp-client-1\""));
    }
}
