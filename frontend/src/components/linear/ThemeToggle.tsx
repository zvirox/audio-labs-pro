import { useEffect, useState } from 'react';
import { Moon, Sun } from 'lucide-react';

export type LinearTheme = 'dark' | 'light';

export const LINEAR_THEME_STORAGE_KEY = 'ui-theme';

export function getLinearTheme(): LinearTheme {
  if (typeof document === 'undefined') return 'dark';
  const attr = document.documentElement.dataset.theme;
  if (attr === 'light' || attr === 'dark') return attr;
  try {
    const stored = localStorage.getItem(LINEAR_THEME_STORAGE_KEY);
    if (stored === 'light' || stored === 'dark') return stored;
  } catch {
    // ignore storage access issues
  }
  if (typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: light)').matches) {
    return 'light';
  }
  return 'dark';
}

export function applyLinearTheme(theme: LinearTheme): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.classList.add('theme-switching');
  root.dataset.theme = theme;
  root.classList.remove('dark', 'light');
  root.classList.add(theme);
  if (document.body) {
    void document.body.offsetHeight;
  }
  requestAnimationFrame(() => {
    root.classList.remove('theme-switching');
  });
  try {
    localStorage.setItem(LINEAR_THEME_STORAGE_KEY, theme);
  } catch {
    // ignore storage write issues
  }
  window.dispatchEvent(new CustomEvent('theme-changed', { detail: theme }));
}

export function toggleLinearTheme(): LinearTheme {
  const next: LinearTheme = getLinearTheme() === 'dark' ? 'light' : 'dark';
  applyLinearTheme(next);
  return next;
}

export function useLinearTheme(): [LinearTheme, (next: LinearTheme) => void] {
  const [theme, setTheme] = useState<LinearTheme>(getLinearTheme);

  useEffect(() => {
    const onThemeChanged = (event: Event) => {
      const custom = event as CustomEvent<LinearTheme>;
      if (custom.detail === 'light' || custom.detail === 'dark') {
        setTheme(custom.detail);
      } else {
        setTheme(getLinearTheme());
      }
    };
    window.addEventListener('theme-changed', onThemeChanged);
    return () => window.removeEventListener('theme-changed', onThemeChanged);
  }, []);

  return [theme, applyLinearTheme];
}

export interface ThemeToggleProps {
  className?: string;
}

export function ThemeToggle({ className = '' }: ThemeToggleProps) {
  const [theme, setTheme] = useLinearTheme();
  const isDark = theme === 'dark';

  return (
    <button
      type="button"
      onClick={() => setTheme(isDark ? 'light' : 'dark')}
      title={isDark ? 'Switch to light theme (Ctrl+D)' : 'Switch to dark theme (Ctrl+D)'}
      aria-label={isDark ? 'Switch to light theme' : 'Switch to dark theme'}
      data-testid="titlebar-theme-toggle"
      className={`inline-flex h-7 w-7 items-center justify-center rounded-md text-muted hover:bg-hover hover:text-fg transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] cursor-pointer ${className}`}
    >
      {isDark ? <Sun className="h-3.5 w-3.5" /> : <Moon className="h-3.5 w-3.5" />}
    </button>
  );
}
