import { useEffect, useState } from 'react';
import { Minus, Square, Copy, X } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isTauriEnvironment } from '../ipc/client';

/**
 * Custom minimize / maximize / close controls for the frameless Tauri window
 * (`decorations: false`). Renders nothing in browser-mock mode, where the
 * browser chrome already provides window management.
 */
export function WindowControls() {
  const [isMaximized, setIsMaximized] = useState(false);
  const native = isTauriEnvironment();

  useEffect(() => {
    if (!native) return;
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    win
      .onResized(() => {
        win.isMaximized().then(setIsMaximized).catch(() => undefined);
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => undefined);
    win.isMaximized().then(setIsMaximized).catch(() => undefined);
    return () => {
      if (unlisten) unlisten();
    };
  }, [native]);

  if (!native) return null;
  const win = getCurrentWindow();

  const base =
    'flex h-8 w-11 items-center justify-center text-[var(--color-fg-muted)] transition-colors cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-fg)]';

  return (
    <div className="flex items-center" role="group" aria-label="Window controls">
      <button
        type="button"
        className={base}
        title="Minimize"
        aria-label="Minimize window"
        onClick={() => win.minimize().catch(() => undefined)}
      >
        <Minus className="h-4 w-4" aria-hidden="true" />
      </button>
      <button
        type="button"
        className={base}
        title={isMaximized ? 'Restore' : 'Maximize'}
        aria-label={isMaximized ? 'Restore window' : 'Maximize window'}
        onClick={() =>
          win
            .toggleMaximize()
            .then(() => win.isMaximized())
            .then(setIsMaximized)
            .catch(() => undefined)
        }
      >
        {isMaximized ? (
          <Copy className="h-3.5 w-3.5" aria-hidden="true" />
        ) : (
          <Square className="h-3.5 w-3.5" aria-hidden="true" />
        )}
      </button>
      <button
        type="button"
        className={`${base} hover:bg-[var(--color-danger)] hover:text-white`}
        title="Close"
        aria-label="Close window"
        onClick={() => win.close().catch(() => undefined)}
      >
        <X className="h-4 w-4" aria-hidden="true" />
      </button>
    </div>
  );
}
