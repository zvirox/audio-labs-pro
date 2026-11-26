//! Dispatch helpers for the Phase 19 studio coverage tools.
//!
//! Kept separate from `dispatch.rs` so the main match stays under the
//! repository's 1500-line limit. Every arm validates required arguments and
//! reports failures by naming the offending field.

use alp_api::{
    AcxChapterFileDto, ApiSession, EqBandDto, M4bChapterDto, M4bExportRequestDto, M4bMetadataDto,
    compute_eq_response,
};
use serde_json::{Value, json};

use crate::protocol::ToolCallResult;

fn required_str<'a>(arguments: &'a Value, name: &str) -> Result<&'a str, ToolCallResult> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ToolCallResult::failure(format!("{name} is required")))
}

fn required_u64(arguments: &Value, name: &str) -> Result<u64, ToolCallResult> {
    arguments
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| ToolCallResult::failure(format!("{name} must be a non-negative integer")))
}

pub(crate) fn dispatch_effects_response(arguments: &Value) -> ToolCallResult {
    let Some(bands_value) = arguments.get("bands") else {
        return ToolCallResult::failure("bands is required");
    };
    let bands: Vec<EqBandDto> = match serde_json::from_value(bands_value.clone()) {
        Ok(bands) => bands,
        Err(error) => return ToolCallResult::failure(format!("invalid bands: {error}")),
    };
    let sample_rate = match required_u64(arguments, "sample_rate") {
        Ok(rate) if rate > 0 && rate <= u64::from(u32::MAX) => rate as u32,
        Ok(_) => return ToolCallResult::failure("sample_rate must be a positive u32"),
        Err(error) => return error,
    };
    let points = arguments
        .get("points")
        .and_then(Value::as_u64)
        .unwrap_or(64);
    let response = compute_eq_response(&bands, sample_rate, points as usize);
    ToolCallResult::success(
        format!(
            "computed {} frequency points",
            response.frequencies_hz.len()
        ),
        serde_json::to_value(response).unwrap(),
    )
}

pub(crate) fn dispatch_acx_qa_analyze(
    session: &mut ApiSession,
    arguments: &Value,
) -> ToolCallResult {
    let Some(chapters_value) = arguments.get("chapters") else {
        return ToolCallResult::failure("chapters is required");
    };
    let parsed: Result<Vec<AcxChapterFileDto>, _> = serde_json::from_value(chapters_value.clone());
    let chapters = match parsed {
        Ok(chapters) if !chapters.is_empty() => chapters,
        Ok(_) => return ToolCallResult::failure("chapters must be a non-empty array"),
        Err(error) => return ToolCallResult::failure(format!("invalid chapters: {error}")),
    };
    match session.acx_qa_analyze(&chapters) {
        Ok(report) => ToolCallResult::success(
            format!(
                "{} of {} chapters compliant",
                report.compliant_count, report.total_count
            ),
            serde_json::to_value(report).unwrap(),
        ),
        Err(error) => ToolCallResult::failure(format!("ACX QA failed: {error}")),
    }
}

pub(crate) fn dispatch_audiobook_export_m4b(
    session: &mut ApiSession,
    arguments: &Value,
) -> ToolCallResult {
    let output_path = match required_str(arguments, "output_path") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let master_audio_path = match required_str(arguments, "master_audio_path") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let chapters: Vec<M4bChapterDto> = match arguments.get("chapters").and_then(Value::as_array) {
        Some(values) if !values.is_empty() => {
            match serde_json::from_value(Value::Array(values.clone())) {
                Ok(chapters) => chapters,
                Err(error) => return ToolCallResult::failure(format!("invalid chapters: {error}")),
            }
        }
        _ => return ToolCallResult::failure("chapters must be a non-empty array"),
    };
    let metadata_value = arguments
        .get("metadata")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let metadata: M4bMetadataDto = match serde_json::from_value(metadata_value) {
        Ok(metadata) => metadata,
        Err(error) => return ToolCallResult::failure(format!("invalid metadata: {error}")),
    };
    let request = M4bExportRequestDto {
        output_path,
        master_audio_path,
        chapters,
        cover_path: arguments
            .get("cover_path")
            .and_then(Value::as_str)
            .filter(|path| !path.is_empty())
            .map(str::to_owned),
        metadata,
    };
    match session.audiobook_export_m4b(&request) {
        Ok(report) => ToolCallResult::success(
            format!(
                "M4B written to {} with {} chapter(s)",
                report.path, report.chapter_count
            ),
            serde_json::to_value(report).unwrap(),
        ),
        Err(error) => ToolCallResult::failure(format!("M4B export failed: {error}")),
    }
}

pub(crate) fn dispatch_marker_remove(
    session: &mut ApiSession,
    arguments: &Value,
) -> ToolCallResult {
    let marker_id = match required_str(arguments, "marker_id") {
        Ok(id) => id.to_owned(),
        Err(error) => return error,
    };
    match session.marker_remove(&marker_id) {
        Ok(()) => ToolCallResult::success(
            format!("removed marker {marker_id}"),
            json!({ "marker_id": marker_id }),
        ),
        Err(error) => ToolCallResult::failure(format!("marker_remove failed: {error}")),
    }
}

pub(crate) fn dispatch_loop_range_set(
    session: &mut ApiSession,
    arguments: &Value,
) -> ToolCallResult {
    let start_secs = arguments.get("start_secs").and_then(Value::as_f64);
    let end_secs = arguments.get("end_secs").and_then(Value::as_f64);
    match session.loop_range_set(start_secs, end_secs) {
        Ok(()) => ToolCallResult::success(
            if start_secs.is_some() {
                "loop range set".to_owned()
            } else {
                "loop range cleared".to_owned()
            },
            json!({ "start_secs": start_secs, "end_secs": end_secs }),
        ),
        Err(error) => ToolCallResult::failure(format!("loop_range_set failed: {error}")),
    }
}

pub(crate) fn dispatch_clip_trim(session: &mut ApiSession, arguments: &Value) -> ToolCallResult {
    let clip_id = match required_str(arguments, "clip_id") {
        Ok(id) => id.to_owned(),
        Err(error) => return error,
    };
    let fields = [
        "source_start_frame",
        "source_end_frame",
        "timeline_start_frame",
    ];
    let mut frames = [0_u64; 3];
    for (index, field) in fields.iter().enumerate() {
        frames[index] = match required_u64(arguments, field) {
            Ok(value) => value,
            Err(error) => return error,
        };
    }
    match session.clip_trim(&clip_id, frames[0], frames[1], frames[2]) {
        Ok(()) => ToolCallResult::success(
            format!("trimmed clip {clip_id}"),
            json!({
                "clip_id": clip_id,
                "source_start_frame": frames[0],
                "source_end_frame": frames[1],
                "timeline_start_frame": frames[2],
            }),
        ),
        Err(error) => ToolCallResult::failure(format!("clip_trim failed: {error}")),
    }
}
