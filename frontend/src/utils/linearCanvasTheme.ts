import { useEffect, useState } from 'react';

export interface LinearCanvasTokens {
  bgApp: string;
  bgSurface: string;
  bgSurface2: string;
  bgBarIdle: string;
  border: string;
  borderStrong: string;
  text: string;
  textMuted: string;
  textFaint: string;
  textOnAccent: string;
  accentBlue: string;
  accentSky: string;
  accentGreen: string;
  accentRed: string;
  accentAmber: string;
  accentTeal: string;
  accentPink: string;
  accentPurple: string;
  accentOrange: string;
  tintBlue: string;
  tintSky: string;
  tintGreen: string;
  tintRed: string;
  tintAmber: string;
  tintPurple: string;
}

function readVar(styles: CSSStyleDeclaration, name: string, fallback = ''): string {
  const val = styles.getPropertyValue(name).trim();
  return val || fallback;
}

export function readLinearCanvasTokens(): LinearCanvasTokens {
  if (typeof document === 'undefined') {
    return {
      bgApp: '',
      bgSurface: '',
      bgSurface2: '',
      bgBarIdle: '',
      border: '',
      borderStrong: '',
      text: '',
      textMuted: '',
      textFaint: '',
      textOnAccent: '',
      accentBlue: '',
      accentSky: '',
      accentGreen: '',
      accentRed: '',
      accentAmber: '',
      accentTeal: '',
      accentPink: '',
      accentPurple: '',
      accentOrange: '',
      tintBlue: '',
      tintSky: '',
      tintGreen: '',
      tintRed: '',
      tintAmber: '',
      tintPurple: '',
    };
  }
  const s = getComputedStyle(document.documentElement);
  return {
    bgApp: readVar(s, '--bg-app'),
    bgSurface: readVar(s, '--bg-surface'),
    bgSurface2: readVar(s, '--bg-surface-2'),
    bgBarIdle: readVar(s, '--bg-bar-idle'),
    border: readVar(s, '--border'),
    borderStrong: readVar(s, '--border-strong'),
    text: readVar(s, '--text'),
    textMuted: readVar(s, '--text-muted'),
    textFaint: readVar(s, '--text-faint'),
    textOnAccent: readVar(s, '--text-on-accent'),
    accentBlue: readVar(s, '--accent-blue'),
    accentSky: readVar(s, '--accent-sky'),
    accentGreen: readVar(s, '--accent-green'),
    accentRed: readVar(s, '--accent-red'),
    accentAmber: readVar(s, '--accent-amber'),
    accentTeal: readVar(s, '--accent-teal'),
    accentPink: readVar(s, '--accent-pink'),
    accentPurple: readVar(s, '--accent-purple'),
    accentOrange: readVar(s, '--accent-orange'),
    tintBlue: readVar(s, '--tint-blue'),
    tintSky: readVar(s, '--tint-sky'),
    tintGreen: readVar(s, '--tint-green'),
    tintRed: readVar(s, '--tint-red'),
    tintAmber: readVar(s, '--tint-amber'),
    tintPurple: readVar(s, '--tint-purple'),
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
