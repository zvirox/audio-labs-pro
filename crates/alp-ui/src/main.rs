//! Audio Labs Pro desktop shell.
//!
//! Phase 2 ships the transparent glass window, floating panel framework,
//! component kit, command palette and GPU waveform. Media, voice and model
//! panels are deliberate stubs; the domain crates remain headless.

mod app;
mod commands;
mod demo;
mod glass;
mod jobs;
mod panels;
mod session;
mod workspace;

use iced::{Size, window};

fn main() -> iced::Result {
    iced::application(app::App::boot, app::App::update, app::App::view)
        .title("Audio Labs Pro")
        .window(window::Settings {
            size: Size::new(1440.0, 900.0),
            min_size: Some(Size::new(960.0, 640.0)),
            transparent: true,
            decorations: false,
            resizable: true,
            ..window::Settings::default()
        })
        .default_font(alp_ui_kit::fonts::regular())
        .font(alp_ui_kit::fonts::INTER_BYTES)
        .antialiasing(true)
        .subscription(app::App::subscription)
        .theme(app::App::theme)
        .run()
}
