import { useEffect, useState } from 'react';

export interface LinearCanvasTokens {
  bgApp: string;
  bgSidebar: string;
  bgSurface: string;
  bgSurface2: string;
  bgActive: string;
  bgHover: string;
  bgTrack: string;
  border: string;
  borderStrong: string;
  text: string;
  textSecondary: string;
  textMuted: string;
  textFaint: string;
  primary: string;
  primaryFg: string;
  primaryText: string;
  primaryTint: string;
  blue: string;
  sky: string;
  skyText: string;
  amber: string;
  pink: string;
  purple: string;
  orange: string;
  red: string;
  redText: string;
  gray: string;
  chipSky: string;
  chipRed: string;
}

function readVar(styles: CSSStyleDeclaration, name: string, fallback = ''): string {
  const val = styles.getPropertyValue(name).trim();
  return val || fallback;
}

export function readLinearCanvasTokens(): LinearCanvasTokens {
  if (typeof document === 'undefined') {
    return {
      bgApp: '',
      bgSidebar: '',
      bgSurface: '',
      bgSurface2: '',
      bgActive: '',
      bgHover: '',
      bgTrack: '',
      border: '',
      borderStrong: '',
      text: '',
      textSecondary: '',
      textMuted: '',
      textFaint: '',
      primary: '',
      primaryFg: '',
      primaryText: '',
      primaryTint: '',
      blue: '',
      sky: '',
      skyText: '',
      amber: '',
      pink: '',
      purple: '',
      orange: '',
      red: '',
      redText: '',
      gray: '',
      chipSky: '',
      chipRed: '',
    };
  }
  const s = getComputedStyle(document.documentElement);
  return {
    bgApp: readVar(s, '--bg-app'),
    bgSidebar: readVar(s, '--bg-sidebar'),
    bgSurface: readVar(s, '--bg-surface'),
    bgSurface2: readVar(s, '--bg-surface-2'),
    bgActive: readVar(s, '--bg-active'),
    bgHover: readVar(s, '--bg-hover'),
    bgTrack: readVar(s, '--bg-track'),
    border: readVar(s, '--border'),
    borderStrong: readVar(s, '--border-strong'),
    text: readVar(s, '--text'),
    textSecondary: readVar(s, '--text-secondary'),
    textMuted: readVar(s, '--text-muted'),
    textFaint: readVar(s, '--text-faint'),
    primary: readVar(s, '--primary'),
    primaryFg: readVar(s, '--primary-fg'),
    primaryText: readVar(s, '--primary-text'),
    primaryTint: readVar(s, '--primary-tint'),
    blue: readVar(s, '--blue'),
    sky: readVar(s, '--sky'),
    skyText: readVar(s, '--sky-text'),
    amber: readVar(s, '--amber'),
    pink: readVar(s, '--pink'),
    purple: readVar(s, '--purple'),
    orange: readVar(s, '--orange'),
    red: readVar(s, '--red'),
    redText: readVar(s, '--red-text'),
    gray: readVar(s, '--gray'),
    chipSky: readVar(s, '--chip-sky'),
    chipRed: readVar(s, '--chip-red'),
  };
}

/** Subscribes a canvas component to live Linear theme token changes. */
export function useLinearCanvasTheme(): number {
  const [tick, setTick] = useState(0);
  useEffect(() => {
    const onThemeChanged = () => setTick((t) => t + 1);
    window.addEventListener('theme-changed', onThemeChanged);
    return () => window.removeEventListener('theme-changed', onThemeChanged);
  }, []);
  return tick;
}
