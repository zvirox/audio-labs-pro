//! Dispatch helpers for the extended dubbing and repair tool coverage.
//!
//! Kept separate from `dispatch.rs` so the main match stays under the
//! repository's 1500-line limit.

use alp_api::{
    ApiSession, DubbingFitRequestDto, DubbingRemuxRequestDto, DubbingStemsRequestDto,
    SpectralRepairRequestDto, TimeStretchRequestDto,
};
use serde_json::Value;

use crate::protocol::ToolCallResult;

fn required_str<'a>(arguments: &'a Value, name: &str) -> Result<&'a str, ToolCallResult> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ToolCallResult::failure(format!("{name} is required")))
}

fn optional_f32(arguments: &Value, name: &str, default: f32) -> f32 {
    arguments
        .get(name)
        .and_then(Value::as_f64)
        .map(|value| value as f32)
        .unwrap_or(default)
}

pub(crate) fn dispatch_dubbing_fit_rate(
    session: &mut ApiSession,
    arguments: &Value,
) -> ToolCallResult {
    let audio_path = match required_str(arguments, "audio_path") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let output_path = match required_str(arguments, "output_path") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let target_duration_secs = match arguments
        .get("target_duration_secs")
        .and_then(Value::as_f64)
    {
        Some(value) => value,
        None => return ToolCallResult::failure("target_duration_secs is required"),
    };
    let request = DubbingFitRequestDto {
        audio_path,
        output_path,
        target_duration_secs,
    };
    match session.dubbing_fit_rate(&request) {
        Ok(report) => ToolCallResult::success(
            format!(
                "fitted take written to {} in {}s",
                report.path, report.duration_secs
            ),
            serde_json::to_value(report).unwrap(),
        ),
        Err(error) => ToolCallResult::failure(format!("dubbing_fit_rate failed: {error}")),
    }
}

pub(crate) fn dispatch_dubbing_remux(
    session: &mut ApiSession,
    arguments: &Value,
) -> ToolCallResult {
    let video_path = match required_str(arguments, "video_path") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let output_path = match required_str(arguments, "output_path") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let request = DubbingRemuxRequestDto {
        video_path,
        output_path,
        audio_path: arguments
            .get("audio_path")
            .and_then(Value::as_str)
            .filter(|path| !path.is_empty())
            .map(str::to_owned),
        subtitle_path: arguments
            .get("subtitle_path")
            .and_then(Value::as_str)
            .filter(|path| !path.is_empty())
            .map(str::to_owned),
    };
    match session.dubbing_remux(&request) {
        Ok(report) => ToolCallResult::success(
            format!("remuxed video written to {}", report.output_path),
            serde_json::to_value(report).unwrap(),
        ),
        Err(error) => ToolCallResult::failure(format!("dubbing_remux failed: {error}")),
    }
}

pub(crate) fn dispatch_dubbing_separate_stems(
    session: &mut ApiSession,
    arguments: &Value,
) -> ToolCallResult {
    let audio_path = match required_str(arguments, "audio_path") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let output_dir = match required_str(arguments, "output_dir") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let request = DubbingStemsRequestDto {
        audio_path,
        output_dir,
    };
    match session.dubbing_separate_stems(&request) {
        Ok(report) => ToolCallResult::success(
            format!(
                "separated {} stems at {} Hz",
                report.stems.len(),
                report.sample_rate
            ),
            serde_json::to_value(report).unwrap(),
        ),
        Err(error) => ToolCallResult::failure(format!("dubbing_separate_stems failed: {error}")),
    }
}

pub(crate) fn dispatch_dsp_spectral_repair(
    session: &mut ApiSession,
    arguments: &Value,
) -> ToolCallResult {
    let input_path = match required_str(arguments, "input_path") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let output_path = match required_str(arguments, "output_path") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let mode = match required_str(arguments, "mode") {
        Ok(mode) => mode.to_owned(),
        Err(error) => return error,
    };
    let harmonics = arguments
        .get("harmonics")
        .and_then(Value::as_u64)
        .unwrap_or(4);
    let request = SpectralRepairRequestDto {
        input_path,
        output_path,
        mode,
        center_hz: optional_f32(arguments, "center_hz", 1_000.0),
        fundamental_hz: optional_f32(arguments, "fundamental_hz", 220.0),
        harmonics: harmonics as usize,
        bandwidth_q: optional_f32(arguments, "bandwidth_q", 12.0),
        reduction_db: optional_f32(arguments, "reduction_db", 24.0),
    };
    match session.dsp_spectral_repair(&request) {
        Ok(report) => ToolCallResult::success(
            format!(
                "{} repair written to {} ({} frames)",
                report.mode, report.path, report.frames
            ),
            serde_json::to_value(report).unwrap(),
        ),
        Err(error) => ToolCallResult::failure(format!("dsp_spectral_repair failed: {error}")),
    }
}

pub(crate) fn dispatch_dsp_time_stretch(
    session: &mut ApiSession,
    arguments: &Value,
) -> ToolCallResult {
    let input_path = match required_str(arguments, "input_path") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let output_path = match required_str(arguments, "output_path") {
        Ok(path) => path.to_owned(),
        Err(error) => return error,
    };
    let ratio = match arguments.get("ratio").and_then(Value::as_f64) {
        Some(value) => value as f32,
        None => return ToolCallResult::failure("ratio is required"),
    };
    let request = TimeStretchRequestDto {
        input_path,
        output_path,
        ratio,
    };
    match session.dsp_time_stretch(&request) {
        Ok(report) => ToolCallResult::success(
            format!(
                "stretched audio written to {} in {}s",
                report.path, report.duration_secs
            ),
            serde_json::to_value(report).unwrap(),
        ),
        Err(error) => ToolCallResult::failure(format!("dsp_time_stretch failed: {error}")),
    }
}
