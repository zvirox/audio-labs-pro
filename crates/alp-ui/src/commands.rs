//! Command registry, filtering and keyboard shortcuts.
//!
//! The registry is data, not closures, so the same entries drive the palette,
//! the shortcuts and the panel menu. `app` translates a [`CommandId`] into its
//! own message and keeps the palette state.

use iced::keyboard::{Key, Modifiers, key::Named};

use crate::panels::PanelKind;

/// Every executable shell command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    /// Open or close the palette.
    TogglePalette,
    /// Cycle dark → light → system.
    ToggleTheme,
    /// Toggle Reduce motion.
    ToggleReduceMotion,
    /// Reset panel geometry.
    ResetLayout,
    /// Save the workspace immediately.
    SaveLayout,
    /// Open a panel.
    OpenPanel(PanelKind),
    /// Close a panel.
    ClosePanel(PanelKind),
    /// Show the import flow (background job demo in Phase 2).
    ImportMedia,
    /// Build/refresh the peak waveform in a worker.
    BuildPeaks,
    /// Play/pause the demo transport.
    PlayPause,
    /// Stop and rewind.
    Stop,
    /// Queue a demo export worker.
    QueueExport,
}

/// A palette entry plus its shortcut hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandEntry {
    /// Executable id.
    pub id: CommandId,
    /// Display name.
    pub title: &'static str,
    /// Group.
    pub category: &'static str,
    /// Shortcut hint, empty when none.
    pub hint: &'static str,
}

impl CommandEntry {
    const fn new(
        id: CommandId,
        title: &'static str,
        category: &'static str,
        hint: &'static str,
    ) -> Self {
        Self {
            id,
            title,
            category,
            hint,
        }
    }
}

const PANELS: [(PanelKind, &str); 9] = [
    (PanelKind::Mixer, "Open Mixer"),
    (PanelKind::Transport, "Open Transport"),
    (PanelKind::Timeline, "Open Tracks / Timeline"),
    (PanelKind::Inspector, "Open Inspector"),
    (PanelKind::MediaBin, "Open Media Bin"),
    (PanelKind::VoiceLab, "Open Voice Lab"),
    (PanelKind::Effects, "Open Effects"),
    (PanelKind::Models, "Open Models"),
    (PanelKind::ExportQueue, "Open Export Queue"),
];

/// The static command list.
#[must_use]
pub fn registry() -> Vec<CommandEntry> {
    let mut entries = vec![
        CommandEntry::new(
            CommandId::TogglePalette,
            "Command Palette",
            "Shell",
            "Ctrl+K",
        ),
        CommandEntry::new(
            CommandId::ToggleTheme,
            "Toggle theme",
            "Appearance",
            "Ctrl+Shift+D",
        ),
        CommandEntry::new(
            CommandId::ToggleReduceMotion,
            "Toggle Reduce motion",
            "Accessibility",
            "Ctrl+Shift+M",
        ),
        CommandEntry::new(
            CommandId::ResetLayout,
            "Reset layout",
            "Layout",
            "Ctrl+Shift+R",
        ),
        CommandEntry::new(CommandId::SaveLayout, "Save layout", "Layout", "Ctrl+S"),
        CommandEntry::new(CommandId::ImportMedia, "Import media", "Media", "Ctrl+O"),
        CommandEntry::new(CommandId::BuildPeaks, "Rebuild waveform peaks", "Media", ""),
        CommandEntry::new(CommandId::PlayPause, "Play / pause", "Transport", "Space"),
        CommandEntry::new(CommandId::Stop, "Stop and rewind", "Transport", ""),
        CommandEntry::new(CommandId::QueueExport, "Queue demo export", "Export", ""),
    ];
    for (kind, title) in PANELS {
        entries.push(CommandEntry::new(
            CommandId::OpenPanel(kind),
            title,
            "Panels",
            "",
        ));
        entries.push(CommandEntry::new(
            CommandId::ClosePanel(kind),
            match kind {
                PanelKind::Mixer => "Close Mixer",
                PanelKind::Transport => "Close Transport",
                PanelKind::Timeline => "Close Tracks / Timeline",
                PanelKind::Inspector => "Close Inspector",
                PanelKind::MediaBin => "Close Media Bin",
                PanelKind::VoiceLab => "Close Voice Lab",
                PanelKind::Effects => "Close Effects",
                PanelKind::Models => "Close Models",
                PanelKind::ExportQueue => "Close Export Queue",
            },
            "Panels",
            "",
        ));
    }
    entries
}

/// Case-insensitive title/category filter.
#[must_use]
pub fn filter(entries: &[CommandEntry], query: &str) -> Vec<usize> {
    let needle = query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return (0..entries.len()).collect();
    }
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            entry.title.to_ascii_lowercase().contains(&needle)
                || entry.category.to_ascii_lowercase().contains(&needle)
        })
        .map(|(index, _)| index)
        .collect()
}

/// A keyboard chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    /// Control (or Command on macOS).
    pub ctrl: bool,
    /// Shift.
    pub shift: bool,
    /// Alt.
    pub alt: bool,
    /// Lowercase character or a named key such as `space`.
    pub key: &'static str,
}

impl Shortcut {
    const fn new(key: &'static str, ctrl: bool, shift: bool, alt: bool) -> Self {
        Self {
            ctrl,
            shift,
            alt,
            key,
        }
    }

    /// Match an iced key event, including exact modifier state.
    #[must_use]
    pub fn matches(self, modifiers: Modifiers, key: &Key) -> bool {
        let token = match key {
            Key::Character(value) => value.to_ascii_lowercase(),
            Key::Named(Named::Space) => "space".into(),
            Key::Named(Named::Enter) => "enter".into(),
            Key::Named(Named::Escape) => "escape".into(),
            Key::Named(Named::Tab) => "tab".into(),
            Key::Named(Named::ArrowUp) => "up".into(),
            Key::Named(Named::ArrowDown) => "down".into(),
            Key::Named(Named::Backspace) => "backspace".into(),
            Key::Named(Named::Delete) => "delete".into(),
            _ => return false,
        };
        self.ctrl == modifiers.control()
            && self.shift == modifiers.shift()
            && self.alt == modifiers.alt()
            && self.key == token
    }

    /// Display form for the palette hint.
    #[must_use]
    pub fn label(self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.alt {
            parts.push("Alt");
        }
        parts.push(match self.key {
            "space" => "Space",
            "escape" => "Esc",
            "enter" => "Enter",
            "up" => "↑",
            "down" => "↓",
            other => other,
        });
        parts.join("+")
    }
}

/// Shortcut table.
#[must_use]
pub fn shortcut(id: CommandId) -> Option<Shortcut> {
    Some(match id {
        CommandId::TogglePalette => Shortcut::new("k", true, false, false),
        CommandId::ToggleTheme => Shortcut::new("d", true, true, false),
        CommandId::ToggleReduceMotion => Shortcut::new("m", true, true, false),
        CommandId::ResetLayout => Shortcut::new("r", true, true, false),
        CommandId::SaveLayout => Shortcut::new("s", true, false, false),
        CommandId::ImportMedia => Shortcut::new("o", true, false, false),
        CommandId::PlayPause => Shortcut::new("space", false, false, false),
        _ => return None,
    })
}

/// Palette state.
#[derive(Debug, Clone, Default)]
pub struct PaletteState {
    /// Whether the overlay is visible.
    pub open: bool,
    /// Current text input.
    pub query: String,
    /// Index into the filtered list.
    pub selected: usize,
    /// Filtered registry indices.
    pub visible: Vec<usize>,
}

impl PaletteState {
    /// Closed empty palette.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Open and reset the query.
    pub fn show(&mut self, entries: &[CommandEntry]) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.visible = filter(entries, "");
    }

    /// Close and clear.
    pub fn hide(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
        self.visible.clear();
    }

    /// Replace the query and recompute selection.
    pub fn set_query(&mut self, query: &str, entries: &[CommandEntry]) {
        self.query = query.to_string();
        self.visible = filter(entries, &self.query);
        self.selected = 0;
    }

    /// Move the highlighted row, wrapping.
    pub fn move_selection(&mut self, delta: isize) {
        if self.visible.is_empty() {
            self.selected = 0;
            return;
        }
        let len = self.visible.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(len) as usize;
    }

    /// Selected command, if any.
    #[must_use]
    pub fn selected_command(&self, entries: &[CommandEntry]) -> Option<CommandEntry> {
        self.visible
            .get(self.selected)
            .and_then(|index| entries.get(*index))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_every_panel() {
        let entries = registry();
        for kind in PanelKind::ALL {
            assert!(
                entries
                    .iter()
                    .any(|entry| entry.id == CommandId::OpenPanel(kind))
            );
            assert!(
                entries
                    .iter()
                    .any(|entry| entry.id == CommandId::ClosePanel(kind))
            );
        }
    }

    #[test]
    fn filter_is_case_insensitive_and_matches_category() {
        let entries = registry();
        let matches = filter(&entries, "inspector");
        assert!(
            matches
                .iter()
                .any(|index| entries[*index].title.contains("Inspector"))
        );
        let matches = filter(&entries, "appearance");
        assert!(
            matches
                .iter()
                .any(|index| entries[*index].category == "Appearance")
        );
        assert_eq!(filter(&entries, "no-such-command").len(), 0);
    }

    #[test]
    fn palette_selection_wraps() {
        let entries = registry();
        let mut palette = PaletteState::new();
        palette.show(&entries);
        palette.move_selection(-1);
        assert_eq!(palette.selected, palette.visible.len() - 1);
        palette.move_selection(1);
        assert_eq!(palette.selected, 0);
        assert!(palette.selected_command(&entries).is_some());
    }

    #[test]
    fn shortcuts_require_exact_modifiers() {
        let shortcut = shortcut(CommandId::SaveLayout).expect("shortcut");
        let key = Key::Character("s".into());
        assert!(shortcut.matches(Modifiers::CTRL, &key));
        assert!(!shortcut.matches(Modifiers::CTRL | Modifiers::SHIFT, &key));
        assert!(!shortcut.matches(Modifiers::NONE, &key));
    }

    #[test]
    fn space_shortcut_matches_named_key() {
        let shortcut = shortcut(CommandId::PlayPause).expect("shortcut");
        assert!(shortcut.matches(Modifiers::NONE, &Key::Named(Named::Space)));
        assert!(!shortcut.matches(Modifiers::CTRL, &Key::Named(Named::Space)));
    }

    #[test]
    fn shortcut_labels_are_human_readable() {
        assert_eq!(
            shortcut(CommandId::TogglePalette).unwrap().label(),
            "Ctrl+k"
        );
        assert_eq!(shortcut(CommandId::PlayPause).unwrap().label(), "Space");
    }
}
