import { useEffect } from 'react';
import { Command } from 'cmdk';
import {
  Mic,
  Layers,
  BookOpen,
  Film,
  Wrench,
  Sparkles,
  Settings,
  Sliders,
  Play,
  RotateCcw,
  Sun,
  Moon,
  Zap,
  HelpCircle,
  Eye,
  Home,
} from 'lucide-react';
import type { PanelKind } from '../ipc/types';

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  onSelectWorkspace: (id: string) => void;
  onTogglePanel: (kind: PanelKind) => void;
  onResetLayout: () => void;
  onToggleTheme: () => void;
  onOpenShortcuts: () => void;
  onTriggerEdit: () => void;
  theme: 'dark' | 'light';
}

export function CommandPalette({
  open,
  onClose,
  onSelectWorkspace,
  onTogglePanel,
  onResetLayout,
  onToggleTheme,
  onOpenShortcuts,
  onTriggerEdit,
  theme,
}: CommandPaletteProps) {
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault();
        if (open) onClose();
      }
      if (e.key === 'Escape' && open) {
        onClose();
      }
    }
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-20 p-4 bg-black/60 backdrop-blur-md animate-in fade-in duration-100"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="w-full max-w-xl rounded-2xl bg-[var(--color-surface)] border border-[var(--color-border)] shadow-2xl overflow-hidden flex flex-col">
        <Command className="w-full text-xs">
          {/* Search bar */}
          <div className="flex items-center px-4 border-b border-[var(--color-border)]">
            <Command.Input
              autoFocus
              placeholder="Search workspaces, panels and actions"
              className="w-full h-11 bg-transparent text-xs text-[var(--color-fg)] placeholder:text-[var(--color-fg-subtle)] focus:outline-none"
            />
          </div>

          <Command.List className="max-h-80 overflow-y-auto p-2 space-y-1">
            <Command.Empty className="p-4 text-center text-xs text-[var(--color-fg-muted)]">
              No matching commands or actions found.
            </Command.Empty>

            {/* Workspaces Group */}
            <Command.Group heading="Studio Workspaces" className="text-[11px] font-medium text-[var(--color-fg-subtle)] px-2 py-1">
              <Command.Item
                onSelect={() => {
                  onSelectWorkspace('dashboard');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Home className="w-4 h-4 text-[var(--color-accent)]" />
                  <span>Dashboard (Home)</span>
                </div>
                <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">Ctrl+0</kbd>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onSelectWorkspace('voice');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Mic className="w-4 h-4 text-[var(--color-accent)]" />
                  <span>Voice Studio Lab</span>
                </div>
                <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">Ctrl+1</kbd>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onSelectWorkspace('multitrack');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Layers className="w-4 h-4 text-indigo-400" />
                  <span>Multitrack Arranger DAW</span>
                </div>
                <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">Ctrl+2</kbd>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onSelectWorkspace('audiobook');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <BookOpen className="w-4 h-4 text-[var(--color-fg-muted)]" />
                  <span>Audiobook Production Lab</span>
                </div>
                <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">Ctrl+3</kbd>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onSelectWorkspace('dubbing');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Film className="w-4 h-4 text-pink-400" />
                  <span>Video Dubbing & Stem Isolation</span>
                </div>
                <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">Ctrl+4</kbd>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onSelectWorkspace('tools');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Wrench className="w-4 h-4 text-[var(--color-fg-muted)]" />
                  <span>Audio Tools, Batch & Repair</span>
                </div>
                <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">Ctrl+5</kbd>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onSelectWorkspace('effects');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Sparkles className="w-4 h-4 text-[var(--color-fg-muted)]" />
                  <span>DSP Effects & Plugins Rack</span>
                </div>
                <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">Ctrl+6</kbd>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onSelectWorkspace('settings');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Settings className="w-4 h-4 text-[var(--color-fg-muted)]" />
                  <span>Preferences & Hardware</span>
                </div>
                <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">Ctrl+7</kbd>
              </Command.Item>
            </Command.Group>

            {/* Studio Panels Group */}
            <Command.Group heading="Floating Panels" className="text-[11px] font-medium text-[var(--color-fg-subtle)] px-2 py-1 mt-2">
              <Command.Item
                onSelect={() => {
                  onTogglePanel('mixer');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Sliders className="w-4 h-4 text-[var(--color-accent)]" />
                  <span>Toggle Mixer Panel</span>
                </div>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onTogglePanel('transport');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Play className="w-4 h-4 text-[var(--color-accent)]" />
                  <span>Toggle Transport Panel</span>
                </div>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onTogglePanel('timeline');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Layers className="w-4 h-4 text-[var(--color-accent)]" />
                  <span>Toggle Timeline Panel</span>
                </div>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onTogglePanel('inspector');
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Eye className="w-4 h-4 text-[var(--color-accent)]" />
                  <span>Toggle Right Inspector Drawer</span>
                </div>
                <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">Ctrl+B</kbd>
              </Command.Item>
            </Command.Group>

            {/* Global Actions */}
            <Command.Group heading="Studio Actions & Views" className="text-[11px] font-medium text-[var(--color-fg-subtle)] px-2 py-1 mt-2">
              <Command.Item
                onSelect={() => {
                  onTriggerEdit();
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <Zap className="w-4 h-4 text-[var(--color-fg-muted)]" />
                  <span>Trigger Timeline Demo Edit</span>
                </div>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onResetLayout();
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <RotateCcw className="w-4 h-4 text-[var(--color-accent)]" />
                  <span>Reset Floating Panels to Default</span>
                </div>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onToggleTheme();
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  {theme === 'dark' ? <Sun className="w-4 h-4 text-[var(--color-fg-muted)]" /> : <Moon className="w-4 h-4 text-indigo-400" />}
                  <span>Toggle Dark / Light Theme</span>
                </div>
                <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">Ctrl+D</kbd>
              </Command.Item>

              <Command.Item
                onSelect={() => {
                  onOpenShortcuts();
                  onClose();
                }}
                className="flex items-center justify-between p-2 rounded-lg cursor-pointer text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] aria-selected:bg-[var(--color-surface-2)]"
              >
                <div className="flex items-center gap-2.5">
                  <HelpCircle className="w-4 h-4 text-[var(--color-accent)]" />
                  <span>Open Keyboard Shortcuts Cheatsheet</span>
                </div>
                <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">F1</kbd>
              </Command.Item>
            </Command.Group>
          </Command.List>

          <div className="p-2.5 border-t border-[var(--color-border)] bg-[var(--color-surface-2)] flex items-center justify-between text-[11px] text-[var(--color-fg-subtle)]">
            <div className="flex items-center gap-3">
              <span>Use <kbd className="font-mono text-[10px] px-1 py-0.5 rounded bg-[var(--color-surface-3)]">↑</kbd> <kbd className="font-mono text-[10px] px-1 py-0.5 rounded bg-[var(--color-surface-3)]">↓</kbd> to navigate</span>
              <span><kbd className="font-mono text-[10px] px-1 py-0.5 rounded bg-[var(--color-surface-3)]">Enter</kbd> to select</span>
            </div>
            <span><kbd className="font-mono text-[10px] px-1 py-0.5 rounded bg-[var(--color-surface-3)]">Esc</kbd> to close</span>
          </div>
        </Command>
      </div>
    </div>
  );
}
