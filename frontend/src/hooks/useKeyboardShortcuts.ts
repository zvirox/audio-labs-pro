import { useEffect } from 'react';

interface KeyboardShortcutHandlers {
  onSelectWorkspace: (id: string) => void;
  onToggleCommandPalette: () => void;
  onToggleShortcuts: () => void;
  onToggleTheme: () => void;
  onToggleReduceMotion: () => void;
  onToggleInspector: () => void;
}

const WORKSPACE_KEYS: Record<string, string> = {
  '1': 'voice',
  '2': 'multitrack',
  '3': 'audiobook',
  '4': 'dubbing',
  '5': 'tools',
  '6': 'effects',
  '7': 'settings',
};

export function useKeyboardShortcuts({
  onSelectWorkspace,
  onToggleCommandPalette,
  onToggleShortcuts,
  onToggleTheme,
  onToggleReduceMotion,
  onToggleInspector,
}: KeyboardShortcutHandlers) {
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Don't intercept when user is typing in an input or textarea
      const target = e.target as HTMLElement;
      if (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
      ) {
        return;
      }

      const isCtrlOrMeta = e.ctrlKey || e.metaKey;

      // F1: Shortcuts modal
      if (e.key === 'F1') {
        e.preventDefault();
        onToggleShortcuts();
        return;
      }

      if (isCtrlOrMeta) {
        // Ctrl+K: Command Palette
        if (e.key.toLowerCase() === 'k') {
          e.preventDefault();
          onToggleCommandPalette();
          return;
        }

        // Ctrl+D: Cycle Theme
        if (e.key.toLowerCase() === 'd') {
          e.preventDefault();
          onToggleTheme();
          return;
        }

        // Ctrl+B: Toggle Right Inspector Drawer
        if (e.key.toLowerCase() === 'b') {
          e.preventDefault();
          onToggleInspector();
          return;
        }

        // Ctrl+Shift+M: Toggle Reduce Motion
        if (e.shiftKey && e.key.toLowerCase() === 'm') {
          e.preventDefault();
          onToggleReduceMotion();
          return;
        }

        // Ctrl+1..7: Workspace Navigation
        if (WORKSPACE_KEYS[e.key]) {
          e.preventDefault();
          onSelectWorkspace(WORKSPACE_KEYS[e.key]);
          return;
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [
    onSelectWorkspace,
    onToggleCommandPalette,
    onToggleShortcuts,
    onToggleTheme,
    onToggleReduceMotion,
    onToggleInspector,
  ]);
}
