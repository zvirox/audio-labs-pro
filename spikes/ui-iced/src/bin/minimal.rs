//! Minimal iced 0.14 window used to isolate idle redraw behaviour.

use std::time::{Duration, Instant};

use iced::widget::text;
use iced::{Element, Subscription, Task};

fn boot() -> State {
    State {
        frames: 0,
        reported: 0,
        started: Instant::now(),
    }
}

fn main() -> iced::Result {
    iced::application(boot, update, view)
        .title("minimal")
        .window_size((600.0, 300.0))
        .subscription(subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Frame,
    Heartbeat,
}

struct State {
    frames: u32,
    reported: u32,
    started: Instant,
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Frame => {
            state.frames += 1;
        }
        Message::Heartbeat => {
            eprintln!("frames/s={}", state.frames - state.reported);
            state.reported = state.frames;
            if state.started.elapsed() > Duration::from_secs(8) {
                return iced::exit();
            }
        }
    }
    Task::none()
}

fn subscription(_state: &State) -> Subscription<Message> {
    Subscription::batch(vec![
        iced::window::frames().map(|_| Message::Frame),
        iced::time::every(Duration::from_secs(1)).map(|_| Message::Heartbeat),
    ])
}

fn view(_state: &State) -> Element<'_, Message> {
    text("idle redraw test").into()
}
