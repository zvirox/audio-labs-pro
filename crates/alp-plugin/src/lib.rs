#![allow(unsafe_code)]

//! Audio Labs Pro CLAP and LV2 audio plugin hosting engine.
//!
//! Provides discovery, dynamic C ABI loading, real-time audio block execution,
//! parameter automation, and state management for Linux audio plugins.

pub mod clap;
pub mod error;
pub mod host;
pub mod instance;
pub mod lv2;
pub mod scanner;
pub mod types;

pub use error::PluginError;
pub use host::{PluginHost, TrackPluginChain};
pub use instance::ActivePlugin;
pub use scanner::PluginScanner;
pub use types::{PluginCategory, PluginDescriptor, PluginFormat, PluginParameter, ProcessContext};
