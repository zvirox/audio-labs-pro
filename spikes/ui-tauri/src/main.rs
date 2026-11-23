//! M0 Feasibility Spike: Throwaway Tauri 2 Desktop Shell
//!
//! Evaluates Tauri 2 with WebKitGTK against the iced baseline:
//! - Transparent, undecorated window.
//! - KWin/X11 blur property via `x11rb` and safe window handle.
//! - Inner-panel CSS backdrop-filter test in WebKitGTK.
//! - 1-hour waveform autoscroll on Canvas 2D at 60 fps.
//! - Idle CPU (4s) and RSS memory benchmarks.
//! - Session probe recording.

use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_type: String,
    pub desktop: String,
    pub display: String,
    pub handle_type: String,
    pub kwin_blur_active: bool,
    pub kwin_blur_detail: String,
    pub force_tint: bool,
    pub is_benchmark: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub fps_mean: f32,
    pub frame_time_ms: f32,
    pub frame_time_min_ms: f32,
    pub frame_time_max_ms: f32,
    pub idle_cpu_percent: f32,
    pub rss_mb: f32,
    pub total_frames: u32,
    pub dropped_frames: u32,
    pub gate_fps_pass: bool,
    pub gate_cpu_pass: bool,
    pub gate_glass_pass: bool,
}

struct SpikeState {
    synthetic_peaks: Vec<f32>,
    session_info: SessionInfo,
    _start_time: Instant,
    autoclose: Option<Duration>,
    is_benchmark: bool,
    metrics_printed: AtomicBool,
}

/// Generate synthetic 1-hour peak pyramid min/max pairs (48kHz, 256 bucket).
fn generate_synthetic_hour_peaks() -> Vec<f32> {
    const SAMPLE_RATE: u32 = 48_000;
    const TOTAL_SECONDS: u32 = 3_600;
    const SAMPLES: u64 = (TOTAL_SECONDS as u64) * (SAMPLE_RATE as u64);
    const BUCKET: usize = 256;

    let total_buckets = (SAMPLES as usize) / BUCKET;
    let mut pairs = Vec::with_capacity(total_buckets * 2);

    let lut: Vec<f32> = (0..4096)
        .map(|i| (std::f32::consts::TAU * (i as f32) / 4096.0).sin())
        .collect();
    let lut_index = |hz: f32, t: f32| -> f32 {
        let phase = ((hz * t * 4096.0) as usize) % 4096;
        lut[phase]
    };

    let phrase_lut: Vec<f32> = (0..4096)
        .map(|i| {
            let phase = std::f32::consts::TAU * (i as f32) / 4096.0;
            phase.sin().max(0.0).powf(0.6)
        })
        .collect();

    for b in 0..total_buckets {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for s in 0..8 {
            let idx = (b * BUCKET + s * (BUCKET / 8)) as u64;
            let t = (idx as f32) / (SAMPLE_RATE as f32);
            let phrase = phrase_lut[((t * 0.22 * 4096.0) as usize) % 4096];
            let tremolo = 0.55 + 0.45 * lut_index(11.0, t);
            let carrier =
                lut_index(220.0, t) * 0.6 + lut_index(440.0, t) * 0.25 + lut_index(97.0, t) * 0.15;
            let sample = (carrier * phrase * tremolo).clamp(-0.95, 0.95);

            min = min.min(sample);
            max = max.max(sample);
        }
        pairs.push(min);
        pairs.push(max);
    }

    pairs
}

/// Read process CPU ticks and RSS in MB from /proc/self.
fn read_process_stats() -> (u64, f32) {
    let mut cpu_ticks = 0u64;
    let mut rss_mb = 0.0f32;

    if let Ok(mut stat_file) = File::open("/proc/self/stat") {
        let mut buf = String::new();
        if stat_file.read_to_string(&mut buf).is_ok() {
            let parts: Vec<&str> = buf.split_whitespace().collect();
            if parts.len() > 14 {
                let utime: u64 = parts[13].parse().unwrap_or(0);
                let stime: u64 = parts[14].parse().unwrap_or(0);
                cpu_ticks = utime + stime;
            }
        }
    }

    if let Ok(mut status_file) = File::open("/proc/self/status") {
        let mut buf = String::new();
        if status_file.read_to_string(&mut buf).is_ok() {
            for line in buf.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<f32>() {
                            rss_mb = kb / 1024.0;
                        }
                    }
                    break;
                }
            }
        }
    }

    (cpu_ticks, rss_mb)
}

/// Safe KWin blur invocation using x11rb
fn enable_kwin_x11_blur(window: &tauri::WebviewWindow) -> Result<(), String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, PropMode};
    use x11rb::wrapper::ConnectionExt as _;

    let handle = window
        .window_handle()
        .map_err(|e| format!("cannot obtain window handle: {e}"))?;

    let window_id = match handle.as_raw() {
        RawWindowHandle::Xlib(xlib) => u32::try_from(xlib.window)
            .map_err(|_| "Xlib window id exceeds 32 bits".to_string())?,
        RawWindowHandle::Xcb(xcb) => xcb.window.get(),
        _ => return Err("window handle is not X11 (Xlib or Xcb)".into()),
    };

    let (conn, _screen) =
        x11rb::connect(None).map_err(|e| format!("cannot connect to X11 server: {e}"))?;

    let geom = conn
        .get_geometry(window_id)
        .map_err(|e| format!("get_geometry failed: {e}"))?
        .reply()
        .map_err(|e| format!("geometry reply failed: {e}"))?;

    let atom = conn
        .intern_atom(false, b"_KDE_NET_WM_BLUR_BEHIND_REGION")
        .map_err(|e| format!("intern_atom failed: {e}"))?
        .reply()
        .map_err(|e| format!("atom reply failed: {e}"))?
        .atom;

    conn.change_property32(
        PropMode::REPLACE,
        window_id,
        atom,
        AtomEnum::CARDINAL,
        &[0, 0, u32::from(geom.width), u32::from(geom.height)],
    )
    .map_err(|e| format!("change_property32 failed: {e}"))?
    .check()
    .map_err(|e| format!("blur property rejected: {e}"))?;

    conn.flush()
        .map_err(|e| format!("flush X11 connection failed: {e}"))?;

    Ok(())
}

#[tauri::command]
fn get_session_info(state: State<'_, SpikeState>) -> SessionInfo {
    state.session_info.clone()
}

#[tauri::command]
fn get_waveform_peaks(state: State<'_, SpikeState>) -> Vec<f32> {
    state.synthetic_peaks.clone()
}

#[tauri::command]
fn get_system_stats() -> (f32, f32) {
    let (_, rss) = read_process_stats();
    (0.0, rss)
}

#[tauri::command]
fn report_benchmark(app: AppHandle, result: BenchmarkResult, state: State<'_, SpikeState>) {
    if !state.metrics_printed.swap(true, Ordering::SeqCst) {
        println!("\n=== M0 TAURI FEASIBILITY SPIKE RESULTS ===");
        println!("Session Type:      {}", state.session_info.session_type);
        println!("Desktop:           {}", state.session_info.desktop);
        println!("Display Handle:    {}", state.session_info.handle_type);
        println!("KWin Blur:         {}", if state.session_info.kwin_blur_active { "ACTIVE" } else { "TINT FALLBACK" });
        println!("Waveform FPS:      {:.2} fps (gate: >= 55 fps -> {})", result.fps_mean, if result.gate_fps_pass { "PASS" } else { "FAIL" });
        println!("Frame Time:        {:.2} ms (min: {:.2} ms, max: {:.2} ms)", result.frame_time_ms, result.frame_time_min_ms, result.frame_time_max_ms);
        println!("Total Frames:      {} (dropped: {})", result.total_frames, result.dropped_frames);
        println!("Idle CPU (4s):     {:.2}% (gate: < 1.0% -> {})", result.idle_cpu_percent, if result.gate_cpu_pass { "PASS" } else { "FAIL" });
        println!("Process RSS:       {:.2} MB (baseline: ~88 MB)", result.rss_mb);
        println!("Overall M0 Gate:   {}", if result.gate_fps_pass && result.gate_cpu_pass { "PASSED" } else { "FAILED" });
        println!("==========================================\n");
    }

    if state.is_benchmark || state.autoclose.is_some() {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            app.exit(0);
        });
    }
}

#[tauri::command]
fn close_spike(app: AppHandle) {
    app.exit(0);
}

fn main() {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".into());
    let display = std::env::var("WAYLAND_DISPLAY")
        .map(|d| format!("wayland:{d}"))
        .or_else(|_| std::env::var("DISPLAY").map(|d| format!("x11:{d}")))
        .unwrap_or_else(|_| "unknown".into());
    let force_tint = std::env::var_os("ALP_FORCE_TINT").is_some();

    let autoclose = std::env::var("ALP_SPIKE_AUTOCLOSE_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs);
    let is_benchmark = std::env::var_os("ALP_SPIKE_BENCHMARK").is_some();

    println!("Generating synthetic 1-hour peak pyramid...");
    let peaks = generate_synthetic_hour_peaks();
    println!("Generated {} peak pairs (1-hour audio signal).", peaks.len() / 2);

    let mut session_info = SessionInfo {
        session_type,
        desktop,
        display,
        handle_type: "pending".into(),
        kwin_blur_active: false,
        kwin_blur_detail: "Pending window setup".into(),
        force_tint,
        is_benchmark,
    };

    tauri::Builder::default()
        .setup(move |app| {
            let window = app.get_webview_window("main").unwrap();

            let handle_type = if let Ok(handle) = window.window_handle() {
                match handle.as_raw() {
                    RawWindowHandle::Wayland(_) => "wayland",
                    RawWindowHandle::Xlib(_) => "xlib",
                    RawWindowHandle::Xcb(_) => "xcb",
                    _ => "other",
                }
            } else {
                "error"
            };

            session_info.handle_type = handle_type.into();

            let is_kde = session_info.desktop.to_ascii_lowercase().contains("kde")
                || session_info.desktop.to_ascii_lowercase().contains("plasma");

            if !force_tint && (handle_type == "xlib" || handle_type == "xcb") && is_kde {
                match enable_kwin_x11_blur(&window) {
                    Ok(()) => {
                        session_info.kwin_blur_active = true;
                        session_info.kwin_blur_detail =
                            "_KDE_NET_WM_BLUR_BEHIND_REGION applied successfully via x11rb.".into();
                        println!("KWin X11 blur activated on Tauri window via safe x11rb.");
                    }
                    Err(e) => {
                        session_info.kwin_blur_detail = format!("X11 blur failed: {e}; using tint fallback.");
                        println!("KWin X11 blur note: {e}; fallback tint active.");
                    }
                }
            } else if force_tint {
                session_info.kwin_blur_detail = "ALP_FORCE_TINT requested; using translucent tint fallback.".into();
            } else {
                session_info.kwin_blur_detail =
                    "Wayland or non-KDE desktop; translucent tint fallback active.".into();
            }

            let spike_state = SpikeState {
                synthetic_peaks: peaks,
                session_info,
                _start_time: Instant::now(),
                autoclose,
                is_benchmark,
                metrics_printed: AtomicBool::new(false),
            };

            app.manage(spike_state);

            if let Some(dur) = autoclose {
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(dur);
                    println!("Autoclose interval ({:?}) reached, closing window.", dur);
                    app_handle.exit(0);
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_session_info,
            get_waveform_peaks,
            get_system_stats,
            report_benchmark,
            close_spike
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
