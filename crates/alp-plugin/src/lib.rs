#![allow(unsafe_code)]

//! Audio Labs Pro CLAP, LV2, and VST3 audio plugin hosting engine.
//!
//! Provides discovery, dynamic ABI loading (CLAP/LV2 C ABI and VST3 COM ABI),
//! real-time audio block execution, parameter automation, and state management
//! for Linux audio plugins.

pub mod clap;
pub mod error;
pub mod host;
pub mod instance;
pub mod lv2;
pub mod scanner;
pub mod types;
pub mod vst3;

pub use error::PluginError;
pub use host::{PluginHost, TrackPluginChain};
pub use instance::ActivePlugin;
pub use scanner::PluginScanner;
pub use types::{PluginCategory, PluginDescriptor, PluginFormat, PluginParameter, ProcessContext};
pub use vst3::{ActiveVst3Instance, scan_vst3_bundle};
