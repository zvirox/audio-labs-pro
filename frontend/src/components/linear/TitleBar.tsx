import { useCallback, useEffect, useState } from 'react';
import { Copy, Minus, Square, X } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ThemeToggle } from './ThemeToggle';

export interface TitleBarProps {
  appName?: string;
  version?: string;
  className?: string;
}

function getDefaultVersion(): string {
  const raw = typeof __APP_VERSION__ !== 'undefined' && __APP_VERSION__ ? __APP_VERSION__ : '0.1.0-beta.4';
  return raw.startsWith('v') ? raw : `v${raw}`;
}

function hasTauriWindow(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export function TitleBar({
  appName = 'Audio Labs Pro',
  version = getDefaultVersion(),
  className = '',
}: TitleBarProps) {
  const [isMaximized, setIsMaximized] = useState(false);

  const syncMaximized = useCallback(async () => {
    if (!hasTauriWindow()) return;
    try {
      const win = getCurrentWindow();
      setIsMaximized(await win.isMaximized());
    } catch {
      // ignore in non-windowed test environments
    }
  }, []);

  useEffect(() => {
    if (!hasTauriWindow()) return;
    let unlisten: (() => void) | undefined;
    void syncMaximized();
    try {
      const win = getCurrentWindow();
      void win
        .onResized(() => {
          void syncMaximized();
        })
        .then((fn) => {
          unlisten = fn;
        });
    } catch {
      // ignore in headless browser preview
    }
    return () => {
      if (unlisten) unlisten();
    };
  }, [syncMaximized]);

  const handleDragMouseDown = (event: React.MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0 || event.detail >= 2 || !hasTauriWindow()) return;
    try {
      void getCurrentWindow().startDragging();
    } catch {
      // ignore in headless browser preview
    }
  };

  const handleToggleMaximize = () => {
    if (!hasTauriWindow()) return;
    try {
      void getCurrentWindow()
        .toggleMaximize()
        .then(() => syncMaximized());
    } catch {
      // ignore in headless browser preview
    }
  };

  const handleMinimize = () => {
    if (!hasTauriWindow()) return;
    try {
      void getCurrentWindow().minimize();
    } catch {
      // ignore in headless browser preview
    }
  };

  const handleClose = () => {
    if (!hasTauriWindow()) return;
    try {
      void getCurrentWindow().close();
    } catch {
      // ignore in headless browser preview
    }
  };

  return (
    <header
      className={`flex h-10 shrink-0 select-none items-center justify-between border-b border-border bg-app px-3 text-fg ${className}`}
      data-testid="linear-titlebar"
    >
      {/* Left: 20px app logo, app name, version tag */}
      <div
        data-tauri-drag-region
        onMouseDown={handleDragMouseDown}
        onDoubleClick={handleToggleMaximize}
        className="flex items-center gap-2 pr-3"
      >
        <svg
          viewBox="0 0 20 20"
          className="h-5 w-5 shrink-0 text-primary-text"
          fill="none"
          aria-hidden="true"
        >
          <rect x="1" y="1" width="18" height="18" rx="4" stroke="currentColor" strokeWidth="1.5" />
          <path
            d="M5.5 11.5V8.5M8.5 13.5V6.5M11.5 12V8M14.5 10.5V9.5"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
          />
        </svg>
        <span className="text-[13px] font-semibold tracking-tight text-fg">
          {appName}
        </span>
        <span className="rounded border border-border bg-surface-2 px-1.5 py-0.5 text-[11px] font-medium leading-none text-muted tabular-nums">
          {version}
        </span>
      </div>

      {/* Center: clean empty drag region */}
      <div
        data-tauri-drag-region
        onMouseDown={handleDragMouseDown}
        onDoubleClick={handleToggleMaximize}
        className="h-full flex-1"
      />

      {/* Right: ThemeToggle, divider, minimize, maximize/restore, close */}
      <div className="flex items-center gap-0.5">
        <ThemeToggle />
        <div className="mx-1.5 h-4 w-px bg-border" aria-hidden="true" />
        <button
          type="button"
          onClick={handleMinimize}
          aria-label="Minimize window"
          title="Minimize"
          className="inline-flex h-7 w-8 items-center justify-center rounded-md text-muted hover:bg-hover hover:text-fg transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] cursor-pointer"
        >
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          onClick={handleToggleMaximize}
          aria-label={isMaximized ? 'Restore window' : 'Maximize window'}
          title={isMaximized ? 'Restore' : 'Maximize'}
          className="inline-flex h-7 w-8 items-center justify-center rounded-md text-muted hover:bg-hover hover:text-fg transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] cursor-pointer"
        >
          {isMaximized ? <Copy className="h-3 w-3" /> : <Square className="h-3 w-3" />}
        </button>
        <button
          type="button"
          onClick={handleClose}
          aria-label="Close window"
          title="Close"
          className="inline-flex h-7 w-8 items-center justify-center rounded-md text-muted hover:bg-close-hover hover:text-primary-fg transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] cursor-pointer"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
    </header>
  );
}
