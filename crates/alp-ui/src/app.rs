//! The Audio Lab Pro shell: state, messages, layout and panel views.
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
use iced::widget::{Space, column, container, mouse_area, row, scrollable, stack, text};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Point, Size, Subscription,
    Task, Theme, mouse, window,
};

use crate::commands::{self, CommandEntry, CommandId, PaletteState};
use crate::demo;
use crate::glass::{self, GlassMode, GlassReport};
use crate::jobs::{JobQueue, JobStatus};
use crate::panels::{DockSide, PanelKind, PanelManager};
use crate::workspace::PersistedWorkspace;

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
    /// Change clip gain.
    SetGain(f32),
    /// Change fade in.
    SetFadeIn(f32),
    /// Change fade out.
    SetFadeOut(f32),
    /// Select a media row.
    SelectMedia(usize),
    /// Select an effect row.
    SelectEffect(usize),
    /// Select a voice row.
    SelectVoice(String),
    /// Select a track.
    SelectTrack(usize),
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
    fade_in: f32,
    fade_out: f32,

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
            gain: 0.82,
            fade_in: 0.04,
            fade_out: 0.08,
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

    /// Apply a message.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowOpened(id) => {
                self.window = Some(id);
                Task::batch([
                    window::run(id, |window| glass::probe(window)).map(Message::GlassProbed),
                    window::size(id).map(Message::WindowResized),
                ])
            }
            Message::WindowEvent(_id, event) => {
                if let window::Event::Resized(size) = event {
                    self.apply_window_size(size);
                }
                Task::none()
            }
            Message::WindowResized(size) => {
                self.apply_window_size(size);
                Task::none()
            }
            Message::CloseRequested => {
                self.save_workspace();
                Task::none()
            }
            Message::GlassProbed(report) => {
                self.glass = report;
                Task::none()
            }
            Message::Frame(now) => {
                self.jobs.poll();
                if self.demo_autoscroll {
                    self.waveform.scroll_by(0.35);
                }
                if self.metrics {
                    self.metrics_frames += 1;
                    if self.metrics_last.elapsed() >= Duration::from_secs(1) {
                        eprintln!(
                            "alp-ui frames/s={} frame_ms={:.2} wave={:.0}s",
                            self.metrics_frames, self.frame_ms, self.media_duration
                        );
                        self.metrics_frames = 0;
                        self.metrics_last = Instant::now();
                    }
                }
                if self.autoclose.is_some_and(|deadline| now >= deadline) {
                    return iced::exit();
                }
                if let Some(last) = self.last_frame {
                    let dt = now.duration_since(last).as_secs_f32();
                    self.frame_ms = self.frame_ms * 0.9 + dt * 1000.0 * 0.1;
                    if self.playing {
                        self.position_secs += dt;
                        if self.position_secs >= self.media_duration {
                            self.position_secs = 0.0;
                            self.playing = false;
                        }
                        self.waveform.set_playhead(self.position_secs);
                    }
                }
                self.last_frame = Some(now);
                for toggle in self.panel_motion.values_mut() {
                    toggle.tick(now);
                }
                Task::none()
            }
            Message::DragPanel(kind) => {
                let rect = self.panels.panel(kind).rect;
                self.drag = Some(Drag::Panel {
                    kind,
                    grab: Point::new(self.last_cursor.x - rect.x, self.last_cursor.y - rect.y),
                });
                self.panels.focus(kind);
                Task::none()
            }
            Message::ResizePanel(kind) => {
                self.drag = Some(Drag::Resize { kind });
                self.panels.focus(kind);
                Task::none()
            }
            Message::TogglePanel(kind) => {
                self.panels.toggle_open(kind);
                let open = self.panels.panel(kind).open;
                self.set_panel_open(kind, open);
                Task::none()
            }
            Message::ClosePanel(kind) => {
                self.set_panel_open(kind, false);
                Task::none()
            }
            Message::CollapsePanel(kind) => {
                self.panels.toggle_collapsed(kind);
                self.save_workspace();
                Task::none()
            }
            Message::ToggleDock(kind) => {
                if self.panels.panel(kind).docked.is_some() {
                    self.panels.undock(kind);
                } else {
                    self.panels.dock(kind, DockSide::Right);
                }
                self.save_workspace();
                Task::none()
            }
            Message::PaletteInput(query) => {
                self.palette.set_query(&query, &self.commands);
                Task::none()
            }
            Message::RunPaletteSelection => {
                if let Some(entry) = self.palette.selected_command(&self.commands) {
                    self.palette.hide();
                    return self.run_command(entry.id);
                }
                Task::none()
            }
            Message::RunCommand(id) => self.run_command(id),
            Message::ToggleTheme => {
                self.theme_mode = self.theme_mode.next();
                self.system_dark = alp_ui_kit::tokens::system_is_dark();
                self.save_workspace();
                Task::none()
            }
            Message::ToggleReduceMotion => {
                self.reduce_motion = !self.reduce_motion;
                for toggle in self.panel_motion.values_mut() {
                    toggle.set_reduce_motion(self.reduce_motion);
                }
                self.save_workspace();
                Task::none()
            }
            Message::SaveLayout => {
                self.save_workspace();
                Task::none()
            }
            Message::ResetLayout => {
                self.panels.reset_geometry();
                self.save_workspace();
                self.show_toast("Layout reset");
                Task::none()
            }
            Message::PlayPause => {
                self.playing = !self.playing;
                if self.playing {
                    self.last_frame = None;
                    self.waveform.set_playhead(self.position_secs);
                }
                Task::none()
            }
            Message::Stop => {
                self.playing = false;
                self.position_secs = 0.0;
                self.waveform.set_playhead(0.0);
                Task::none()
            }
            Message::ImportMedia => {
                self.jobs.spawn("Import demo-session.wav", |reporter| {
                    for step in 0..=60u16 {
                        if reporter.is_cancelled() {
                            return Err("cancelled".into());
                        }
                        reporter.progress(f32::from(step) / 60.0);
                        reporter.detail("Reading headers and checksums");
                        std::thread::sleep(Duration::from_millis(18));
                    }
                    Ok("Imported 1 source".into())
                });
                self.show_toast("Import started in the background");
                Task::none()
            }
            Message::BuildPeaks => {
                self.jobs.spawn("Verify peak cache", |reporter| {
                    for step in 0..=40u16 {
                        if reporter.is_cancelled() {
                            return Err("cancelled".into());
                        }
                        reporter.progress(f32::from(step) / 40.0);
                        reporter.detail("Checking pyramid levels");
                        std::thread::sleep(Duration::from_millis(15));
                    }
                    Ok("Peak cache verified".into())
                });
                Task::none()
            }
            Message::QueueExport => {
                self.jobs.spawn_demo_export("Export demo mix");
                self.show_toast("Export queued");
                Task::none()
            }
            Message::CancelJob(id) => {
                self.jobs.cancel(id);
                Task::none()
            }
            Message::ClearJobs => {
                self.jobs.clear_finished();
                Task::none()
            }
            Message::DismissToast => {
                self.toast = None;
                Task::none()
            }
            Message::WindowDrag => {
                if let Some(id) = self.window {
                    window::drag(id)
                } else {
                    Task::none()
                }
            }
            Message::WindowMinimize => {
                if let Some(id) = self.window {
                    window::minimize::<Message>(id, true).map(|_| Message::Noop)
                } else {
                    Task::none()
                }
            }
            Message::WindowMaximize => {
                if let Some(id) = self.window {
                    window::toggle_maximize::<Message>(id).map(|_| Message::Noop)
                } else {
                    Task::none()
                }
            }
            Message::WindowClose => {
                self.save_workspace();
                iced::exit()
            }
            Message::Event(event) => self.handle_event(event),
            Message::SetGain(value) => {
                self.gain = value;
                Task::none()
            }
            Message::SetFadeIn(value) => {
                self.fade_in = value;
                Task::none()
            }
            Message::SetFadeOut(value) => {
                self.fade_out = value;
                Task::none()
            }
            Message::SelectMedia(index) => {
                self.selected_media = index.min(self.media.len().saturating_sub(1));
                Task::none()
            }
            Message::SelectEffect(index) => {
                self.selected_effect = index.min(self.effects.len().saturating_sub(1));
                Task::none()
            }
            Message::SelectVoice(voice) => {
                self.selected_voice = voice;
                Task::none()
            }
            Message::SelectTrack(index) => {
                self.selected_track = index.min(2);
                Task::none()
            }
            Message::ZoomWaveform(factor) => {
                self.waveform.zoom_by(factor);
                Task::none()
            }
            Message::SeekWaveform(secs) => {
                self.position_secs = secs.clamp(0.0, self.media_duration);
                self.waveform.set_playhead(self.position_secs);
                Task::none()
            }
            Message::Noop => Task::none(),
        }
    }

    fn run_command(&mut self, id: CommandId) -> Task<Message> {
        if self.palette.open && id != CommandId::TogglePalette {
            self.palette.hide();
        }
        match id {
            CommandId::TogglePalette => {
                if self.palette.open {
                    self.palette.hide();
                    Task::none()
                } else {
                    self.palette.show(&self.commands);
                    iced::widget::operation::focus::<Message>(self.palette_id.clone())
                }
            }
            CommandId::ToggleTheme => self.update(Message::ToggleTheme),
            CommandId::ToggleReduceMotion => self.update(Message::ToggleReduceMotion),
            CommandId::ResetLayout => self.update(Message::ResetLayout),
            CommandId::SaveLayout => self.update(Message::SaveLayout),
            CommandId::OpenPanel(kind) => {
                self.set_panel_open(kind, true);
                Task::none()
            }
            CommandId::ClosePanel(kind) => {
                self.set_panel_open(kind, false);
                Task::none()
            }
            CommandId::ImportMedia => self.update(Message::ImportMedia),
            CommandId::BuildPeaks => self.update(Message::BuildPeaks),
            CommandId::PlayPause => self.update(Message::PlayPause),
            CommandId::Stop => self.update(Message::Stop),
            CommandId::QueueExport => self.update(Message::QueueExport),
        }
    }

    fn handle_event(&mut self, event: iced::Event) -> Task<Message> {
        match event {
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                if self.palette.open {
                    match key {
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                            self.palette.hide();
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                            return self.update(Message::RunPaletteSelection);
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                            self.palette.move_selection(1);
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                            self.palette.move_selection(-1);
                        }
                        _ => {}
                    }
                    return Task::none();
                }
                if let Some(id) = command_for_key(modifiers, &key) {
                    return self.run_command(id);
                }
                if let iced::keyboard::Key::Character(value) = &key
                    && modifiers.control()
                    && !modifiers.shift()
                    && !modifiers.alt()
                    && let Some(index) = value
                        .chars()
                        .next()
                        .and_then(|character| character.to_digit(10))
                    && (1..=8).contains(&index)
                {
                    let kind = PanelKind::ALL[index as usize - 1];
                    self.set_panel_open(kind, true);
                }
                Task::none()
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                self.last_cursor = position;
                if let Some(drag) = self.drag {
                    match drag {
                        Drag::Panel { kind, grab } => {
                            self.panels.drag_to(kind, position, grab);
                        }
                        Drag::Resize { kind } => {
                            self.panels.resize_to(kind, position);
                        }
                    }
                }
                Task::none()
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(_)) => {
                if let Some(drag) = self.drag.take() {
                    let kind = match drag {
                        Drag::Panel { kind, .. } | Drag::Resize { kind } => kind,
                    };
                    self.panels.settle(kind);
                    self.save_workspace();
                }
                Task::none()
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let timeline = self.panels.panel(PanelKind::Timeline).rect;
                let over_timeline = self.last_cursor.x >= timeline.x
                    && self.last_cursor.x <= timeline.right()
                    && self.last_cursor.y >= timeline.y
                    && self.last_cursor.y <= timeline.bottom();
                if over_timeline {
                    let y = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y,
                        mouse::ScrollDelta::Pixels { y, .. } => y / 40.0,
                    };
                    self.waveform.zoom_by(if y > 0.0 { 0.8 } else { 1.25 });
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn apply_window_size(&mut self, size: Size) {
        if !self.layout_restored && !self.bounds_initialized {
            self.panels = PanelManager::default_for(size);
            self.bounds_initialized = true;
        } else {
            self.panels.set_bounds(size);
        }
    }

    fn set_panel_open(&mut self, kind: PanelKind, open: bool) {
        self.panels.set_open(kind, open);
        let reduce = self.reduce_motion;
        let toggle = self.panel_motion.entry(kind).or_insert_with(|| {
            Toggle::new(if open { 1.0 } else { 0.0 }).with_duration(MotionDuration::PANEL_MS)
        });
        toggle.set_reduce_motion(reduce);
        toggle.toggle(open);
        self.save_workspace();
    }

    fn save_workspace(&mut self) {
        let Some(path) = &self.save_path else {
            return;
        };
        let layout = PersistedWorkspace::new(
            self.theme_mode.as_str(),
            self.reduce_motion,
            self.glass.mode.as_str(),
            self.panels.snapshots(),
        );
        if let Err(error) = layout.save(path) {
            self.last_save_error = Some(error.to_string());
        } else {
            self.last_save_error = None;
        }
    }

    fn show_toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
    }

    /// Render the whole shell.
    pub fn view(&self) -> Element<'_, Message> {
        let colors = self.colors();
        let mut layers: Vec<Element<'_, Message>> = Vec::new();
        layers.push(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_theme| container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.58,
                        ..colors.background
                    })),
                    ..container::Style::default()
                })
                .into(),
        );

        for kind in self.visible_panels() {
            layers.push(self.panel_view(kind, colors));
        }

        layers.push(self.chrome_view(colors));

        if self.palette.open {
            let items: Vec<CommandItem<Message>> = self
                .palette
                .visible
                .iter()
                .filter_map(|index| self.commands.get(*index))
                .map(|entry| CommandItem {
                    title: entry.title.to_string(),
                    hint: if !entry.hint.is_empty() {
                        format!("{} · {}", entry.category, entry.hint)
                    } else if let Some(shortcut) = commands::shortcut(entry.id) {
                        shortcut.label()
                    } else {
                        entry.category.to_string()
                    },
                    message: Message::RunCommand(entry.id),
                })
                .collect();
            layers.push(overlays::command_palette(
                self.palette_id.clone(),
                &self.palette.query,
                &items,
                self.palette.selected,
                colors,
                Message::PaletteInput,
            ));
        }

        if let Some((message, _)) = &self.toast {
            layers.push(
                container(overlays::toast(
                    message.clone(),
                    colors,
                    Message::DismissToast,
                ))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(Spacing::XXL)
                .into(),
            );
        }

        stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn visible_panels(&self) -> Vec<PanelKind> {
        let mut kinds = self.panels.painter_order();
        for panel in self.panels.panels() {
            if !panel.open
                && self
                    .panel_motion
                    .get(&panel.kind)
                    .is_some_and(|toggle| toggle.value() > 0.01)
            {
                kinds.push(panel.kind);
            }
        }
        kinds
    }

    fn chrome_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let drag = mouse_area(
            row![
                text("◈").size(15.0).color(colors.accent),
                text("AUDIO LAB PRO")
                    .size(f32::from(TypeScale::CAPTION))
                    .font(fonts::semibold())
                    .color(colors.text),
                components::muted(
                    format!(
                        "{:.0} fps · {:.1} ms",
                        1000.0 / self.frame_ms.max(0.1),
                        self.frame_ms
                    ),
                    colors,
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .align_y(Alignment::Center),
        )
        .on_press(Message::WindowDrag);

        let mut panel_buttons = row![].spacing(f32::from(Spacing::XXS));
        for kind in PanelKind::ALL {
            panel_buttons = panel_buttons.push(components::with_tooltip(
                components::icon_button(kind.glyph(), colors, Some(Message::TogglePanel(kind))),
                kind.id(),
                colors,
            ));
        }

        let chrome = row![
            drag,
            Space::new().width(Length::Fill),
            panel_buttons,
            components::with_tooltip(
                components::button(self.glass.badge(), ButtonKind::Ghost, colors, None),
                self.glass.detail.clone(),
                colors,
            ),
            components::icon_button("◐", colors, Some(Message::ToggleTheme)),
            components::icon_button("≈", colors, Some(Message::ToggleReduceMotion)),
            components::icon_button("⌄", colors, Some(Message::WindowMinimize)),
            components::icon_button("□", colors, Some(Message::WindowMaximize)),
            components::icon_button("×", colors, Some(Message::WindowClose)),
        ]
        .spacing(f32::from(Spacing::XXS))
        .align_y(Alignment::Center);

        container(chrome)
            .width(Length::Fill)
            .height(Length::Fixed(f32::from(Control::CHROME)))
            .padding([Spacing::XS, Spacing::SM])
            .style(move |_theme| container::Style {
                background: Some(Background::Color(colors.glass_strong)),
                border: Border {
                    radius: 0.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                shadow: iced::Shadow::default(),
                text_color: Some(colors.text),
                snap: true,
            })
            .into()
    }

    fn panel_view(&self, kind: PanelKind, colors: ThemeColors) -> Element<'_, Message> {
        let panel = self.panels.panel(kind);
        let motion = self.panel_motion.get(&kind).map_or(
            PanelMotion {
                open: 1.0,
                offset: 0.0,
            },
            PanelMotion::from_open,
        );
        let focused = self.panels.active() == Some(kind);
        let offset = motion.offset;
        let height = if panel.collapsed {
            f32::from(Control::TITLE_BAR)
        } else {
            panel.rect.height
        };

        let drag = mouse_area(
            row![
                text(kind.glyph()).size(12.0).color(colors.faint),
                text(kind.title())
                    .size(f32::from(TypeScale::BODY_STRONG))
                    .font(fonts::semibold())
                    .color(colors.text),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center)
            .width(Length::Fill),
        )
        .on_press(Message::DragPanel(kind));

        let dock_glyph = if panel.docked.is_some() { "⇤" } else { "⇥" };
        let header = row![
            drag,
            components::icon_button(dock_glyph, colors, Some(Message::ToggleDock(kind))),
            components::icon_button(
                if panel.collapsed { "⌄" } else { "⌃" },
                colors,
                Some(Message::CollapsePanel(kind))
            ),
            components::icon_button("×", colors, Some(Message::ClosePanel(kind))),
        ]
        .spacing(f32::from(Spacing::XXS))
        .align_y(Alignment::Center)
        .padding([Spacing::XXS, Spacing::SM]);

        let body: Element<'_, Message> = if panel.collapsed {
            Space::new().height(Length::Fixed(0.0)).into()
        } else {
            container(self.panel_body(kind, colors))
                .padding([Spacing::XS, Spacing::MD])
                .height(Length::Fill)
                .clip(true)
                .into()
        };

        let panel_container = container(column![header, body].spacing(0.0))
            .width(Length::Fixed(panel.rect.width))
            .height(Length::Fixed(height))
            .style(move |_theme| colors.panel_style(focused));

        let content: Element<'_, Message> = if panel.collapsed || panel.docked.is_some() {
            panel_container.into()
        } else {
            stack![
                panel_container,
                container(
                    mouse_area(
                        container(text("◢").size(10.0).color(colors.faint))
                            .width(Length::Fixed(f32::from(Control::GRIP)))
                            .height(Length::Fixed(f32::from(Control::GRIP)))
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center),
                    )
                    .on_press(Message::ResizePanel(kind)),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::End)
                .padding(Spacing::XXS),
            ]
            .into()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                top: panel.rect.y + offset,
                left: panel.rect.x,
                right: 0.0,
                bottom: 0.0,
            })
            .into()
    }

    fn panel_body(&self, kind: PanelKind, colors: ThemeColors) -> Element<'_, Message> {
        match kind {
            PanelKind::Transport => self.transport_view(colors),
            PanelKind::Timeline => self.timeline_view(colors),
            PanelKind::Inspector => self.inspector_view(colors),
            PanelKind::MediaBin => self.media_view(colors),
            PanelKind::VoiceLab => self.voice_view(colors),
            PanelKind::Effects => self.effects_view(colors),
            PanelKind::Models => self.models_view(colors),
            PanelKind::ExportQueue => self.export_view(colors),
        }
    }

    fn transport_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let play_glyph = if self.playing { "⏸" } else { "▶" };
        column![
            row![
                components::icon_button("⏮", colors, Some(Message::Stop)),
                components::icon_button(play_glyph, colors, Some(Message::PlayPause)),
                components::icon_button("⏹", colors, Some(Message::Stop)),
                text(format!(
                    "{} / {}",
                    format_time(self.position_secs),
                    format_time(self.media_duration)
                ))
                .size(f32::from(TypeScale::BODY_STRONG))
                .font(fonts::medium())
                .color(colors.text),
                Space::new().width(Length::Fill),
                components::muted(format!("{:.0}% gain", self.gain * 100.0), colors),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
            components::progress(
                (self.position_secs / self.media_duration.max(1.0)).clamp(0.0, 1.0),
                colors
            ),
        ]
        .spacing(f32::from(Spacing::MD))
        .into()
    }

    fn timeline_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let x = self.waveform.visible();
        let waveform = container(self.waveform.view())
            .width(Length::Fill)
            .height(Length::Fixed(120.0))
            .style(move |_theme| colors.card_style());

        let mut tracks = column![].spacing(f32::from(Spacing::XXS));
        for (index, name) in ["Voice", "Music", "SFX"].iter().enumerate() {
            let selected = index == self.selected_track;
            let row = components::button(
                format!("{index:02}  {name}"),
                if selected {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                colors,
                Some(Message::SelectTrack(index)),
            );
            tracks = tracks.push(row);
        }

        column![
            row![
                components::icon_button("−", colors, Some(Message::ZoomWaveform(1.25))),
                components::icon_button("+", colors, Some(Message::ZoomWaveform(0.8))),
                components::icon_button("⟲", colors, Some(Message::SeekWaveform(0.0))),
                components::muted(format!("view {:.1}s → {:.1}s", x.0, x.1), colors),
                Space::new().width(Length::Fill),
                components::muted("Scroll over waveform to zoom", colors),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
            waveform,
            scrollable(tracks).height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    fn inspector_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let name = self
            .media
            .get(self.selected_media)
            .map_or("No clip selected", |item| item.name.as_str());
        scrollable(
            column![
                components::section_title("Clip inspector", colors),
                components::muted(name, colors),
                overlays::property_row(
                    "Gain",
                    components::slider_control(self.gain, Message::SetGain),
                    colors
                ),
                overlays::property_row(
                    "Fade in",
                    components::slider_control(self.fade_in, Message::SetFadeIn),
                    colors
                ),
                overlays::property_row(
                    "Fade out",
                    components::slider_control(self.fade_out, Message::SetFadeOut),
                    colors
                ),
                components::kicker("Effects", colors),
                for_each_effect(&self.effects, colors),
            ]
            .spacing(f32::from(Spacing::SM)),
        )
        .height(Length::Fill)
        .into()
    }

    fn media_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::XXS));
        for (index, item) in self.media.iter().enumerate() {
            let selected = index == self.selected_media;
            rows = rows.push(components::button(
                format!(
                    "{}  ·  {}  ·  {} ch",
                    item.name,
                    format_time(item.duration_secs),
                    item.channels
                ),
                if selected {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                colors,
                Some(Message::SelectMedia(index)),
            ));
        }
        column![
            row![
                components::button(
                    "Import",
                    ButtonKind::Primary,
                    colors,
                    Some(Message::ImportMedia)
                ),
                components::button(
                    "Rebuild peaks",
                    ButtonKind::Secondary,
                    colors,
                    Some(Message::BuildPeaks)
                ),
            ]
            .spacing(f32::from(Spacing::XS)),
            scrollable(rows).height(Length::Fill),
            components::muted(
                format!(
                    "{} source(s) · 48 kHz · {:.1}s",
                    self.media.len(),
                    self.media_duration
                ),
                colors
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    fn voice_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        column![
            components::section_title("Voice Lab", colors),
            components::muted("Local synthesis and cloning stubs", colors),
            components::dropdown(
                self.voices.clone(),
                Some(self.selected_voice.clone()),
                Message::SelectVoice
            ),
            components::button("Synthesize line", ButtonKind::Primary, colors, None),
            overlays::card(
                components::muted(
                    "The editor, transcript import and streaming preview land in Phase 3. This panel proves the glass, focus and dock behaviour.",
                    colors
                ),
                colors
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    fn effects_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::XXS));
        for (index, effect) in self.effects.iter().enumerate() {
            let selected = index == self.selected_effect;
            rows = rows.push(components::button(
                format!(
                    "{}  {}",
                    if effect.enabled { "●" } else { "○" },
                    effect.name
                ),
                if selected {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                colors,
                Some(Message::SelectEffect(index)),
            ));
        }
        column![
            row![
                components::section_title("Effects", colors),
                Space::new().width(Length::Fill),
                components::button("Add", ButtonKind::Secondary, colors, None),
            ]
            .align_y(Alignment::Center),
            scrollable(rows).height(Length::Fill),
            components::muted("Chain state is a Phase 2 stub.", colors),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    fn models_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::SM));
        for model in &self.models {
            rows = rows.push(overlays::card(
                row![
                    column![
                        text(model.name.clone())
                            .size(f32::from(TypeScale::BODY_STRONG))
                            .font(fonts::semibold())
                            .color(colors.text),
                        components::muted(format!("{} · {}", model.purpose, model.size), colors),
                    ]
                    .spacing(f32::from(Spacing::XXS)),
                    Space::new().width(Length::Fill),
                    components::muted(
                        if model.installed {
                            "Installed"
                        } else {
                            "Available"
                        },
                        colors
                    ),
                ]
                .align_y(Alignment::Center),
                colors,
            ));
        }
        column![
            row![
                components::section_title("Models", colors),
                Space::new().width(Length::Fill),
                components::muted("Local only", colors),
            ],
            scrollable(rows).height(Length::Fill),
            components::muted(self.glass.detail.clone(), colors),
            row![
                components::toggle(
                    "Reduce motion",
                    self.reduce_motion,
                    colors,
                    Message::ToggleReduceMotion
                ),
                components::button(
                    "Cycle theme",
                    ButtonKind::Secondary,
                    colors,
                    Some(Message::ToggleTheme)
                ),
            ]
            .spacing(f32::from(Spacing::MD))
            .align_y(Alignment::Center),
            components::muted(
                self.last_save_error
                    .clone()
                    .unwrap_or_else(|| "Workspace saves automatically".into()),
                colors
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    fn export_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::SM));
        if self.jobs.jobs().is_empty() {
            rows = rows.push(components::muted("No export jobs yet.", colors));
        }
        for job in self.jobs.jobs() {
            let status = match job.status {
                JobStatus::Running => format!("{:.0}%", job.progress * 100.0),
                JobStatus::Completed => "Done".into(),
                JobStatus::Cancelled => "Cancelled".into(),
                JobStatus::Failed => "Failed".into(),
            };
            rows = rows.push(overlays::card(
                column![
                    row![
                        text(job.name.clone())
                            .size(f32::from(TypeScale::BODY))
                            .font(fonts::medium())
                            .color(colors.text),
                        Space::new().width(Length::Fill),
                        components::muted(status, colors),
                        if job.status.is_running() {
                            components::button(
                                "Cancel",
                                ButtonKind::Ghost,
                                colors,
                                Some(Message::CancelJob(job.id)),
                            )
                        } else {
                            Space::new().width(Length::Fixed(0.0)).into()
                        },
                    ]
                    .align_y(Alignment::Center)
                    .spacing(f32::from(Spacing::XS)),
                    components::progress(job.progress, colors),
                    components::muted(job.detail.clone(), colors),
                ]
                .spacing(f32::from(Spacing::XS)),
                colors,
            ));
        }
        column![
            row![
                components::section_title("Export Queue", colors),
                components::muted(
                    if self.jobs.running_count() > 0 {
                        format!("{} running", self.jobs.running_count())
                    } else {
                        "idle".into()
                    },
                    colors
                ),
                Space::new().width(Length::Fill),
                components::button(
                    "Queue demo",
                    ButtonKind::Primary,
                    colors,
                    Some(Message::QueueExport)
                ),
                components::button(
                    "Clear done",
                    ButtonKind::Ghost,
                    colors,
                    Some(Message::ClearJobs)
                ),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
            scrollable(rows).height(Length::Fill),
            components::muted(
                if self.jobs.has_running() {
                    "A worker is running; the UI stays responsive."
                } else {
                    "Workers run outside the render loop."
                },
                colors
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
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

fn for_each_effect<'a>(effects: &'a [EffectItem], colors: ThemeColors) -> Element<'a, Message> {
    let mut rows = column![].spacing(f32::from(Spacing::XXS));
    for effect in effects {
        rows = rows.push(components::muted(
            format!(
                "{} · {}",
                if effect.enabled { "on" } else { "off" },
                effect.name
            ),
            colors,
        ));
    }
    rows.into()
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
