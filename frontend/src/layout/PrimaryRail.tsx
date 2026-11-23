import React from 'react';
import {
  Mic,
  Layers,
  BookOpen,
  Film,
  Wrench,
  Sparkles,
  Settings,
  HelpCircle,
  Command as CommandIcon,
  Sun,
  Moon,
  Home,
} from 'lucide-react';
import { Tooltip } from '../kit/Tooltip';
import appIcon from '../assets/app-icon.png';

export interface WorkspaceItem {
  id: string;
  name: string;
  icon: React.ComponentType<{ className?: string; style?: React.CSSProperties }>;
  shortcut: string;
  desc: string;
}

export const DASHBOARD_ITEM: WorkspaceItem = {
  id: 'dashboard',
  name: 'Dashboard',
  icon: Home,
  shortcut: 'Ctrl+0',
  desc: 'Home overview with one-tap workspace and action shortcuts',
};

export const WORKSPACE_LIST: WorkspaceItem[] = [
  { id: 'voice', name: 'Voice Studio', icon: Mic, shortcut: 'Ctrl+1', desc: 'Neural voice cloning, TTS synthesis, and pronunciation lexicon' },
  { id: 'multitrack', name: 'Multitrack DAW', icon: Layers, shortcut: 'Ctrl+2', desc: 'Non-linear multitrack arranger, clip envelopes, and zero-latency mixing' },
  { id: 'audiobook', name: 'Audiobook Lab', icon: BookOpen, shortcut: 'Ctrl+3', desc: 'Long-form narrative, multi-cast tagging, and ACX loudness compliance' },
  { id: 'dubbing', name: 'Video Dubbing', icon: Film, shortcut: 'Ctrl+4', desc: 'Video playback sync, stem isolation, and ADR dialog replacement' },
  { id: 'tools', name: 'Batch & Repair', icon: Wrench, shortcut: 'Ctrl+5', desc: 'Spectral de-noising, click removal, and batch audio normalizer' },
  { id: 'effects', name: 'DSP & Effects', icon: Sparkles, shortcut: 'Ctrl+6', desc: 'Parametric EQ, multiband compression, and VST/LV2 plugin hosting' },
  { id: 'settings', name: 'Preferences', icon: Settings, shortcut: 'Ctrl+7', desc: 'Audio hardware devices, buffer sizes, MCP agents, and glass settings' },
];

interface PrimaryRailProps {
  activeWorkspace: string;
  onSelectWorkspace: (id: string) => void;
  onOpenCommandPalette: () => void;
  onOpenShortcuts: () => void;
  onToggleTheme: () => void;
  theme: 'dark' | 'light';
}

const rowBase =
  'relative w-10 h-10 rounded-lg flex items-center justify-center cursor-pointer transition-[color,background-color,border-color,opacity] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus)]';
const activeRow = 'bg-[var(--color-surface-2)] text-[var(--color-fg)]';
const idleRow =
  'text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] hover:bg-[var(--color-surface-2)]';

/** Active indicator: a short teal pip, the one accent mark in the chrome. */
function ActivePip() {
  return (
    <span
      aria-hidden="true"
      className="absolute -left-1.5 w-0.5 h-5 rounded-r-full bg-[var(--color-accent)]"
    />
  );
}

export function PrimaryRail({
  activeWorkspace,
  onSelectWorkspace,
  onOpenCommandPalette,
  onOpenShortcuts,
  onToggleTheme,
  theme,
}: PrimaryRailProps) {
  const dashboardActive = activeWorkspace === 'dashboard';

  return (
    <aside
      className="w-14 border-r border-[var(--color-border)] bg-[var(--color-surface)] flex flex-col items-center py-3 justify-between flex-shrink-0 z-30 select-none"
      aria-label="Workspace Navigation"
    >
      <div className="flex flex-col items-center gap-2 w-full">
        <Tooltip
          position="right"
          content={
            <div className="flex items-center gap-2">
              <span>Dashboard</span>
              <kbd className="opacity-60 text-[10px] font-mono">Ctrl+0</kbd>
            </div>
          }
        >
          <button
            type="button"
            onClick={() => onSelectWorkspace('dashboard')}
            aria-label="Dashboard (Home)"
            aria-current={dashboardActive ? 'page' : undefined}
            className={`${rowBase} mb-2 ${dashboardActive ? activeRow : idleRow}`}
          >
            <img src={appIcon} alt="" aria-hidden="true" className="w-7 h-7 rounded-md" />
            {dashboardActive && <ActivePip />}
          </button>
        </Tooltip>

        {WORKSPACE_LIST.map((ws) => {
          const Icon = ws.icon;
          const isActive = activeWorkspace === ws.id;
          return (
            <Tooltip
              key={ws.id}
              position="right"
              content={
                <div className="flex items-center gap-2">
                  <span>{ws.name}</span>
                  <kbd className="opacity-60 text-[10px] font-mono">{ws.shortcut}</kbd>
                </div>
              }
            >
              <button
                type="button"
                onClick={() => onSelectWorkspace(ws.id)}
                aria-label={ws.name}
                aria-current={isActive ? 'page' : undefined}
                className={`${rowBase} ${isActive ? activeRow : idleRow}`}
              >
                <Icon className="w-4 h-4" aria-hidden="true" />
                {isActive && <ActivePip />}
              </button>
            </Tooltip>
          );
        })}
      </div>

      <div className="flex flex-col items-center gap-2 w-full pt-2 border-t border-[var(--color-border)]">
        <Tooltip position="right" content="Command palette (Ctrl+K)">
          <button
            type="button"
            onClick={onOpenCommandPalette}
            aria-label="Command palette"
            className={`${rowBase} ${idleRow}`}
          >
            <CommandIcon className="w-4 h-4" aria-hidden="true" />
          </button>
        </Tooltip>
        <Tooltip position="right" content="Keyboard shortcuts (F1)">
          <button
            type="button"
            onClick={onOpenShortcuts}
            aria-label="Keyboard shortcuts"
            className={`${rowBase} ${idleRow}`}
          >
            <HelpCircle className="w-4 h-4" aria-hidden="true" />
          </button>
        </Tooltip>
        <Tooltip
          position="right"
          content={`Switch to ${theme === 'dark' ? 'light' : 'dark'} theme (Ctrl+D)`}
        >
          <button
            type="button"
            onClick={onToggleTheme}
            aria-label="Toggle theme"
            className={`${rowBase} ${idleRow}`}
          >
            {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
          </button>
        </Tooltip>
      </div>
    </aside>
  );
}
