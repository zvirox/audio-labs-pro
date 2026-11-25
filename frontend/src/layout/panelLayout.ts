import type { PanelState, PanelRect } from '../ipc/types';

export interface CanvasBounds {
  width: number;
  height: number;
}

const MIN_W = 260;
const MIN_H = 140;
const GAP = 12;

/** Keep a panel rect fully inside the canvas so nothing renders off-screen. */
export function clampRect(rect: PanelRect, bounds: CanvasBounds): PanelRect {
  const width = Math.min(Math.max(rect.width, MIN_W), Math.max(MIN_W, bounds.width));
  const height = Math.min(Math.max(rect.height, MIN_H), Math.max(MIN_H, bounds.height));
  const maxX = Math.max(0, bounds.width - width);
  const maxY = Math.max(0, bounds.height - height);
  return {
    ...rect,
    width,
    height,
    x: Math.min(Math.max(rect.x, 0), maxX),
    y: Math.min(Math.max(rect.y, 0), maxY),
  };
}

export function clampPanels(panels: PanelState[], bounds: CanvasBounds): PanelState[] {
  if (bounds.width <= 0 || bounds.height <= 0) return panels;
  return panels.map((p) => ({ ...p, rect: clampRect(p.rect, bounds) }));
}

function overlaps(a: PanelRect, b: PanelRect): boolean {
  return a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
}

export function hasOverlap(panels: PanelState[]): boolean {
  const open = panels.filter((p) => p.open && !p.collapsed);
  for (let i = 0; i < open.length; i++) {
    for (let j = i + 1; j < open.length; j++) {
      if (overlaps(open[i].rect, open[j].rect)) return true;
    }
  }
  return false;
}

/**
 * Reflow open panels into a tidy, non-overlapping responsive grid inside the
 * canvas. Used on first load and on "Reset layout" so the default arrangement
 * never stacks panels on top of each other or pushes them off-screen.
 */
export function tidyPanels(panels: PanelState[], bounds: CanvasBounds): PanelState[] {
  if (bounds.width <= 0 || bounds.height <= 0) return panels;
  const open = panels.filter((p) => p.open);
  if (open.length === 0) return clampPanels(panels, bounds);

  // Choose a column count that keeps every row at least MIN_H tall so the
  // grid always fits the canvas without stacking panels on each other.
  const maxRows = Math.max(1, Math.floor((bounds.height - GAP) / (MIN_H + GAP)));
  const maxColsByWidth = Math.max(1, Math.floor((bounds.width - GAP) / (MIN_W + GAP)));
  const cols = Math.max(1, Math.min(maxColsByWidth, 3, Math.ceil(open.length / maxRows)));
  const rows = Math.ceil(open.length / cols);
  const cellW = (bounds.width - GAP * (cols + 1)) / cols;
  const cellH = (bounds.height - GAP * (rows + 1)) / rows;

  const rectByKind = new Map<string, PanelRect>();
  open.forEach((p, i) => {
    const col = i % cols;
    const row = Math.floor(i / cols);
    rectByKind.set(p.kind, {
      x: GAP + col * (cellW + GAP),
      y: GAP + row * (cellH + GAP),
      width: cellW,
      height: cellH,
    });
  });

  return panels.map((p) => {
    const tidy = rectByKind.get(p.kind);
    return tidy ? { ...p, rect: tidy } : { ...p, rect: clampRect(p.rect, bounds) };
  });
}

/** Rough canvas size before the DOM is measured (window minus fixed chrome). */
export function estimateCanvasBounds(): CanvasBounds {
  const chromeW = 240; // expanded sidebar width
  const chromeH = 40 + 44 + 40 + 56; // titlebar + breadcrumb + subtab + docked player
  return {
    width: Math.max(320, window.innerWidth - chromeW),
    height: Math.max(240, window.innerHeight - chromeH),
  };
}
