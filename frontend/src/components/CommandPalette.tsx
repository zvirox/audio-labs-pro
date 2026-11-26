import { useEffect, useState } from 'react';
import { Command } from 'cmdk';
import {
  LayoutGrid,
  Moon,
  Plus,
  RotateCcw,
  Search,
  Sparkles,
  Sun,
  Terminal,
} from 'lucide-react';
import { DASHBOARD_ITEM, WORKSPACE_LIST } from '../layout/PrimaryRail';
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
  const [search, setSearch] = useState('');

  useEffect(() => {
    if (!open) {
      setSearch('');
    }
  }, [open]);

  if (!open) return null;

  const runAction = (fn: () => void) => {
    fn();
    onClose();
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[14vh] px-4"
      role="dialog"
      aria-modal="true"
      aria-label="Command Palette"
    >
      {/* Solid dimmed scrim */}
      <div
        className="fixed inset-0 bg-scrim"
        onClick={onClose}
        aria-hidden="true"
      />

      <div className="relative z-10 w-full max-w-[600px] overflow-hidden rounded-lg border border-border-strong bg-surface shadow-card">
        <Command
          label="Command Palette"
          className="flex flex-col"
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              e.preventDefault();
              onClose();
            }
          }}
        >
          <div className="flex items-center gap-2.5 border-b border-border px-3.5 py-2.5">
            <Search className="h-4 w-4 shrink-0 text-muted" />
            <Command.Input
              autoFocus
              value={search}
              onValueChange={setSearch}
              placeholder="Jump to workspace, toggle panel, or run command..."
              className="w-full bg-transparent text-[13px] text-fg placeholder:text-faint outline-none"
            />
            <kbd className="rounded border border-border bg-surface-2 px-1.5 py-0.5 font-mono text-[10px] text-muted">
              ESC
            </kbd>
          </div>

          <Command.List className="max-h-[360px] overflow-y-auto p-2 text-[13px]">
            <Command.Empty className="py-8 text-center text-[13px] text-muted">
              No matching commands found.
            </Command.Empty>

            <Command.Group
              heading="Workspaces"
              className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:uppercase [&_[cmdk-group-heading]]:tracking-wider [&_[cmdk-group-heading]]:text-faint"
            >
              {[DASHBOARD_ITEM, ...WORKSPACE_LIST].map((ws) => {
                const Icon = ws.icon;
                return (
                  <Command.Item
                    key={ws.id}
                    value={`${ws.name} ${ws.desc}`}
                    onSelect={() => runAction(() => onSelectWorkspace(ws.id))}
                    className="flex items-center justify-between gap-3 rounded-md px-2.5 py-2 text-fg-2 transition-colors duration-[var(--duration-quick)] aria-selected:bg-active aria-selected:text-fg cursor-pointer"
                  >
                    <div className="flex min-w-0 items-center gap-2.5">
                      <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded border border-border bg-primary-tint text-primary-text">
                        <Icon className="h-3.5 w-3.5" />
                      </span>
                      <div className="min-w-0">
                        <div className="truncate font-medium text-fg">{ws.name}</div>
                        <div className="truncate text-[11px] text-muted">{ws.desc}</div>
                      </div>
                    </div>
                    <kbd className="shrink-0 rounded border border-border bg-surface-2 px-1.5 py-0.5 font-mono text-[10px] text-muted">
                      {ws.shortcut}
                    </kbd>
                  </Command.Item>
                );
              })}
            </Command.Group>

            <Command.Group
              heading="Panels & Layout"
              className="mt-1.5 [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:uppercase [&_[cmdk-group-heading]]:tracking-wider [&_[cmdk-group-heading]]:text-faint"
            >
              {(
                [
                  { id: 'timeline', label: 'Toggle Timeline Panel' },
                  { id: 'mixer', label: 'Toggle Mixer Rack Panel' },
                  { id: 'transport', label: 'Toggle Transport Panel' },
                  { id: 'voice_lab', label: 'Toggle Voice Lab Panel' },
                  { id: 'effects', label: 'Toggle DSP Effects Panel' },
                  { id: 'models', label: 'Toggle Neural Models Panel' },
                  { id: 'media_bin', label: 'Toggle Media Bin Panel' },
                  { id: 'ai_activity', label: 'Toggle MCP Agent Activity Panel' },
                  { id: 'export_queue', label: 'Toggle Export Queue Panel' },
                ] as { id: PanelKind; label: string }[]
              ).map((p) => (
                <Command.Item
                  key={p.id}
                  value={p.label}
                  onSelect={() => runAction(() => onTogglePanel(p.id))}
                  className="flex items-center gap-2.5 rounded-md px-2.5 py-2 text-fg-2 transition-colors duration-[var(--duration-quick)] aria-selected:bg-active aria-selected:text-fg cursor-pointer"
                >
                  <LayoutGrid className="h-3.5 w-3.5 text-muted" />
                  <span>{p.label}</span>
                </Command.Item>
              ))}
              <Command.Item
                value="Reset Panel Layout"
                onSelect={() => runAction(onResetLayout)}
                className="flex items-center gap-2.5 rounded-md px-2.5 py-2 text-fg-2 transition-colors duration-[var(--duration-quick)] aria-selected:bg-active aria-selected:text-fg cursor-pointer"
              >
                <RotateCcw className="h-3.5 w-3.5 text-muted" />
                <span>Reset Panel Layout</span>
              </Command.Item>
            </Command.Group>

            <Command.Group
              heading="Session & Appearance"
              className="mt-1.5 [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:uppercase [&_[cmdk-group-heading]]:tracking-wider [&_[cmdk-group-heading]]:text-faint"
            >
              <Command.Item
                value="Add Sample Track Timeline Mutation"
                onSelect={() => runAction(onTriggerEdit)}
                className="flex items-center gap-2.5 rounded-md px-2.5 py-2 text-fg-2 transition-colors duration-[var(--duration-quick)] aria-selected:bg-active aria-selected:text-fg cursor-pointer"
              >
                <Plus className="h-3.5 w-3.5 text-primary-text" />
                <span>Add Sample Track to Session</span>
              </Command.Item>
              <Command.Item
                value="Toggle Light Dark Theme"
                onSelect={() => runAction(onToggleTheme)}
                className="flex items-center justify-between rounded-md px-2.5 py-2 text-fg-2 transition-colors duration-[var(--duration-quick)] aria-selected:bg-active aria-selected:text-fg cursor-pointer"
              >
                <span className="flex items-center gap-2.5">
                  {theme === 'dark' ? (
                    <Sun className="h-3.5 w-3.5 text-amber" />
                  ) : (
                    <Moon className="h-3.5 w-3.5 text-primary-text" />
                  )}
                  <span>Switch to {theme === 'dark' ? 'Light' : 'Dark'} Theme</span>
                </span>
                <kbd className="rounded border border-border bg-surface-2 px-1.5 py-0.5 font-mono text-[10px] text-muted">
                  Ctrl+D
                </kbd>
              </Command.Item>
              <Command.Item
                value="Show Keyboard Shortcuts"
                onSelect={() => runAction(onOpenShortcuts)}
                className="flex items-center justify-between rounded-md px-2.5 py-2 text-fg-2 transition-colors duration-[var(--duration-quick)] aria-selected:bg-active aria-selected:text-fg cursor-pointer"
              >
                <span className="flex items-center gap-2.5">
                  <Terminal className="h-3.5 w-3.5 text-muted" />
                  <span>Open Keyboard Shortcuts Reference</span>
                </span>
                <kbd className="rounded border border-border bg-surface-2 px-1.5 py-0.5 font-mono text-[10px] text-muted">
                  ?
                </kbd>
              </Command.Item>
              <Command.Item
                value="Audio Labs Engine Info"
                onSelect={() => runAction(() => onSelectWorkspace('dashboard'))}
                className="flex items-center gap-2.5 rounded-md px-2.5 py-2 text-fg-2 transition-colors duration-[var(--duration-quick)] aria-selected:bg-active aria-selected:text-fg cursor-pointer"
              >
                <Sparkles className="h-3.5 w-3.5 text-sky" />
                <span>Go to Studio Overview</span>
              </Command.Item>
            </Command.Group>
          </Command.List>
        </Command>
      </div>
    </div>
  );
}
