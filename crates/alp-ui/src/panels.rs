//! Floating panel model: geometry, z-order, snap, collapse and docking.
//!
//! The manager is pure state. `app` turns pointer and keyboard messages into
//! manager calls, and `workspace` serialises the result. Keeping geometry out
//! of the view makes snapping and layout restore testable without a display.

use iced::{Point, Size};
use serde::{Deserialize, Serialize};

use alp_ui_kit::Control;

/// Minimum floating width.
pub const MIN_WIDTH: f32 = 260.0;
/// Minimum floating height.
pub const MIN_HEIGHT: f32 = 150.0;
/// Distance at which a dragged panel snaps to a window edge.
pub const SNAP_PX: f32 = 22.0;

/// The eight core panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelKind {
    /// Mixer and master bus.
    Mixer,
    /// Transport controls.
    Transport,
    /// Tracks and timeline.
    Timeline,
    /// Clip/selection inspector.
    Inspector,
    /// Media bin.
    MediaBin,
    /// Voice lab.
    VoiceLab,
    /// Effects chain.
    Effects,
    /// Model manager.
    Models,
    /// Export queue.
    ExportQueue,
    /// AI activity and live attach control.
    AiActivity,
}

impl PanelKind {
    /// Stable order used for defaults and menus.
    pub const ALL: [Self; 10] = [
        Self::Transport,
        Self::Mixer,
        Self::Timeline,
        Self::Inspector,
        Self::MediaBin,
        Self::VoiceLab,
        Self::Effects,
        Self::Models,
        Self::ExportQueue,
        Self::AiActivity,
    ];

    /// Stable persisted id.
    #[must_use]
    #[allow(dead_code)]
    pub fn id(self) -> &'static str {
        match self {
            Self::Mixer => "mixer",
            Self::Transport => "transport",
            Self::Timeline => "timeline",
            Self::Inspector => "inspector",
            Self::MediaBin => "media_bin",
            Self::VoiceLab => "voice_lab",
            Self::Effects => "effects",
            Self::Models => "models",
            Self::ExportQueue => "export_queue",
            Self::AiActivity => "ai_activity",
        }
    }

    /// Panel title.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Mixer => "Mixer",
            Self::Transport => "Transport",
            Self::Timeline => "Tracks / Timeline",
            Self::Inspector => "Inspector",
            Self::MediaBin => "Media Bin",
            Self::VoiceLab => "Voice Lab",
            Self::Effects => "Effects",
            Self::Models => "Models",
            Self::ExportQueue => "Export Queue",
            Self::AiActivity => "AI Activity & Control",
        }
    }

    /// Short glyph used in compact menus.
    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Mixer => "∑",
            Self::Transport => "▶",
            Self::Timeline => "▤",
            Self::Inspector => "⚙",
            Self::MediaBin => "▣",
            Self::VoiceLab => "◉",
            Self::Effects => "✦",
            Self::Models => "◇",
            Self::ExportQueue => "⇩",
            Self::AiActivity => "⚡",
        }
    }

    fn default_rect(self, bounds: Size) -> PanelRect {
        let w = bounds.width.max(900.0);
        let h = bounds.height.max(620.0);
        match self {
            Self::Transport => {
                let left = 32.0 + 320.0 + 16.0;
                let available = (w - 332.0) - left - 16.0;
                let width = available.clamp(360.0, 720.0);
                PanelRect::new(left + (available - width) / 2.0, 72.0, width, 150.0)
            }
            Self::Timeline => PanelRect::new(32.0, h - 382.0, w - 64.0, 330.0),
            Self::Mixer => PanelRect::new(w - 684.0, 72.0, 320.0, 390.0),
            Self::Inspector => PanelRect::new(w - 332.0, 72.0, 300.0, 340.0),
            Self::MediaBin => PanelRect::new(32.0, 72.0, 320.0, 340.0),
            Self::VoiceLab => PanelRect::new(w / 2.0 - 280.0, h / 2.0 - 220.0, 560.0, 440.0),
            Self::Effects => PanelRect::new(w - 332.0, 430.0, 300.0, 260.0),
            Self::Models => PanelRect::new(w / 2.0 - 250.0, h / 2.0 - 190.0, 500.0, 380.0),
            Self::ExportQueue => PanelRect::new(w - 452.0, h - 332.0, 420.0, 260.0),
            Self::AiActivity => PanelRect::new(w - 452.0, 72.0, 420.0, 360.0),
        }
    }
}

/// A serialisable rectangle in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PanelRect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl PanelRect {
    /// Construct a rectangle.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Right edge.
    #[must_use]
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge.
    #[must_use]
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    /// Clamp the rectangle fully inside the window (shrinking if needed).
    #[must_use]
    pub fn clamp(self, bounds: Size, collapsed: bool) -> Self {
        let max_width = (bounds.width - 24.0).max(MIN_WIDTH);
        let min_height = if collapsed {
            f32::from(Control::TITLE_BAR)
        } else {
            MIN_HEIGHT
        };
        let max_height = (bounds.height - 48.0).max(min_height);
        let width = self.width.clamp(MIN_WIDTH, max_width);
        let height = self.height.clamp(min_height, max_height);
        let x = self.x.clamp(0.0, (bounds.width - width).max(0.0));
        let y = self.y.clamp(0.0, (bounds.height - height).max(0.0));
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Snap near an edge of the window.
    #[must_use]
    pub fn snapped(self, bounds: Size) -> Self {
        let mut rect = self;
        if rect.x.abs() <= SNAP_PX {
            rect.x = 0.0;
        }
        if (rect.right() - bounds.width).abs() <= SNAP_PX {
            rect.x = bounds.width - rect.width;
        }
        if rect.y.abs() <= SNAP_PX {
            rect.y = 0.0;
        }
        if (rect.bottom() - bounds.height).abs() <= SNAP_PX {
            rect.y = bounds.height - rect.height;
        }
        rect
    }
}

/// Docked side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockSide {
    /// Left edge.
    Left,
    /// Right edge.
    Right,
}

/// Persisted state for one panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelState {
    /// Which panel this is.
    pub kind: PanelKind,
    /// Floating geometry.
    pub rect: PanelRect,
    /// Higher paints later.
    pub z: u32,
    /// Header-only state.
    pub collapsed: bool,
    /// Docked side, if any.
    pub docked: Option<DockSide>,
    /// Visibility.
    pub open: bool,
}

impl PanelState {
    fn new(kind: PanelKind, rect: PanelRect, z: u32) -> Self {
        Self {
            kind,
            rect,
            z,
            collapsed: false,
            docked: None,
            open: true,
        }
    }
}

/// The complete floating-panel workspace.
#[derive(Debug, Clone)]
pub struct PanelManager {
    panels: Vec<PanelState>,
    active: Option<PanelKind>,
    bounds: Size,
    next_z: u32,
}

impl PanelManager {
    /// Build the default workspace for a window size.
    #[must_use]
    pub fn default_for(bounds: Size) -> Self {
        let mut panels = Vec::with_capacity(PanelKind::ALL.len());
        for (index, kind) in PanelKind::ALL.into_iter().enumerate() {
            let mut panel = PanelState::new(kind, kind.default_rect(bounds), index as u32 + 1);
            panel.open = matches!(
                kind,
                PanelKind::Transport
                    | PanelKind::Mixer
                    | PanelKind::Timeline
                    | PanelKind::Inspector
                    | PanelKind::MediaBin
            );
            panels.push(panel);
        }
        Self {
            panels,
            active: Some(PanelKind::Timeline),
            bounds,
            next_z: PanelKind::ALL.len() as u32 + 1,
        }
    }

    /// Update bounds and re-clamp every panel.
    pub fn set_bounds(&mut self, bounds: Size) {
        self.bounds = bounds;
        for panel in &mut self.panels {
            panel.rect = panel.rect.clamp(bounds, panel.collapsed);
            if let Some(side) = panel.docked {
                panel.rect = docked_rect(panel.rect, side, bounds);
            }
        }
    }

    /// All panel states in stable kind order.
    #[must_use]
    #[allow(dead_code)]
    pub fn panels(&self) -> &[PanelState] {
        &self.panels
    }

    /// One panel.
    #[must_use]
    pub fn panel(&self, kind: PanelKind) -> &PanelState {
        self.panels
            .iter()
            .find(|panel| panel.kind == kind)
            .expect("every panel kind exists")
    }

    /// Mutable one panel.
    pub fn panel_mut(&mut self, kind: PanelKind) -> &mut PanelState {
        self.panels
            .iter_mut()
            .find(|panel| panel.kind == kind)
            .expect("every panel kind exists")
    }

    /// The focused panel.
    #[must_use]
    #[allow(dead_code)]
    pub fn active(&self) -> Option<PanelKind> {
        self.active
    }

    /// Set visibility and focus when opening.
    pub fn set_open(&mut self, kind: PanelKind, open: bool) {
        self.panel_mut(kind).open = open;
        if open {
            self.focus(kind);
        } else if self.active == Some(kind) {
            self.active = None;
        }
    }

    /// Toggle visibility.
    #[allow(dead_code)]
    pub fn toggle_open(&mut self, kind: PanelKind) {
        let open = !self.panel(kind).open;
        self.set_open(kind, open);
    }

    /// Raise and focus a panel.
    pub fn focus(&mut self, kind: PanelKind) {
        let z = self.next_z;
        self.next_z = self.next_z.saturating_add(1);
        let panel = self.panel_mut(kind);
        panel.z = z;
        panel.open = true;
        self.active = Some(kind);
    }

    /// Toggle collapsed.
    pub fn toggle_collapsed(&mut self, kind: PanelKind) {
        let collapsed = !self.panel(kind).collapsed;
        let panel = self.panel_mut(kind);
        panel.collapsed = collapsed;
        if collapsed && !panel.docked.is_some() {
            panel.rect.height = f32::from(Control::TITLE_BAR);
        } else if !collapsed {
            let height = panel.rect.height.max(MIN_HEIGHT);
            panel.rect.height = height;
        }
        self.focus(kind);
    }

    /// Drag a panel to a cursor position with a grab offset.
    pub fn drag_to(&mut self, kind: PanelKind, cursor: Point, grab: Point) {
        let bounds = self.bounds;
        let panel = self.panel_mut(kind);
        panel.rect.x = cursor.x - grab.x;
        panel.rect.y = cursor.y - grab.y;
        panel.rect = panel.rect.clamp(bounds, panel.collapsed);
    }

    /// Resize from the bottom-right grip.
    pub fn resize_to(&mut self, kind: PanelKind, cursor: Point) {
        let bounds = self.bounds;
        let panel = self.panel_mut(kind);
        panel.rect.width = (cursor.x - panel.rect.x).max(MIN_WIDTH);
        panel.rect.height = (cursor.y - panel.rect.y).max(MIN_HEIGHT);
        panel.rect = panel.rect.clamp(bounds, panel.collapsed);
    }

    /// Snap the panel after a drag/resize and focus it.
    pub fn settle(&mut self, kind: PanelKind) {
        let bounds = self.bounds;
        let panel = self.panel_mut(kind);
        panel.rect = panel.rect.snapped(bounds).clamp(bounds, panel.collapsed);
        self.focus(kind);
    }

    /// Dock to a side.
    pub fn dock(&mut self, kind: PanelKind, side: DockSide) {
        let bounds = self.bounds;
        let panel = self.panel_mut(kind);
        panel.docked = Some(side);
        panel.collapsed = false;
        panel.rect = docked_rect(panel.rect, side, bounds);
        self.focus(kind);
    }

    /// Return a docked panel to a floating position.
    pub fn undock(&mut self, kind: PanelKind) {
        let bounds = self.bounds;
        let panel = self.panel_mut(kind);
        let was_right = panel.docked == Some(DockSide::Right);
        panel.docked = None;
        panel.rect.width = panel.rect.width.max(320.0);
        panel.rect.x = if was_right {
            (bounds.width - panel.rect.width - 32.0).max(0.0)
        } else {
            32.0
        };
        panel.rect.y = 96.0;
        panel.rect = panel.rect.clamp(bounds, panel.collapsed);
        self.focus(kind);
    }

    /// Restore the default geometry while keeping visibility.
    pub fn reset_geometry(&mut self) {
        let bounds = self.bounds;
        for panel in &mut self.panels {
            panel.rect = panel.kind.default_rect(bounds);
            panel.docked = None;
            panel.collapsed = false;
        }
        self.active = Some(PanelKind::Timeline);
    }

    /// Panels sorted painter-first (lowest z first).
    #[must_use]
    #[allow(dead_code)]
    pub fn painter_order(&self) -> Vec<PanelKind> {
        let mut panels: Vec<&PanelState> = self.panels.iter().filter(|panel| panel.open).collect();
        panels.sort_by_key(|panel| (panel.z, panel.kind));
        panels.into_iter().map(|panel| panel.kind).collect()
    }

    /// Replace geometry/state from a persisted workspace.
    pub fn restore(&mut self, snapshots: &[PanelState]) {
        let bounds = self.bounds;
        for snapshot in snapshots {
            if !PanelKind::ALL.contains(&snapshot.kind) {
                continue;
            }
            let panel = self.panel_mut(snapshot.kind);
            panel.rect = snapshot.rect.clamp(bounds, snapshot.collapsed);
            panel.z = snapshot.z;
            panel.collapsed = snapshot.collapsed;
            panel.docked = snapshot.docked;
            panel.open = snapshot.open;
            if let Some(side) = panel.docked {
                panel.rect = docked_rect(panel.rect, side, bounds);
            }
            self.next_z = self.next_z.max(snapshot.z.saturating_add(1));
        }
        self.active = self
            .panels
            .iter()
            .filter(|panel| panel.open)
            .max_by_key(|panel| panel.z)
            .map(|panel| panel.kind);
    }

    /// Persisted snapshots.
    #[must_use]
    pub fn snapshots(&self) -> Vec<PanelState> {
        self.panels.clone()
    }
}

fn docked_rect(mut rect: PanelRect, side: DockSide, bounds: Size) -> PanelRect {
    rect.width = rect.width.clamp(300.0, 440.0);
    rect.x = match side {
        DockSide::Left => 0.0,
        DockSide::Right => bounds.width - rect.width,
    };
    rect.y = f32::from(Control::CHROME);
    rect.height = (bounds.height - rect.y).max(MIN_HEIGHT);
    rect
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> PanelManager {
        PanelManager::default_for(Size::new(1440.0, 900.0))
    }

    #[test]
    fn every_core_panel_exists_with_four_visible() {
        let manager = manager();
        assert_eq!(manager.panels().len(), PanelKind::ALL.len());
        assert_eq!(
            manager.panels().iter().filter(|panel| panel.open).count(),
            5
        );
        assert_eq!(manager.active(), Some(PanelKind::Timeline));
    }

    #[test]
    fn default_layout_keeps_the_top_row_apart_on_a_narrow_window() {
        let manager = PanelManager::default_for(Size::new(1163.0, 821.0));
        let transport = manager.panel(PanelKind::Transport).rect;
        let inspector = manager.panel(PanelKind::Inspector).rect;
        let media = manager.panel(PanelKind::MediaBin).rect;
        assert!(transport.right() < inspector.x);
        assert!(media.right() < transport.x);
        assert!(transport.bottom() < manager.panel(PanelKind::Timeline).rect.y);
    }

    #[test]
    fn drag_snaps_to_left_and_top_edges() {
        let mut manager = manager();
        manager.drag_to(
            PanelKind::Inspector,
            Point::new(10.0, 8.0),
            Point::new(30.0, 20.0),
        );
        manager.settle(PanelKind::Inspector);
        let rect = manager.panel(PanelKind::Inspector).rect;
        assert_eq!(rect.x, 0.0);
        assert_eq!(rect.y, 0.0);
    }

    #[test]
    fn focus_raises_z_order() {
        let mut manager = manager();
        manager.focus(PanelKind::MediaBin);
        let order = manager.painter_order();
        assert_eq!(order.last(), Some(&PanelKind::MediaBin));
    }

    #[test]
    fn collapse_keeps_only_the_title_bar() {
        let mut manager = manager();
        manager.toggle_collapsed(PanelKind::Effects);
        assert!(manager.panel(PanelKind::Effects).collapsed);
        assert_eq!(
            manager.panel(PanelKind::Effects).rect.height,
            f32::from(Control::TITLE_BAR)
        );
    }

    #[test]
    fn dock_and_undock_move_between_edges() {
        let mut manager = manager();
        manager.dock(PanelKind::MediaBin, DockSide::Left);
        assert_eq!(manager.panel(PanelKind::MediaBin).rect.x, 0.0);
        assert_eq!(
            manager.panel(PanelKind::MediaBin).docked,
            Some(DockSide::Left)
        );
        manager.undock(PanelKind::MediaBin);
        assert_eq!(manager.panel(PanelKind::MediaBin).docked, None);
        assert!(manager.panel(PanelKind::MediaBin).rect.x > 0.0);
    }

    #[test]
    fn resize_respects_minimum_size() {
        let mut manager = manager();
        manager.resize_to(PanelKind::Timeline, Point::new(10.0, 10.0));
        let rect = manager.panel(PanelKind::Timeline).rect;
        assert!(rect.width >= MIN_WIDTH);
        assert!(rect.height >= MIN_HEIGHT);
    }

    #[test]
    fn restore_clamps_out_of_bounds_geometry() {
        let mut manager = manager();
        let mut snapshot = manager.snapshots();
        let inspector = snapshot
            .iter_mut()
            .find(|panel| panel.kind == PanelKind::Inspector)
            .expect("inspector");
        inspector.rect = PanelRect::new(9_000.0, 9_000.0, 10.0, 10.0);
        manager.restore(&snapshot);
        let rect = manager.panel(PanelKind::Inspector).rect;
        assert!(rect.x <= 1440.0);
        assert!(rect.width >= MIN_WIDTH);
    }
}
