//! Desktop application state container.
//!
//! Owns the live [`EditSession`], the persisted [`PersistedWorkspace`], and
//! the [`GlassReport`]. Access is synchronized via standard [`std::sync::Mutex`].

use std::sync::{Arc, Mutex};

use alp_session::{EditSession, PersistedWorkspace, Size};

use crate::glass::GlassReport;

/// Shared application state passed to Tauri command handlers.
#[derive(Clone)]
pub struct AppState {
    pub session: Arc<Mutex<EditSession>>,
    pub workspace: Arc<Mutex<PersistedWorkspace>>,
    pub glass: Arc<Mutex<GlassReport>>,
}

impl AppState {
    /// Create a new application state with demo session and persisted workspace.
    #[must_use]
    pub fn new_demo() -> Self {
        let workspace = PersistedWorkspace::load_or_default(Size::new(1280.0, 800.0));
        Self {
            session: Arc::new(Mutex::new(EditSession::demo())),
            workspace: Arc::new(Mutex::new(workspace)),
            glass: Arc::new(Mutex::new(GlassReport::pending())),
        }
    }
}
