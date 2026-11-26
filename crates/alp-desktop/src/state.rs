//! Desktop application state container.
//!
//! Owns the live [`EditSession`], the persisted [`PersistedWorkspace`], and
//! the [`GlassReport`]. Access is synchronized via standard [`std::sync::Mutex`].

use std::sync::{Arc, Mutex};

use alp_session::{AiControlManager, EditSession, PersistedWorkspace, Size, TransportController};

use crate::glass::GlassReport;
use crate::plugin_window::PluginWindowRegistry;

/// Shared application state passed to Tauri command handlers.
#[derive(Clone)]
pub struct AppState {
    pub session: Arc<Mutex<EditSession>>,
    pub workspace: Arc<Mutex<PersistedWorkspace>>,
    pub glass: Arc<Mutex<GlassReport>>,
    pub transport: Arc<Mutex<TransportController>>,
    pub ai: Arc<Mutex<AiControlManager>>,
    pub plugin_windows: Arc<Mutex<PluginWindowRegistry>>,
}

impl AppState {
    /// Create a new application state with demo session, transport, and persisted workspace.
    #[must_use]
    pub fn new_demo() -> Self {
        let workspace = PersistedWorkspace::load_or_default(Size::new(1280.0, 800.0));
        let session = EditSession::demo();
        let duration = session.duration_secs();
        let sample_rate = session.project().sample_rate;
        Self {
            session: Arc::new(Mutex::new(session)),
            workspace: Arc::new(Mutex::new(workspace)),
            glass: Arc::new(Mutex::new(GlassReport::pending())),
            transport: Arc::new(Mutex::new(TransportController::new(duration, sample_rate))),
            ai: Arc::new(Mutex::new(AiControlManager::default())),
            plugin_windows: Arc::new(Mutex::new(PluginWindowRegistry::new())),
        }
    }
}
