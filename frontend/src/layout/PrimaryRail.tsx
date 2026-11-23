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
  neon: string;
  lightNeon: string;
  shortcut: string;
  desc: string;
}

export const DASHBOARD_ITEM: WorkspaceItem = {
  id: 'dashboard',
  name: 'Dashboard',
  icon: Home,
  neon: '#22d3ee',
  lightNeon: '#0e7490',
  shortcut: 'Ctrl+0',
  desc: 'Home overview with one-tap workspace and action shortcuts',
};

export const WORKSPACE_LIST: WorkspaceItem[] = [
  {
    id: 'voice',
    name: 'Voice Studio',
    icon: Mic,
    neon: '#00f0ff',
    lightNeon: '#008899',
    shortcut: 'Ctrl+1',
    desc: 'Neural voice cloning, TTS synthesis, and pronunciation lexicon',
  },
  {
    id: 'multitrack',
    name: 'Multitrack DAW',
    icon: Layers,
    neon: '#6366f1',
    lightNeon: '#4338ca',
    shortcut: 'Ctrl+2',
    desc: 'Non-linear multitrack arranger, clip envelopes, and zero-latency mixing',
  },
  {
    id: 'audiobook',
    name: 'Audiobook Lab',
    icon: BookOpen,
    neon: '#f59e0b',
    lightNeon: '#b45309',
    shortcut: 'Ctrl+3',
    desc: 'Long-form narrative, multi-cast tagging, and ACX loudness compliance',
  },
  {
    id: 'dubbing',
    name: 'Video Dubbing',
    icon: Film,
    neon: '#ec4899',
    lightNeon: '#be185d',
    shortcut: 'Ctrl+4',
    desc: 'Video playback sync, stem isolation, and ADR dialog replacement',
  },
  {
    id: 'tools',
    name: 'Batch & Repair',
    icon: Wrench,
    neon: '#10b981',
    lightNeon: '#047857',
    shortcut: 'Ctrl+5',
    desc: 'Spectral de-noising, click removal, and batch audio normalizer',
  },
  {
    id: 'effects',
    name: 'DSP & Effects',
    icon: Sparkles,
    neon: '#8b5cf6',
    lightNeon: '#6d28d9',
    shortcut: 'Ctrl+6',
    desc: 'Parametric EQ, multiband compression, and VST/LV2 plugin hosting',
  },
  {
    id: 'settings',
    name: 'Preferences',
    icon: Settings,
    neon: '#94a3b8',
    lightNeon: '#475569',
    shortcut: 'Ctrl+7',
    desc: 'Audio hardware devices, buffer sizes, MCP agents, and glass settings',
  },
];

interface PrimaryRailProps {
  activeWorkspace: string;
  onSelectWorkspace: (id: string) => void;
  onOpenCommandPalette: () => void;
  onOpenShortcuts: () => void;
  onToggleTheme: () => void;
  theme: 'dark' | 'light';
}

export function PrimaryRail({
  activeWorkspace,
  onSelectWorkspace,
  onOpenCommandPalette,
  onOpenShortcuts,
  onToggleTheme,
  theme,
}: PrimaryRailProps) {
  return (
    <aside
      className="w-14 border-r border-[var(--color-border)] bg-[var(--color-surface)] flex flex-col items-center py-3 justify-between flex-shrink-0 z-30 select-none"
      aria-label="Workspace Navigation"
    >
      {/* Top Section: App Logo (Dashboard) & 7 Workspaces */}
      <div className="flex flex-col items-center gap-2 w-full">
        {/* App Logo = Dashboard / Home button */}
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
            aria-current={activeWorkspace === 'dashboard' ? 'page' : undefined}
            className={`relative group w-10 h-10 rounded-xl flex items-center justify-center mb-2 transition-all cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus)] ${
              activeWorkspace === 'dashboard'
                ? 'bg-[var(--color-surface-2)] shadow-md'
                : 'hover:bg-[var(--color-surface-2)]'
            }`}
          >
            <img
              src={appIcon}
              alt=""
              aria-hidden="true"
              className="w-7 h-7 rounded-lg transition-transform group-hover:scale-110 motion-reduce:transition-none"
            />
            {activeWorkspace === 'dashboard' && (
              <span
                className="absolute -left-1.5 w-1 h-5 rounded-r-full"
                style={{ backgroundColor: theme === 'light' ? DASHBOARD_ITEM.lightNeon : DASHBOARD_ITEM.neon }}
              />
            )}
          </button>
        </Tooltip>

        {/* 7 Workspaces */}
        {WORKSPACE_LIST.map((ws) => {
          const Icon = ws.icon;
          const isActive = activeWorkspace === ws.id;
          const activeColor = theme === 'light' ? ws.lightNeon : ws.neon;

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
                className={`relative group w-10 h-10 rounded-xl flex items-center justify-center transition-all cursor-pointer ${
                  isActive
                    ? 'bg-[var(--color-surface-2)] shadow-md text-white'
                    : 'text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] hover:bg-[var(--color-surface-2)]'
                }`}
              >
                <Icon
                  className="w-5 h-5 transition-transform group-hover:scale-110"
                  style={{ color: isActive ? activeColor : undefined }}
                />

                {/* Left Active Indicator Pip */}
                {isActive && (
                  <span
                    className="absolute -left-1.5 w-1 h-5 rounded-r-full"
                    style={{ backgroundColor: activeColor }}
                  />
                )}
              </button>
            </Tooltip>
          );
        })}
      </div>

      {/* Bottom Section: Utility Actions (Theme, Shortcuts, Palette) */}
      <div className="flex flex-col items-center gap-2 w-full pt-2 border-t border-[var(--color-border)]">
        <Tooltip position="right" content="Command Palette (Ctrl+K)">
          <button
            type="button"
            onClick={onOpenCommandPalette}
            aria-label="Command Palette"
            className="w-10 h-10 rounded-xl flex items-center justify-center text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] transition-colors cursor-pointer"
          >
            <CommandIcon className="w-4 h-4" />
          </button>
        </Tooltip>

        <Tooltip position="right" content="Keyboard Shortcuts (F1)">
          <button
            type="button"
            onClick={onOpenShortcuts}
            aria-label="Keyboard Shortcuts"
            className="w-10 h-10 rounded-xl flex items-center justify-center text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] transition-colors cursor-pointer"
          >
            <HelpCircle className="w-4 h-4" />
          </button>
        </Tooltip>

        <Tooltip position="right" content={`Toggle Theme (${theme === 'dark' ? 'Light' : 'Dark'}, Ctrl+D)`}>
          <button
            type="button"
            onClick={onToggleTheme}
            aria-label="Toggle Theme"
            className="w-10 h-10 rounded-xl flex items-center justify-center text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] transition-colors cursor-pointer"
          >
            {theme === 'dark' ? <Sun className="w-4 h-4 text-amber-400" /> : <Moon className="w-4 h-4 text-indigo-400" />}
          </button>
        </Tooltip>
      </div>
    </aside>
  );
}
