//! Phase 0 UI spike for iced 0.14: transparent window, glass-style floating
//! panel, 1-hour waveform scrolling, component set and motion.
//!
//! Run:
//!   cargo run -p ui-iced-spike --release
//! Env:
//!   ALP_SPIKE_AUTOCLOSE_SECS=15  close automatically (used for measurements)
//!   ALP_SPIKE_METRICS=1          print frames/s and process CPU% once a second
//!
//! `window::frames()` is subscribed only while something is animating. A
//! permanent frames subscription would itself request a redraw per message and
//! keep the app at 60 fps forever (measured), which is exactly the idle-CPU
//! behaviour the plan forbids.

mod components;
mod glass;
mod motion;
mod waveform;

use std::time::{Duration, Instant};

use iced::widget::{Space, column, container, mouse_area, row, stack, text};
use iced::{
    Color, Element, Font, Length, Point, Subscription, Task, Theme, alignment, mouse, window,
};

use waveform::Waveform;

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title("Audio Labs Pro — Phase 0 UI spike")
        .transparent(true)
        .decorations(false)
        .antialiasing(true)
        .default_font(Font {
            family: iced::font::Family::Name("Inter"),
            ..Font::DEFAULT
        })
        .subscription(App::subscription)
        .theme(|_: &App| Theme::Dark)
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Glass,
    Waveform,
    Components,
}

#[derive(Debug, Clone, Copy)]
enum Drag {
    Panel { grab: Point },
    Resize,
}

struct App {
    tab: Tab,
    panel: Panel,
    drag: Option<Drag>,
    waveform: Waveform,
    auto_scroll: bool,
    reduce_motion: bool,
    /// Draw the in-app faux desktop (default) or leave the window transparent
    /// so the compositor/desktop shows through.
    backdrop: bool,

    panel_open: motion::Toggle,
    toast: motion::Toggle,
    modal: motion::Toggle,
    palette: motion::Toggle,

    last_cursor: Point,
    last_frame: Option<Instant>,
    frame_ms: f32,
    frames: u32,
    frames_since_report: u32,

    cpu_last: (u64, Instant),
    cpu_percent: f32,

    autoclose: Option<Instant>,
    metrics: bool,
}

#[derive(Debug, Clone)]
struct Panel {
    position: Point,
    size: iced::Size,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            position: Point::new(96.0, 72.0),
            size: iced::Size::new(880.0, 520.0),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Frame(Instant),
    Event(iced::Event),
    Tick,
    Heartbeat,
    Tab(Tab),
    WindowDrag,
    ResizeStart,
    ToggleAutoScroll,
    ToggleReduceMotion,
    TogglePanel,
    Toast,
    ToggleModal,
    TogglePalette,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let env_secs = std::env::var("ALP_SPIKE_AUTOCLOSE_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok());

        let tab = match std::env::var("ALP_SPIKE_START_TAB").as_deref() {
            Ok("waveform") => Tab::Waveform,
            Ok("components") => Tab::Components,
            _ => Tab::Glass,
        };
        let auto_scroll = std::env::var("ALP_SPIKE_AUTOSCROLL").is_ok();

        let started = Instant::now();
        let waveform = Waveform::synthetic_hour();
        eprintln!(
            "peak pyramid built in {:?} ({} buckets, {} bytes)",
            started.elapsed(),
            waveform.buckets(),
            waveform.bytes()
        );

        (
            Self {
                tab,
                panel: Panel::default(),
                drag: None,
                waveform,
                auto_scroll,
                reduce_motion: false,
                backdrop: std::env::var("ALP_SPIKE_TRANSPARENT").is_err(),
                panel_open: motion::Toggle::new(1.0).with_duration(motion::PANEL_MS),
                toast: motion::Toggle::new(0.0).with_duration(motion::FAST_MS),
                modal: motion::Toggle::new(0.0).with_duration(motion::BASE_MS),
                palette: motion::Toggle::new(0.0).with_duration(motion::BASE_MS),
                last_cursor: Point::ORIGIN,
                last_frame: None,
                frame_ms: 0.0,
                frames: 0,
                frames_since_report: 0,
                cpu_last: (process_cpu_ticks(), Instant::now()),
                cpu_percent: 0.0,
                autoclose: env_secs.map(|s| Instant::now() + Duration::from_secs(s)),
                metrics: std::env::var("ALP_SPIKE_METRICS").is_ok(),
            },
            Task::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![
            iced::event::listen().map(Message::Event),
            iced::time::every(Duration::from_secs(1)).map(|_| Message::Heartbeat),
        ];

        if self.auto_scroll || self.animating() {
            subs.push(window::frames().map(Message::Frame));
            subs.push(iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick));
        }

        Subscription::batch(subs)
    }

    fn animating(&self) -> bool {
        self.panel_open.is_animating()
            || self.toast.is_animating()
            || self.modal.is_animating()
            || self.palette.is_animating()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Frame(now) => {
                if let Some(last) = self.last_frame {
                    let dt = now.duration_since(last).as_secs_f32() * 1000.0;
                    self.frame_ms = self.frame_ms * 0.9 + dt * 0.1;
                }
                self.last_frame = Some(now);
                self.frames += 1;
                self.frames_since_report += 1;
                Task::none()
            }
            Message::Tick => {
                let now = Instant::now();
                self.panel_open.tick(now);
                self.toast.tick(now);
                self.modal.tick(now);
                self.palette.tick(now);

                if self.auto_scroll {
                    self.waveform.scroll_by(0.35);
                }

                Task::none()
            }
            Message::Heartbeat => {
                let cpu = self.sample_cpu();
                if self.metrics {
                    eprintln!(
                        "frames/s={} frame_ms={:.2} cpu={:.1}% animating={}",
                        self.frames_since_report,
                        self.frame_ms,
                        cpu,
                        self.animating() || self.auto_scroll
                    );
                    self.frames_since_report = 0;
                }

                if self
                    .autoclose
                    .is_some_and(|deadline| Instant::now() >= deadline)
                {
                    eprintln!(
                        "METRICS total_frames={} frame_ms={:.2} cpu={:.1}%",
                        self.frames, self.frame_ms, cpu
                    );
                    return iced::exit();
                }

                Task::none()
            }
            Message::Event(iced::Event::Mouse(mouse::Event::CursorMoved { position })) => {
                self.last_cursor = position;
                match self.drag {
                    Some(Drag::Panel { grab }) => {
                        self.panel.position = Point::new(position.x - grab.x, position.y - grab.y);
                    }
                    Some(Drag::Resize) => {
                        self.panel.size = iced::Size::new(
                            (position.x - self.panel.position.x).max(360.0),
                            (position.y - self.panel.position.y).max(220.0),
                        );
                    }
                    None => {}
                }
                Task::none()
            }
            Message::Event(iced::Event::Mouse(mouse::Event::ButtonReleased(_))) => {
                self.drag = None;
                Task::none()
            }
            Message::Event(iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key,
                ..
            })) => {
                if matches!(
                    key,
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
                ) && self.palette.is_on()
                {
                    self.palette.toggle(false);
                }
                Task::none()
            }
            Message::Event(_) => Task::none(),
            Message::Tab(tab) => {
                self.tab = tab;
                Task::none()
            }
            Message::WindowDrag => {
                // In-window panel drag (a single transparent OS window).
                self.drag = Some(Drag::Panel {
                    grab: Point::new(
                        self.last_cursor.x - self.panel.position.x,
                        self.last_cursor.y - self.panel.position.y,
                    ),
                });
                Task::none()
            }
            Message::ResizeStart => {
                self.drag = Some(Drag::Resize);
                Task::none()
            }
            Message::ToggleAutoScroll => {
                self.auto_scroll = !self.auto_scroll;
                Task::none()
            }
            Message::ToggleReduceMotion => {
                self.reduce_motion = !self.reduce_motion;
                self.panel_open.set_reduce_motion(self.reduce_motion);
                self.toast.set_reduce_motion(self.reduce_motion);
                self.modal.set_reduce_motion(self.reduce_motion);
                self.palette.set_reduce_motion(self.reduce_motion);
                Task::none()
            }
            Message::TogglePanel => {
                self.panel_open.toggle(!self.panel_open.is_on());
                Task::none()
            }
            Message::Toast => {
                let on = !self.toast.is_on();
                self.toast.toggle(on);
                Task::none()
            }
            Message::ToggleModal => {
                self.modal.toggle(!self.modal.is_on());
                Task::none()
            }
            Message::TogglePalette => {
                self.palette.toggle(!self.palette.is_on());
                Task::none()
            }
        }
    }

    fn sample_cpu(&mut self) -> f32 {
        let ticks = process_cpu_ticks();
        let now = Instant::now();
        let dt = now.duration_since(self.cpu_last.1).as_secs_f32();
        let dt_ticks = (ticks.saturating_sub(self.cpu_last.0)) as f32;
        self.cpu_last = (ticks, now);

        if dt > 0.0 {
            // USER_HZ is 100 on Linux.
            self.cpu_percent = (dt_ticks / 100.0) / dt * 100.0;
        }
        self.cpu_percent
    }

    fn view(&self) -> Element<'_, Message> {
        let mut layers: Vec<Element<'_, Message>> = Vec::new();
        if self.backdrop {
            layers.push(glass::backdrop());
        }

        if self.panel_open.value() > 0.01 {
            layers.push(self.panel_view());
        }

        if self.toast.value() > 0.01 {
            layers.push(components::toast(self.toast.value()));
        }

        if self.modal.value() > 0.01 {
            layers.push(components::modal(self.modal.value()));
        }

        if self.palette.value() > 0.01 {
            layers.push(components::palette(self.palette.value()));
        }

        if self.panel_open.value() <= 0.01 {
            layers.push(
                container(components::primary("Open panel", Message::TogglePanel))
                    .padding(24)
                    .align_x(alignment::Horizontal::Left)
                    .align_y(alignment::Vertical::Top)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
            );
        }

        stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn panel_view(&self) -> Element<'_, Message> {
        let header = mouse_area(
            row![
                text("AUDIO LAB PRO").size(13).font(semibold()),
                Space::new().width(Length::Fill),
                text(format!(
                    "{:.0} fps · {:.1} ms",
                    1000.0 / self.frame_ms.max(0.1),
                    self.frame_ms
                ))
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
            ]
            .align_y(alignment::Vertical::Center),
        )
        .on_press(Message::WindowDrag);

        let tabs = row![
            components::tab_button("Glass", self.tab == Tab::Glass, Message::Tab(Tab::Glass)),
            components::tab_button(
                "Waveform",
                self.tab == Tab::Waveform,
                Message::Tab(Tab::Waveform)
            ),
            components::tab_button(
                "Components",
                self.tab == Tab::Components,
                Message::Tab(Tab::Components)
            ),
        ]
        .spacing(6);

        let body: Element<'_, Message> = match self.tab {
            Tab::Glass => self.glass_tab(),
            Tab::Waveform => self.waveform_tab(),
            Tab::Components => components::showcase(
                self.reduce_motion,
                self.toast.is_on(),
                self.modal.is_on(),
                self.palette.is_on(),
            ),
        };

        let panel = container(column![header, tabs, body].spacing(14))
            .width(Length::Fixed(self.panel.size.width))
            .height(Length::Fixed(self.panel.size.height))
            .padding(18)
            .style(|_theme| glass::panel_style());

        let grip = mouse_area(
            container(
                text("⋰")
                    .size(12)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.35)),
            )
            .width(18)
            .height(18)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
        )
        .on_press(Message::ResizeStart);

        let panel = stack![
            panel,
            container(grip)
                .align_x(alignment::Horizontal::Right)
                .align_y(alignment::Vertical::Bottom)
                .padding(6)
        ];

        let offset = (1.0 - self.panel_open.value()) * 14.0;

        container(panel)
            .padding(iced::Padding {
                top: self.panel.position.y + offset,
                left: self.panel.position.x,
                ..iced::Padding::ZERO
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn glass_tab(&self) -> Element<'_, Message> {
        let status = glass::BlurSupport::detect();

        column![
            text("Frosted glass fallback").size(20),
            text(status.summary())
                .size(13)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.65)),
            row![
                components::primary("Show toast", Message::Toast),
                components::ghost(
                    if self.reduce_motion {
                        "Reduce motion: on"
                    } else {
                        "Reduce motion: off"
                    },
                    Message::ToggleReduceMotion
                ),
                components::ghost("Open modal", Message::ToggleModal),
                components::ghost("Command palette", Message::TogglePalette),
                components::ghost("Close panel", Message::TogglePanel),
            ]
            .spacing(8),
            text(
                "Panels are drawn inside one transparent window. When the compositor \
                 cannot blur, the panel keeps a translucent tint plus an in-app frost \
                 gradient so contrast and legibility survive."
            )
            .size(13)
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.55)),
        ]
        .spacing(14)
        .into()
    }

    fn waveform_tab(&self) -> Element<'_, Message> {
        column![
            row![
                components::primary(
                    if self.auto_scroll {
                        "Stop scrolling"
                    } else {
                        "Scroll at 60 fps"
                    },
                    Message::ToggleAutoScroll
                ),
                text(format!(
                    "view {:.1}s → {:.1}s",
                    self.waveform.scroll_secs,
                    self.waveform.scroll_secs + self.waveform.visible_secs
                ))
                .size(12),
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Center),
            container(self.waveform.view()).height(Length::Fill),
        ]
        .spacing(12)
        .into()
    }
}

fn semibold() -> Font {
    Font {
        family: iced::font::Family::Name("Inter"),
        weight: iced::font::Weight::Semibold,
        ..Font::DEFAULT
    }
}

/// Total CPU ticks (user + system) for this process, from `/proc/self/stat`.
fn process_cpu_ticks() -> u64 {
    let Ok(stat) = std::fs::read_to_string("/proc/self/stat") else {
        return 0;
    };
    let Some(rest) = stat.rsplit_once(") ").map(|(_, rest)| rest) else {
        return 0;
    };
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // After the comm field, index 11 = utime, 12 = stime.
    let utime = fields
        .get(11)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let stime = fields
        .get(12)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    utime + stime
}
