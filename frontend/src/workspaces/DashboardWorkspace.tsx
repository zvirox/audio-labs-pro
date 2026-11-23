import {
  ArrowRight,
  Command as CommandIcon,
  HelpCircle,
} from 'lucide-react';
import type React from 'react';
import { WORKSPACE_LIST } from '../layout/PrimaryRail';
import appIcon from '../assets/app-icon.png';

interface DashboardWorkspaceProps {
  theme: 'dark' | 'light';
  onOpenWorkspace: (id: string) => void;
  onOpenPalette: () => void;
  onOpenShortcuts: () => void;
}

interface DashboardCardProps {
  icon: React.ComponentType<{ className?: string; style?: React.CSSProperties }>;
  name: string;
  desc: string;
  shortcut?: string;
  color: string;
  onClick: () => void;
}

/** A floating, keyboard-operable shortcut panel (icon + name + description). */
function DashboardCard({ icon: Icon, name, desc, shortcut, color, onClick }: DashboardCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="group relative flex min-h-[136px] flex-col items-start gap-2.5 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-4 text-left shadow-sm transition-all cursor-pointer hover:-translate-y-0.5 hover:border-[var(--color-border-strong)] hover:bg-[var(--color-surface-2)] hover:shadow-md focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus)] active:scale-[0.99] motion-reduce:transition-none motion-reduce:hover:translate-y-0 motion-reduce:active:scale-100"
    >
      <span className="flex w-full items-start justify-between">
        <span
          aria-hidden="true"
          className="flex h-10 w-10 items-center justify-center rounded-lg border"
          style={{
            color,
            backgroundColor: `color-mix(in oklab, ${color} 14%, transparent)`,
            borderColor: `color-mix(in oklab, ${color} 35%, transparent)`,
          }}
        >
          <Icon className="h-5 w-5" />
        </span>
        {shortcut && (
          <kbd className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono text-[10px] text-[var(--color-fg-muted)]">
            {shortcut}
          </kbd>
        )}
      </span>

      <span className="text-sm font-semibold text-[var(--color-fg)]">{name}</span>
      <span className="pr-5 text-xs leading-relaxed text-[var(--color-fg-muted)]">{desc}</span>

      <ArrowRight
        aria-hidden="true"
        className="absolute bottom-4 right-4 h-4 w-4 text-[var(--color-fg-subtle)] transition-transform group-hover:translate-x-0.5 group-hover:text-[var(--color-fg)] motion-reduce:transition-none"
      />
    </button>
  );
}

/** Decorative waveform motif (purely visual). */
function WaveformMotif() {
  const bars = [10, 22, 14, 30, 18, 36, 24, 12, 28, 16, 34, 20, 10, 26, 14];
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 150 40"
      className="pointer-events-none h-10 w-40 text-[var(--color-accent)] opacity-20"
      fill="currentColor"
    >
      {bars.map((h, i) => (
        <rect key={i} x={i * 10} y={(40 - h) / 2} width={4} height={h} rx={2} />
      ))}
    </svg>
  );
}

/**
 * Dashboard (Home) — the default landing view. Presents the studio's
 * workspaces and most-used actions as floating shortcut panels, each with an
 * icon, name and short description, mirroring the reference home layout.
 */
export function DashboardWorkspace({
  theme,
  onOpenWorkspace,
  onOpenPalette,
  onOpenShortcuts,
}: DashboardWorkspaceProps) {
  const quickActions = [
    {
      icon: CommandIcon,
      name: 'Command Palette',
      desc: 'Search and run any workspace, panel or studio action from one place.',
      shortcut: 'Ctrl+K',
      color: 'var(--color-info)',
      onClick: onOpenPalette,
    },
    {
      icon: HelpCircle,
      name: 'Keyboard Shortcuts',
      desc: 'Browse the full shortcut cheatsheet for navigation and editing.',
      shortcut: 'F1',
      color: 'var(--color-focus)',
      onClick: onOpenShortcuts,
    },
  ];

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
      {/* Hero */}
      <header className="relative flex items-center gap-4 overflow-hidden rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-5">
        <img
          src={appIcon}
          alt="Audio Labs Pro logo"
          className="h-12 w-12 flex-shrink-0 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface-2)] p-1.5"
        />
        <div className="min-w-0">
          <h1 className="text-xl font-bold tracking-tight text-[var(--color-fg)]">
            Audio Labs Pro
          </h1>
          <p className="mt-1 text-xs leading-relaxed text-[var(--color-fg-muted)]">
            Your local-first audio studio — clone and design voices, arrange multitrack
            sessions, produce audiobooks, dub video, and repair audio, all on-device.
          </p>
        </div>
        <div className="ml-auto hidden flex-shrink-0 md:block">
          <WaveformMotif />
        </div>
      </header>

      {/* Shortcut panels */}
      <section aria-label="Workspaces and quick actions">
        <h2 className="mb-3 text-[11px] font-bold uppercase tracking-wider text-[var(--color-fg-muted)]">
          Jump back in
        </h2>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {WORKSPACE_LIST.map((ws) => (
            <DashboardCard
              key={ws.id}
              icon={ws.icon}
              name={ws.name}
              desc={ws.desc}
              shortcut={ws.shortcut}
              color={theme === 'light' ? ws.lightNeon : ws.neon}
              onClick={() => onOpenWorkspace(ws.id)}
            />
          ))}
          {quickActions.map((action) => (
            <DashboardCard
              key={action.name}
              icon={action.icon}
              name={action.name}
              desc={action.desc}
              shortcut={action.shortcut}
              color={action.color}
              onClick={action.onClick}
            />
          ))}
        </div>
      </section>

      <p className="text-[11px] text-[var(--color-fg-muted)]">
        Tip: press <kbd className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-1 py-0.5 font-mono">Ctrl+K</kbd> anywhere
        to open the command palette, or <kbd className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-1 py-0.5 font-mono">Ctrl+0</kbd> to
        return to this dashboard.
      </p>
    </div>
  );
}
