import React, { useState, useRef, useEffect } from 'react';
import {
  ChevronDown,
  ChevronUp,
  X,
  Maximize2,
  Minimize2,
  Sliders,
  Play,
  Layers,
  FileAudio,
  Mic,
  Sparkles,
  Cpu,
  DownloadCloud,
  Bot,
  Info,
} from 'lucide-react';
import type { PanelState, PanelKind } from '../ipc/types';
import type { CanvasBounds } from '../layout/panelLayout';

interface PanelContainerProps {
  panel: PanelState;
  bounds?: CanvasBounds;
  onUpdate: (updated: PanelState) => void;
  onClose: (kind: PanelKind) => void;
  onFocus: (kind: PanelKind) => void;
  children: React.ReactNode;
}

const PANEL_ICONS: Record<PanelKind, React.ComponentType<{ className?: string }>> = {
  mixer: Sliders,
  transport: Play,
  timeline: Layers,
  inspector: Info,
  media_bin: FileAudio,
  voice_lab: Mic,
  effects: Sparkles,
  models: Cpu,
  export_queue: DownloadCloud,
  ai_activity: Bot,
};

const PANEL_TITLES: Record<PanelKind, string> = {
  mixer: 'Master & Track Mixer',
  transport: 'Transport Controls',
  timeline: 'Multitrack Timeline',
  inspector: 'Clip & Track Inspector',
  media_bin: 'Media Bin & Sources',
  voice_lab: 'Voice Lab & Cloner',
  effects: 'DSP Effects Chain',
  models: 'Neural Voice Models',
  export_queue: 'Render & Export Queue',
  ai_activity: 'MCP Autonomous Agent Logs',
};

export function PanelContainer({
  panel,
  bounds,
  onUpdate,
  onClose,
  onFocus,
  children,
}: PanelContainerProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [isResizing, setIsResizing] = useState(false);
  const dragStartRef = useRef<{ mouseX: number; mouseY: number; startX: number; startY: number }>({
    mouseX: 0,
    mouseY: 0,
    startX: panel.rect.x,
    startY: panel.rect.y,
  });
  const resizeStartRef = useRef<{ mouseX: number; mouseY: number; startW: number; startH: number }>({
    mouseX: 0,
    mouseY: 0,
    startW: panel.rect.width,
    startH: panel.rect.height,
  });

  const Icon = PANEL_ICONS[panel.kind] || Info;
  const title = PANEL_TITLES[panel.kind] || panel.kind;

  // Dragging logic
  const handleMouseDownHeader = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('button')) return;
    onFocus(panel.kind);
    setIsDragging(true);
    dragStartRef.current = {
      mouseX: e.clientX,
      mouseY: e.clientY,
      startX: panel.rect.x,
      startY: panel.rect.y,
    };
  };

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      const deltaX = e.clientX - dragStartRef.current.mouseX;
      const deltaY = e.clientY - dragStartRef.current.mouseY;
      const maxX = bounds ? Math.max(0, bounds.width - panel.rect.width) : Infinity;
      const maxY = bounds ? Math.max(0, bounds.height - panel.rect.height) : Infinity;
      const newX = Math.min(Math.max(0, dragStartRef.current.startX + deltaX), maxX);
      const newY = Math.min(Math.max(0, dragStartRef.current.startY + deltaY), maxY);

      onUpdate({
        ...panel,
        rect: {
          ...panel.rect,
          x: newX,
          y: newY,
        },
      });
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, panel, onUpdate, bounds]);

  // Resizing logic
  const handleMouseDownResize = (e: React.MouseEvent) => {
    e.stopPropagation();
    onFocus(panel.kind);
    setIsResizing(true);
    resizeStartRef.current = {
      mouseX: e.clientX,
      mouseY: e.clientY,
      startW: panel.rect.width,
      startH: panel.rect.height,
    };
  };

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      const deltaX = e.clientX - resizeStartRef.current.mouseX;
      const deltaY = e.clientY - resizeStartRef.current.mouseY;
      const maxW = bounds ? Math.max(260, bounds.width - panel.rect.x) : Infinity;
      const maxH = bounds ? Math.max(140, bounds.height - panel.rect.y) : Infinity;
      const newW = Math.min(Math.max(260, resizeStartRef.current.startW + deltaX), maxW);
      const newH = Math.min(Math.max(140, resizeStartRef.current.startH + deltaY), maxH);

      onUpdate({
        ...panel,
        rect: {
          ...panel.rect,
          width: newW,
          height: newH,
        },
      });
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizing, panel, onUpdate, bounds]);

  if (!panel.open) return null;

  return (
    <div
      onMouseDown={() => onFocus(panel.kind)}
      style={{
        transform: `translate3d(${panel.rect.x}px, ${panel.rect.y}px, 0)`,
        width: `${panel.rect.width}px`,
        height: panel.collapsed ? '36px' : `${panel.rect.height}px`,
        zIndex: panel.z,
      }}
      className={`absolute top-0 left-0 rounded-xl bg-[var(--color-surface)] border border-[var(--color-border)] shadow-2xl overflow-hidden flex flex-col transition-shadow ${
 isDragging ? ' ring-1 ring-[var(--color-accent)]' : ''
 }`}
    >
      {/* Title Bar / Drag Header */}
      <div
        onMouseDown={handleMouseDownHeader}
        className="h-9 px-3 bg-[var(--color-surface-2)] border-b border-[var(--color-border)] flex items-center justify-between cursor-move select-none"
      >
        <div className="flex items-center gap-2 text-xs font-medium text-[var(--color-fg)] truncate">
          <Icon className="w-4 h-4 text-[var(--color-accent)] flex-shrink-0" />
          <span className="truncate">{title}</span>
        </div>

        <div className="flex items-center gap-1">
          <button
            onClick={() => onUpdate({ ...panel, collapsed: !panel.collapsed })}
            className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] hover:bg-[var(--color-surface-3)]"
            title={panel.collapsed ? 'Expand panel' : 'Collapse panel'}
          >
            {panel.collapsed ? <ChevronDown className="w-4 h-4" /> : <ChevronUp className="w-4 h-4" />}
          </button>
          <button
            onClick={() =>
              onUpdate({
                ...panel,
                docked: panel.docked ? null : 'right',
              })
            }
            className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] hover:bg-[var(--color-surface-3)]"
            title={panel.docked ? 'Undock panel' : 'Dock panel right'}
          >
            {panel.docked ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
          </button>
          <button
            onClick={() => onClose(panel.kind)}
            className="p-1 rounded text-[var(--color-fg-muted)] hover:text-red-400 hover:bg-[var(--color-surface-3)]"
            title="Close panel"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Body Content */}
      {!panel.collapsed && (
        <div className="flex-1 overflow-auto p-4 text-xs text-[var(--color-fg)] relative">
          {children}

          {/* Bottom-right corner resize handle */}
          <div
            onMouseDown={handleMouseDownResize}
            className="absolute bottom-1 right-1 w-4 h-4 cursor-se-resize text-[var(--color-fg-subtle)] hover:text-[var(--color-accent)]"
            title="Resize panel"
          >
            <svg viewBox="0 0 6 6" className="w-full h-full fill-current">
              <circle cx="5" cy="5" r="0.75" />
              <circle cx="5" cy="2.5" r="0.75" />
              <circle cx="2.5" cy="5" r="0.75" />
            </svg>
          </div>
        </div>
      )}
    </div>
  );
}
