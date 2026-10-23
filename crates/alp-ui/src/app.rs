//! The Audio Labs Pro shell: state, messages, layout and panel views.
//!
//! Everything that mutates the UI is routed through [`Message`]. Worker threads
//! communicate through [`JobQueue`]; the UI never waits on them. The eight panel
//! stubs share one floating manager, so dragging, snapping, docking and z-order
//! behave identically everywhere.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use alp_ui_kit::components;
use alp_ui_kit::motion::{PanelMotion, Toggle};
use alp_ui_kit::overlays::{self, CommandItem};
use alp_ui_kit::tokens::{Control, Duration as MotionDuration, Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, ThemeMode, WaveformWidget, fonts};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Point, Size, Subscription,
    Task, Theme, mouse, window,
};

use alp_core::{ClipId, frames_to_secs, now_ms};
use alp_engine::{CaptureConfig, Recording, list_devices, start_recording};
use alp_media::ExportSettings;

use crate::commands::{self, CommandEntry, CommandId, PaletteState};
use crate::demo;
use crate::glass::{self, GlassMode, GlassReport};
use crate::jobs::{JobQueue, JobStatus};
use crate::panels::{DockSide, PanelKind, PanelManager};
use crate::session::EditSession;
use crate::workspace::PersistedWorkspace;

mod update;
mod views;

/// Top-level shell messages.
#[derive(Debug, Clone)]
pub enum Message {
    /// iced opened the main window.
    WindowOpened(window::Id),
    /// A window event (resize, focus, close).
    WindowEvent(window::Id, window::Event),
    /// The window's current inner size.
    WindowResized(Size),
    /// Close request from the compositor.
    CloseRequested,
    /// Glass probe completed.
    GlassProbed(GlassReport),
    /// Raw input event.
    Event(iced::Event),
    /// Animation frame.
    Frame(Instant),
    /// Start dragging a panel.
    DragPanel(PanelKind),
    /// Start resizing a panel.
    ResizePanel(PanelKind),
    /// Toggle a panel.
    TogglePanel(PanelKind),
    /// Close a panel.
    ClosePanel(PanelKind),
    /// Toggle collapse.
    CollapsePanel(PanelKind),
    /// Dock or undock a panel.
    ToggleDock(PanelKind),
    /// Palette text changed.
    PaletteInput(String),
    /// Run the highlighted palette command.
    RunPaletteSelection,
    /// Run a command by id.
    RunCommand(CommandId),
    /// Theme cycle.
    ToggleTheme,
    /// Reduce motion.
    ToggleReduceMotion,
    /// Save layout.
    SaveLayout,
    /// Reset layout.
    ResetLayout,
    /// Play/pause.
    PlayPause,
    /// Stop and rewind.
    Stop,
    /// Import demo media.
    ImportMedia,
    /// Import a file dropped onto the window.
    ImportPath(PathBuf),
    /// Rebuild peaks (worker stub).
    BuildPeaks,
    /// Queue a demo export.
    QueueExport,
    /// Cancel a job.
    CancelJob(u64),
    /// Remove finished jobs.
    ClearJobs,
    /// Dismiss the toast.
    DismissToast,
    /// Window title drag.
    WindowDrag,
    /// Minimize.
    WindowMinimize,
    /// Toggle maximize.
    WindowMaximize,
    /// Close button.
    WindowClose,
    /// Select a clip in the editing track.
    SelectClip(ClipId),
    /// Selection start text.
    SelectionStart(String),
    /// Selection end text.
    SelectionEnd(String),
    /// Apply the typed selection.
    ApplySelection,
    /// Cut the selection.
    CutSelection,
    /// Copy the selection.
    CopySelection,
    /// Paste the clipboard.
    PasteClipboard,
    /// Delete the selection or selected clip.
    DeleteSelection,
    /// Silence the selection.
    SilenceSelection,
    /// Lower the clip envelope over the selection.
    DipEnvelope,
    /// Clear the selected clip envelope.
    ResetEnvelope,
    /// Trim the selected clip to the selection.
    TrimSelection,
    /// Split the selected clip at the cursor.
    SplitSelection,
    /// Duplicate the selected clip.
    DuplicateSelection,
    /// Normalize the selected clip.
    NormalizeSelection,
    /// Reverse the selected clip.
    ToggleReverse,
    /// Invert the selected clip.
    ToggleInvert,
    /// Undo the last edit.
    UndoEdit,
    /// Redo the next edit.
    RedoEdit,
    /// Clip gain for the selected clip.
    EditGain(f32),
    /// Clip fade-in fraction.
    EditFadeIn(f32),
    /// Clip fade-out fraction.
    EditFadeOut(f32),
    /// Select an input device.
    SelectInput(String),
    /// Toggle input monitoring.
    ToggleMonitor,
    /// Toggle recording into the selected range instead of appending.
    TogglePunchIn,
    /// Start or stop a recording.
    ToggleRecord,
    /// Export the current mix.
    ExportMix,
    /// Select a media row.
    SelectMedia(usize),
    /// Select an effect row.
    SelectEffect(usize),
    /// Select a voice row.
    SelectVoice(String),
    /// Select a track.
    SelectTrack(usize),
    /// Crossfade the selected clip with the following clip.
    CrossfadeSelection(f32),
    /// Set the master bus gain.
    SetMasterGain(f32),
    /// Set a track's linear gain.
    SetTrackGain(usize, f32),
    /// Set a track's pan position.
    SetTrackPan(usize, f32),
    /// Toggle a track's mute state.
    ToggleTrackMute(usize),
    /// Toggle a track's solo state.
    ToggleTrackSolo(usize),
    /// Toggle a track's record-arm state.
    ToggleTrackArm(usize),
    /// Zoom waveform in/out.
    ZoomWaveform(f32),
    /// Seek waveform.
    SeekWaveform(f32),
    /// No-op task result.
    Noop,
}

#[derive(Debug, Clone, Copy)]
enum Drag {
    Panel { kind: PanelKind, grab: Point },
    Resize { kind: PanelKind },
}

/// One media-row stub.
#[derive(Debug, Clone)]
struct MediaItem {
    name: String,
    duration_secs: f32,
    channels: u16,
}

/// One effect stub.
#[derive(Debug, Clone)]
struct EffectItem {
    name: String,
    enabled: bool,
}

/// One model stub.
#[derive(Debug, Clone)]
struct ModelItem {
    name: String,
    purpose: String,
    installed: bool,
    size: String,
}

/// The application.
pub struct App {
    window: Option<window::Id>,
    panels: PanelManager,
    panel_motion: HashMap<PanelKind, Toggle>,
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

    media: Vec<MediaItem>,
    effects: Vec<EffectItem>,
    models: Vec<ModelItem>,
    voices: Vec<String>,
    selected_media: usize,
    selected_effect: usize,
    selected_voice: String,
    selected_track: usize,

    jobs: JobQueue,
    layout_restored: bool,
    bounds_initialized: bool,
    save_path: Option<PathBuf>,
    last_save_error: Option<String>,

    last_frame: Option<Instant>,
    frame_ms: f32,
    metrics: bool,
    metrics_frames: u32,
    metrics_last: Instant,
    demo_autoscroll: bool,
    autoclose: Option<Instant>,
}

impl App {
    /// Boot the shell and restore the last workspace.
    pub fn boot() -> (Self, Task<Message>) {
        let dirs = alp_core::AppDirs::discover().ok();
        let save_path = dirs.as_ref().map(PersistedWorkspace::path);
        let restored = save_path
            .as_ref()
            .and_then(|path| PersistedWorkspace::load(path));
        let layout_restored = restored.is_some();

        let theme_mode = restored
            .as_ref()
            .map_or(ThemeMode::Dark, |layout| ThemeMode::parse(&layout.theme));
        let reduce_motion = restored.as_ref().is_some_and(|layout| layout.reduce_motion);
        let system_dark = alp_ui_kit::tokens::system_is_dark();
        let saved_glass = restored.as_ref().map_or(GlassMode::TintFallback, |layout| {
            GlassMode::parse(&layout.glass)
        });

        let bounds = Size::new(1440.0, 900.0);
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

        let colors = theme_colors(theme_mode, system_dark);
        let seconds = demo::configured_seconds();
        let (waveform, frames) = demo::waveform(seconds, colors);
        let media_duration = frames as f32 / 48_000.0;
        let metrics = std::env::var_os("ALP_METRICS").is_some();
        let demo_autoscroll = std::env::var_os("ALP_DEMO_AUTOSCROLL").is_some();
        let autoclose = std::env::var("ALP_AUTOCLOSE_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(|seconds| Instant::now() + Duration::from_secs(seconds));

        let media = vec![
            MediaItem {
                name: "demo-session.wav".into(),
                duration_secs: media_duration,
                channels: 1,
            },
            MediaItem {
                name: "voice-take-01.wav".into(),
                duration_secs: 42.8,
                channels: 1,
            },
        ];

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
        let input_devices: Vec<String> = list_devices()
            .unwrap_or_default()
            .into_iter()
            .filter(|device| device.direction == alp_engine::DeviceDirection::Input)
            .map(|device| device.name)
            .collect();
        let record_device = input_devices.first().cloned();

        let app = Self {
            window: None,
            panels,
            panel_motion,
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
            media,
            effects: vec![
                EffectItem {
                    name: "Noise gate".into(),
                    enabled: true,
                },
                EffectItem {
                    name: "De-esser".into(),
                    enabled: false,
                },
                EffectItem {
                    name: "Studio EQ".into(),
                    enabled: true,
                },
            ],
            models: vec![
                ModelItem {
                    name: "Kokoro 82M".into(),
                    purpose: "Speech synthesis".into(),
                    installed: true,
                    size: "340 MB".into(),
                },
                ModelItem {
                    name: "Whisper Small".into(),
                    purpose: "Transcription".into(),
                    installed: true,
                    size: "466 MB".into(),
                },
                ModelItem {
                    name: "RVC Base".into(),
                    purpose: "Voice cloning".into(),
                    installed: false,
                    size: "1.1 GB".into(),
                },
            ],
            voices: vec![
                "Narrator (neutral)".into(),
                "Studio alto".into(),
                "Documentary warm".into(),
            ],
            selected_media: 0,
            selected_effect: 0,
            selected_voice: "Narrator (neutral)".into(),
            selected_track: 0,
            jobs: JobQueue::new(),
            layout_restored,
            bounds_initialized: false,
            save_path,
            last_save_error: None,
            last_frame: None,
            frame_ms: 16.7,
            metrics,
            metrics_frames: 0,
            metrics_last: Instant::now(),
            demo_autoscroll,
            autoclose,
        };
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
            || self.session.is_importing()
            || self.session.needs_autosave()
        {
            subscriptions.push(window::frames().map(Message::Frame));
        }
        Subscription::batch(subscriptions)
    }

    fn animating(&self) -> bool {
        self.panel_motion.values().any(Toggle::is_animating)
    }

    /// iced theme.
    pub fn theme(&self) -> Theme {
        self.colors().to_theme()
    }

    fn colors(&self) -> ThemeColors {
        theme_colors(self.theme_mode, self.system_dark)
    }
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
