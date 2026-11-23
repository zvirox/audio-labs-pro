import { ArrowRight, Command as CommandIcon, HelpCircle } from 'lucide-react';
import type React from 'react';
import { WORKSPACE_LIST } from '../layout/PrimaryRail';
import appIcon from '../assets/app-icon.png';

interface DashboardWorkspaceProps {
  theme: 'dark' | 'light';
  onOpenWorkspace: (id: string) => void;
  onOpenPalette: () => void;
  onOpenShortcuts: () => void;
}

interface ShortcutCardProps {
  icon: React.ComponentType<{ className?: string }>;
  name: string;
  desc: string;
  shortcut?: string;
  featured?: boolean;
  onClick: () => void;
}

/** Waveform motif — a small piece of the product's real material. */
function Waveform({ tall = false }: { tall?: boolean }) {
  const bars = [12, 26, 16, 38, 22, 46, 30, 16, 36, 20, 44, 26, 12, 32, 18];
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 150 48"
      className={tall ? 'h-12 w-full text-[var(--color-accent)] opacity-70' : 'h-8 w-28 text-[var(--color-fg-subtle)] opacity-60'}
      fill="currentColor"
    >
      {bars.map((h, i) => (
        <rect key={i} x={i * 10} y={(48 - h) / 2} width={4} height={h} rx={2} />
      ))}
    </svg>
  );
}

function ShortcutCard({
  icon: Icon,
  name,
  desc,
  shortcut,
  featured = false,
  onClick,
}: ShortcutCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="group flex min-h-32 flex-col items-start gap-3 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-5 text-left transition-[background-color,border-color] duration-[var(--duration-quick)] hover:bg-[var(--color-surface-2)] hover:border-[var(--color-border-strong)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus)]"
    >
      <span className="flex w-full items-start justify-between gap-3">
        <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]">
          <Icon className="h-4 w-4" aria-hidden="true" />
        </span>
        {shortcut ? (
          <kbd className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono text-[11px] text-[var(--color-fg-muted)]">
            {shortcut}
          </kbd>
        ) : (
          <ArrowRight className="h-4 w-4 text-[var(--color-fg-subtle)]" aria-hidden="true" />
        )}
      </span>

      <span className="text-sm font-medium text-[var(--color-fg)]">{name}</span>
      <span className="max-w-56 text-[13px] leading-5 text-[var(--color-fg-muted)]">{desc}</span>

      {featured && (
        <span className="mt-auto w-full rounded-lg bg-[var(--color-surface-2)] px-3 py-2">
          <Waveform tall />
        </span>
      )}
    </button>
  );
}

/** Dashboard (Home) — the app's landing view and workspace launcher. */
export function DashboardWorkspace({
  onOpenWorkspace,
  onOpenPalette,
  onOpenShortcuts,
}: DashboardWorkspaceProps) {
  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-8 pb-8">
      <header className="flex items-center gap-4">
        <img
          src={appIcon}
          alt=""
          aria-hidden="true"
          className="h-12 w-12 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-1.5"
        />
        <div className="min-w-0">
          <h1 className="text-[28px] leading-9 font-medium text-[var(--color-fg)]">Studio overview</h1>
          <p className="mt-1 text-sm leading-5 text-[var(--color-fg-muted)]">
            Voice, multitrack, audiobook, dubbing and repair, all on this machine
          </p>
        </div>
        <div className="ml-auto hidden w-28 lg:block">
          <Waveform />
        </div>
      </header>

      <section aria-label="Workspaces and actions">
        <h2 className="mb-3 text-[13px] font-medium text-[var(--color-fg-muted)]">Workspaces</h2>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {WORKSPACE_LIST.map((ws, i) => (
            <ShortcutCard
              key={ws.id}
              icon={ws.icon}
              name={ws.name}
              desc={ws.desc}
              shortcut={ws.shortcut}
              featured={i === 0}
              onClick={() => onOpenWorkspace(ws.id)}
            />
          ))}
          <ShortcutCard
            icon={CommandIcon}
            name="Command palette"
            desc="Search and run any workspace, panel or studio action"
            shortcut="Ctrl+K"
            onClick={onOpenPalette}
          />
          <ShortcutCard
            icon={HelpCircle}
            name="Keyboard shortcuts"
            desc="Browse the full shortcut reference"
            shortcut="F1"
            onClick={onOpenShortcuts}
          />
        </div>
      </section>

      <p className="text-[13px] text-[var(--color-fg-muted)]">
        Press Ctrl+K for the command palette, or Ctrl+0 to return here
      </p>
    </div>
  );
}
