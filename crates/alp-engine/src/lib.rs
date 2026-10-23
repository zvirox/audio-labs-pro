//! `alp-engine` — real-time playback, capture, transport and offline mixdown.
//!
//! The engine owns the only audio-thread code in the application. Everything it
//! touches during a callback is prepared in advance; control messages arrive over
//! a lock-free SPSC channel. See `docs/modules/alp-engine.md`.

mod capture;
mod command;
mod device;
mod error;
mod mixer;
mod render;
mod scene;
mod transport;

pub use capture::{
    CaptureConfig, CaptureReport, InputMeter, Recording, record_to_wav, recover_capture,
    start_recording,
};
pub use command::{CommandReceiver, CommandSender, EngineCommand, command_channel};
pub use device::{
    DeviceDirection, DeviceInfo, PlaybackHandle, PlaybackMeter, PlaybackReport, list_devices,
    play_scene, play_scene_live,
};
pub use error::EngineError;
pub use mixer::{Mixer, TrackMeter, TrackMeterBank};
pub use render::{DEFAULT_BLOCK_FRAMES, RenderReport, render_scene, render_scene_to_wav};
pub use scene::{MixScene, SceneClip, SourceAudio};
pub use transport::{Transport, TransportState};
