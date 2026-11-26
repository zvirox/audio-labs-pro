//! Dispatch helper for the `dsp_macro_execute` MCP tool.
//!
//! Kept separate so `dispatch.rs` stays under the repository's 1500-line
//! limit while every tool arm keeps a single, explicit failure contract.

use alp_api::{ApiSession, MacroRequest, MacroStep};
use serde_json::{Value, json};

use crate::protocol::ToolCallResult;

/// Parse and dispatch a DSP macro batch call.
pub(crate) fn dispatch_dsp_macro(
    session: &mut ApiSession,
    arguments: &Value,
) -> ToolCallResult {
    let input_dir = arguments
        .get("input_dir")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let output_dir = arguments
        .get("output_dir")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    if input_dir.is_empty() || output_dir.is_empty() {
        return ToolCallResult::failure("input_dir and output_dir are required workspace paths");
    }
    let Some(steps_json) = arguments.get("steps").and_then(Value::as_array) else {
        return ToolCallResult::failure("steps must be a non-empty array");
    };
    if steps_json.is_empty() {
        return ToolCallResult::failure("steps must be a non-empty array");
    }

    let mut steps = Vec::with_capacity(steps_json.len());
    for step in steps_json {
        let Some(effect_id) = step.get("effect_id").and_then(Value::as_str) else {
            return ToolCallResult::failure("each step requires effect_id");
        };
        let params = step.get("params").cloned().unwrap_or_else(|| json!({}));
        let params: std::collections::BTreeMap<String, f64> =
            match serde_json::from_value(params) {
                Ok(params) => params,
                Err(error) => {
                    return ToolCallResult::failure(format!(
                        "invalid parameters for {effect_id}: {error}"
                    ));
                }
            };
        steps.push(MacroStep {
            effect_id: effect_id.to_owned(),
            params,
        });
    }

    let request = MacroRequest {
        input_dir,
        output_dir,
        pattern: arguments
            .get("pattern")
            .and_then(Value::as_str)
            .map(str::to_owned),
        overwrite: arguments
            .get("overwrite")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        steps,
    };
    match session.dsp_macro_execute(&request) {
        Ok(report) => ToolCallResult::success(
            format!(
                "Macro job {} processed {} file(s): {} succeeded, {} failed",
                report.job_id,
                report.report.processed,
                report.report.succeeded,
                report.report.failed
            ),
            serde_json::to_value(report).unwrap(),
        ),
        Err(error) => ToolCallResult::failure(format!("DSP macro failed: {error}")),
    }
}
