//! The Audio Labs Pro shell: state, messages, layout and panel views.
//!
//! Everything that mutates the UI is routed through [`Message`]. Worker threads
//! communicate through [`JobQueue`]; the UI never waits on them. The eight panel
//! stubs share one floating manager, so dragging, snapping, docking and z-order
//! behave identically everywhere.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use alp_ui_kit::components;
use alp_ui_kit::motion::{PanelMotion, Toggle};
use alp_ui_kit::overlays::{self, CommandItem};
use alp_ui_kit::tokens::{Control, Duration as MotionDuration, Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, ThemeMode, WaveformWidget, fonts};
use iced::widget::text_editor;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Point, Size, Subscription, Task, Theme,
    mouse, window,
};

use alp_core::{frames_to_secs, now_ms};
use alp_engine::{
    CaptureConfig, PlaybackHandle, Recording, list_devices, play_scene_live, start_recording,
};
use alp_voice::{ModelManager, ProfileStore, Script, VoiceProfile};

use crate::commands::{self, CommandEntry, CommandId, PaletteState};
use crate::demo;
use crate::glass::{self, GlassMode, GlassReport};
use crate::jobs::{JobQueue, JobStatus};
use crate::panels::{DockSide, PanelKind, PanelManager};
use crate::session::EditSession;
use crate::workspace::PersistedWorkspace;

mod ai_control;
mod ai_mcp;
mod ai_views;
mod capture_views;
mod create_voice_views;
mod effects_views;
mod export_handlers;
mod generate_views;
mod message;
mod model_actions;
mod settings_views;
mod stories_views;
mod studio;
mod track_editor;
mod types;
mod update;
mod views;
mod voice_views;
mod wizard_views;
mod workspace_layout;

pub use message::{CreateVoiceTab, Message};
use types::*;
pub use workspace_layout::WorkflowTab;

/// The application.
pub struct App {
    window: Option<window::Id>,
    panels: PanelManager,
    panel_motion: HashMap<PanelKind, Toggle>,
    /// One-shot settle motion for workspace changes.
    workspace_motion: Toggle,
    /// Latest window size, used for responsive columns and gutters.
    window_size: Size,
    drag: Option<Drag>,
    last_cursor: Point,

    theme_mode: ThemeMode,
    system_dark: bool,
    reduce_motion: bool,
    glass: GlassReport,
    palette: PaletteState,
    palette_id: iced::widget::Id,
    commands: Vec<CommandEntry>,
    toast: Option<(String, Instant)>,

    waveform: WaveformWidget,
    /// Tint the waveform was last drawn with, to detect accent changes.
    waveform_tint: ThemeColors,
    media_duration: f32,
    playing: bool,
    position_secs: f32,
    gain: f32,
    session: EditSession,
    selection_start: String,
    selection_end: String,
    edit_gain: f32,
    edit_fade_in: f32,
    edit_fade_out: f32,
    input_devices: Vec<String>,
    record_device: Option<String>,
    record_monitor: bool,
    punch_in: bool,
    recording: Option<Recording>,
    record_path: Option<PathBuf>,
    record_started: Option<Instant>,
    record_peak: f32,
    record_rms: f32,

    effects: Vec<EffectItem>,
    discovered_plugins: Vec<alp_plugin::PluginDescriptor>,
    models_offline: bool,
    model_manager: ModelManager,
    /// Model id to job id for running background downloads.
    model_jobs: HashMap<String, u64>,
    /// Model id to the last failed download's inline error.
    model_errors: HashMap<String, String>,
    /// Last observed alp-infer daemon state.
    engine: model_actions::EngineState,
    /// Slot worker threads use to hand back engine probes.
    engine_probe: Arc<Mutex<Option<model_actions::EngineState>>>,
    /// Whether an engine probe is in flight.
    engine_checking: bool,
    profiles: Vec<VoiceProfile>,
    profile_store: Option<ProfileStore>,
    selected_profile: String,
    profile_name: String,
    profile_tags: String,
    history: Vec<HistoryItem>,
    media_root: PathBuf,
    generation_slot: Arc<Mutex<Option<Result<RenderOutput, String>>>>,
    generation_kind: GenerationKind,
    generating: bool,
    pending_story: Option<usize>,
    stories: Vec<Script>,
    selected_story: usize,
    selected_story_line: Option<usize>,
    story_dir: PathBuf,
    story_text: String,
    captures: Vec<CaptureItem>,
    selected_capture: usize,
    voice_text: String,
    voice_editor: text_editor::Content,
    player_title: String,
    voice_status: String,
    create_voice_open: bool,
    create_voice_tab: CreateVoiceTab,
    create_voice_reference: String,
    /// Speaker-permission declaration, required before the consent record.
    create_voice_consent: bool,
    create_voice_language: String,
    stories_tools_open: bool,
    dictating: bool,
    dictation_preview: String,
    transcript: alp_voice::Transcript,
    selected_segment: Option<usize>,
    selected_effect: usize,
    reference_audit_status: Option<String>,
    consent_dialog_open: bool,
    consent_speaker: String,
    consent_attester: String,
    hardware_summary: String,
    selected_track: usize,
    ripple: bool,
    first_run_open: bool,
    shortcuts_help_open: bool,

    workflow_tab: WorkflowTab,
    selected_subtabs: [usize; 7],
    inspector_open: bool,

    ai_enabled: bool,
    ai_tier: alp_api::PermissionTier,
    ai_status: String,
    ai_control: ai_control::AiControlState,
    ai_activity_log: Vec<String>,

    jobs: JobQueue,
    layout_restored: bool,
    bounds_initialized: bool,
    save_path: Option<PathBuf>,
    last_save_error: Option<String>,

    playback: Option<PlaybackHandle>,
    playback_rate: u32,
    playback_meters: Option<std::sync::Arc<alp_engine::TrackMeterBank>>,
    last_frame: Option<Instant>,
    frame_ms: f32,
    metrics: bool,
    metrics_frames: u32,
    metrics_last: Instant,
    demo_autoscroll: bool,
    autoclose: Option<Instant>,
}

impl App {
    fn live_track_peak(&self, track_index: usize) -> f32 {
        let Some(meters) = self.playback_meters.as_ref() else {
            return self.record_peak;
        };
        let Some(track) = self.session.tracks().get(track_index) else {
            return 0.0;
        };
        meters
            .track_ids
            .iter()
            .position(|id| id == &track.id)
            .map(|index| meters.track_meters[index].peak().clamp(0.0, 1.0))
            .unwrap_or(0.0)
    }
    /// Boot the shell and restore the last workspace.
    pub fn boot() -> (Self, Task<Message>) {
        let dirs = alp_core::AppDirs::discover().ok();
        let save_path = dirs.as_ref().map(PersistedWorkspace::path);
        let restored = save_path
            .as_ref()
            .and_then(|path| PersistedWorkspace::load(path));
        let layout_restored = restored.is_some();
        let first_run_completed = restored
            .as_ref()
            .is_some_and(|layout| layout.first_run_completed);
        let first_run_open = !first_run_completed;

        let theme_mode = restored
            .as_ref()
            .map_or(ThemeMode::Dark, |layout| ThemeMode::parse(&layout.theme));
        let reduce_motion = restored.as_ref().is_some_and(|layout| layout.reduce_motion);
        let system_dark = alp_ui_kit::tokens::system_is_dark();
        let saved_glass = restored.as_ref().map_or(GlassMode::TintFallback, |layout| {
            GlassMode::parse(&layout.glass)
        });

        let bounds = alp_session::panels::Size::new(1440.0, 900.0);
        let mut panels = PanelManager::default_for(bounds);
        if let Some(layout) = &restored {
            panels.restore(&layout.panels);
        }

        let mut panel_motion = HashMap::new();
        for kind in PanelKind::ALL {
            let open = panels.panel(kind).open;
            let mut toggle =
                Toggle::new(if open { 1.0 } else { 0.0 }).with_duration(MotionDuration::PANEL_MS);
            toggle.set_reduce_motion(reduce_motion);
            panel_motion.insert(kind, toggle);
        }
        let mut workspace_motion = Toggle::new(1.0).with_duration(MotionDuration::BASE_MS);
        workspace_motion.set_reduce_motion(reduce_motion);

        // Smoke runs can pin a workspace so each tab can be screenshotted.
        let start_tab = std::env::var("ALP_START_TAB")
            .ok()
            .and_then(|value| WorkflowTab::from_slug(&value))
            .unwrap_or(WorkflowTab::Generate);

        // The palette carries the boot workspace's neon hue; the update loop
        // re-tints it when the active workspace (or theme) changes.
        let colors = theme_colors(theme_mode, system_dark).with_neon(start_tab.neon());
        let seconds = demo::configured_seconds();
        let (waveform, frames) = demo::waveform(seconds, colors);
        let media_duration = frames as f32 / 48_000.0;
        let metrics = std::env::var_os("ALP_METRICS").is_some();
        let demo_autoscroll = std::env::var_os("ALP_DEMO_AUTOSCROLL").is_some();
        let autoclose = std::env::var("ALP_AUTOCLOSE_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(|seconds| Instant::now() + Duration::from_secs(seconds));

        let data_root = dirs
            .as_ref()
            .map_or_else(|| PathBuf::from("."), |dirs| dirs.data.clone());
        let media_root = data_root.join("generated");
        let story_dir = data_root.join("stories");
        let _ = std::fs::create_dir_all(&media_root);
        let _ = std::fs::create_dir_all(&story_dir);

        let (profile_store, profiles) = match ProfileStore::default_store() {
            Ok(store) => {
                let mut list = store.list_profiles();
                if list.is_empty() {
                    list = default_profiles();
                }
                (Some(store), list)
            }
            Err(_) => (None, default_profiles()),
        };
        let selected_profile = profiles
            .first()
            .map_or_else(|| "aura".to_owned(), |profile| profile.id.clone());

        let mut stories = load_stories(&story_dir);
        if stories.is_empty() {
            let mut script = Script::new("First Story", &selected_profile);
            script.add_line("Welcome to Audio Labs Pro, a local voice and multitrack workstation.");
            script.add_line("Every take is rendered on this machine.");
            stories.push(script);
        }

        let model_manager = ModelManager::with_config(alp_voice::manager::ModelManagerConfig {
            storage_dir: data_root.join("models"),
            offline_mode: true,
        });

        let session = dirs.as_ref().map_or_else(EditSession::demo, |dirs| {
            EditSession::open_or_demo(dirs.projects_dir().join("ui-demo"))
        });
        let selection = session.selection();
        let rate = session.project().sample_rate;
        let selection_start = format!("{:.2}", frames_to_secs(selection.range().start, rate));
        let selection_end = format!("{:.2}", frames_to_secs(selection.range().end, rate));
        let selected = session.selected_clip().and_then(|id| session.clip(id));
        let edit_gain = selected.map_or(1.0, |clip| clip.gain);
        let edit_fade_in = selected.map_or(0.0, |clip| {
            if clip.is_empty() {
                0.0
            } else {
                clip.fade_in_frames as f32 / clip.len() as f32
            }
        });
        let edit_fade_out = selected.map_or(0.0, |clip| {
            if clip.is_empty() {
                0.0
            } else {
                clip.fade_out_frames as f32 / clip.len() as f32
            }
        });
        let mut input_devices: Vec<String> = list_devices()
            .unwrap_or_default()
            .into_iter()
            .filter(|device| device.direction == alp_engine::DeviceDirection::Input)
            .map(|device| device.name)
            .filter(|name| !is_placeholder_device(name))
            .collect();
        input_devices.sort();
        input_devices.dedup();
        let record_device = input_devices.first().cloned();

        let mut app = Self {
            window: None,
            panels,
            panel_motion,
            workspace_motion,
            window_size: Size::new(bounds.width, bounds.height),
            drag: None,
            last_cursor: Point::ORIGIN,
            theme_mode,
            system_dark,
            reduce_motion,
            glass: GlassReport {
                mode: saved_glass,
                detail: format!("Restored {}; probing the window…", saved_glass.as_str()),
                ..GlassReport::pending()
            },
            palette: PaletteState::new(),
            palette_id: iced::widget::Id::unique(),
            commands: commands::registry(),
            toast: None,
            waveform,
            waveform_tint: colors,
            media_duration,
            playing: false,
            position_secs: 0.0,
            gain: 1.0,
            session,
            selection_start,
            selection_end,
            edit_gain,
            edit_fade_in,
            edit_fade_out,
            input_devices,
            record_device,
            record_monitor: false,
            punch_in: false,
            recording: None,
            record_path: None,
            record_started: None,
            record_peak: 0.0,
            record_rms: 0.0,
            effects: vec![
                EffectItem {
                    name: "De-Hum".into(),
                    enabled: true,
                    params: "50/60 Hz notch cascade".into(),
                },
                EffectItem {
                    name: "De-Click".into(),
                    enabled: true,
                    params: "Transient click repair".into(),
                },
                EffectItem {
                    name: "Voice EQ".into(),
                    enabled: true,
                    params: "5-band parametric (HPF 80Hz)".into(),
                },
                EffectItem {
                    name: "De-Esser".into(),
                    enabled: true,
                    params: "6.0 kHz sibilance detector".into(),
                },
                EffectItem {
                    name: "Compressor".into(),
                    enabled: true,
                    params: "Soft-knee vocal leveller".into(),
                },
                EffectItem {
                    name: "Peak Limiter".into(),
                    enabled: true,
                    params: "Ceiling -1.0 dBFS".into(),
                },
            ],
            discovered_plugins: alp_plugin::PluginHost::new().list_plugins(),
            models_offline: true,
            model_manager,
            model_jobs: HashMap::new(),
            model_errors: HashMap::new(),
            engine: model_actions::EngineState::default(),
            engine_probe: Arc::new(Mutex::new(None)),
            engine_checking: false,
            profiles,
            profile_store,
            selected_profile,
            profile_name: "Studio Voice".into(),
            profile_tags: "warm documentary narration".into(),
            history: Vec::new(),
            media_root,
            generation_slot: Arc::new(Mutex::new(None)),
            generation_kind: GenerationKind::Take,
            generating: false,
            pending_story: None,
            stories,
            selected_story: 0,
            selected_story_line: None,
            story_dir,
            story_text: String::new(),
            captures: Vec::new(),
            selected_capture: 0,
            selected_effect: 0,
            voice_text: "Hello from Audio Labs Pro.".into(),
            voice_editor: text_editor::Content::with_text("Hello from Audio Labs Pro."),
            player_title: "Studio session".into(),
            voice_status: "Ready".into(),
            create_voice_open: false,
            create_voice_tab: CreateVoiceTab::Record,
            create_voice_reference: String::new(),
            create_voice_consent: false,
            create_voice_language: "English".into(),
            stories_tools_open: false,
            dictating: false,
            dictation_preview: String::new(),
            transcript: alp_voice::Transcript::new(),
            selected_segment: None,
            reference_audit_status: None,
            consent_dialog_open: false,
            consent_speaker: "Speaker Name".into(),
            consent_attester: "Audio Engineer".into(),
            hardware_summary: {
                let hw = alp_voice::HardwareProfile::probe();
                format!(
                    "{:.1} GB RAM · {} cores · {}",
                    hw.total_ram_gb(),
                    hw.cpu_cores,
                    hw.recommended_tier.as_str()
                )
            },
            selected_track: 0,
            ripple: true,
            first_run_open,
            shortcuts_help_open: false,
            workflow_tab: start_tab,
            selected_subtabs: [0; 7],
            inspector_open: false,
            ai_enabled: false,
            ai_tier: alp_api::PermissionTier::Edit,
            ai_status: "AI attach off: enable to create an owner-only control socket".into(),
            ai_control: Self::ai_control_state(
                restored
                    .as_ref()
                    .map(|layout| layout.approved_ai_clients.clone())
                    .unwrap_or_default(),
            ),
            ai_activity_log: vec![
                "Project opened; edits from this window are undoable".into(),
                "Headless MCP opens projects separately; live attach is off".into(),
            ],
            jobs: JobQueue::new(),
            layout_restored,
            bounds_initialized: false,
            save_path,
            last_save_error: None,
            playback: None,
            playback_rate: 48_000,
            playback_meters: None,
            last_frame: None,
            frame_ms: 16.7,
            metrics,
            metrics_frames: 0,
            metrics_last: Instant::now(),
            demo_autoscroll,
            autoclose,
        };
        // Smoke runs can pin an overlay so each modal can be screenshotted.
        match std::env::var("ALP_START_OVERLAY").as_deref() {
            Ok("create-voice") => app.create_voice_open = true,
            Ok("consent") => app.consent_dialog_open = true,
            Ok("shortcuts") => app.shortcuts_help_open = true,
            Ok("palette") => app.palette.show(&app.commands),
            Ok("wizard") => app.first_run_open = true,
            _ => {}
        }
        (app, Task::none())
    }

    /// iced subscription graph. Frames exist only while something moves.
    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            window::open_events().map(Message::WindowOpened),
            window::close_requests().map(|_| Message::CloseRequested),
            window::events().map(|(id, event)| Message::WindowEvent(id, event)),
            iced::event::listen().map(Message::Event),
        ];
        if self.animating()
            || self.playing
            || self.jobs.has_running()
            || self.metrics
            || self.demo_autoscroll
            || self.autoclose.is_some()
            || self.recording.is_some()
            || self.generating
            || self.ai_enabled
            || self.session.is_importing()
            || self.session.needs_autosave()
        {
            subscriptions.push(window::frames().map(Message::Frame));
        }
        Subscription::batch(subscriptions)
    }

    fn animating(&self) -> bool {
        self.panel_motion.values().any(Toggle::is_animating) || self.workspace_motion.is_animating()
    }

    /// iced theme.
    pub fn theme(&self) -> Theme {
        self.colors().to_theme()
    }

    fn colors(&self) -> ThemeColors {
        theme_colors(self.theme_mode, self.system_dark).with_neon(self.workflow_tab.neon())
    }
}

/// ALSA exposes a "null" device with a long explanatory name on machines
/// without a real input; hide it rather than showing it as a microphone.
fn is_placeholder_device(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("discard all samples")
        || lower.contains("generate zero samples")
        || lower.starts_with("null")
}

fn default_profiles() -> Vec<VoiceProfile> {
    ["aura", "atlas", "echo", "sol"]
        .into_iter()
        .map(|id| {
            let name = match id {
                "atlas" => "Atlas",
                "echo" => "Echo",
                "sol" => "Sol",
                _ => "Aura",
            };
            VoiceProfile::builtin(id, name, vec!["builtin".into(), "english".into()])
        })
        .collect()
}

/// Load saved story scripts from `dir`, newest title first.
fn load_stories(dir: &std::path::Path) -> Vec<Script> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
                .collect()
        })
        .unwrap_or_default();
    paths.sort();

    paths
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(path).ok()?;
            let mut script: Script = serde_json::from_str(&text).ok()?;
            for line in &mut script.lines {
                line.audio = None;
                line.status = alp_voice::LineStatus::Pending;
            }
            Some(script)
        })
        .collect()
}

fn theme_colors(mode: ThemeMode, system_dark: bool) -> ThemeColors {
    match mode.resolve(system_dark) {
        ThemeMode::Light => ThemeColors::LIGHT,
        _ => ThemeColors::DARK,
    }
}

fn command_for_key(
    modifiers: iced::keyboard::Modifiers,
    key: &iced::keyboard::Key,
) -> Option<CommandId> {
    if modifiers.control()
        && modifiers.shift()
        && matches!(key, iced::keyboard::Key::Character(value) if value.eq_ignore_ascii_case("p"))
    {
        return Some(CommandId::TogglePalette);
    }
    [
        CommandId::TogglePalette,
        CommandId::ToggleTheme,
        CommandId::ToggleReduceMotion,
        CommandId::ResetLayout,
        CommandId::SaveLayout,
        CommandId::ImportMedia,
        CommandId::PlayPause,
    ]
    .into_iter()
    .find(|id| commands::shortcut(*id).is_some_and(|shortcut| shortcut.matches(modifiers, key)))
}

fn format_time(seconds: f32) -> String {
    let whole = seconds.max(0.0) as u64;
    format!("{}:{:02}", whole / 60, whole % 60)
}

/// Shorten a label to `max` characters on a char boundary, suffixing an
/// ellipsis. Shared by every list row that shows a file name so long media
/// names never wrap their row out of shape.
pub(crate) fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() > max {
        let short: String = text.chars().take(max).collect();
        format!("{short}…")
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_key_mapping_is_exact() {
        use iced::keyboard::{Key, Modifiers};
        let key = Key::Character("k".into());
        assert_eq!(
            command_for_key(Modifiers::CTRL, &key),
            Some(CommandId::TogglePalette)
        );
        assert_eq!(command_for_key(Modifiers::NONE, &key), None);
    }

    #[test]
    fn time_format_is_minutes_and_seconds() {
        assert_eq!(format_time(0.0), "0:00");
        assert_eq!(format_time(61.9), "1:01");
        assert_eq!(format_time(3_600.0), "60:00");
    }
}
