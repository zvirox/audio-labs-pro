import React, { useEffect, useRef, useState } from 'react';
import { GripHorizontal, X } from 'lucide-react';
import type { PanelKind, PanelState } from '../ipc/types';
import type { CanvasBounds } from '../layout/panelLayout';

interface PanelContainerProps {
  panel: PanelState;
  bounds: CanvasBounds;
  onUpdate: (next: PanelState) => void;
  onClose: (kind: PanelKind) => void;
  onFocus: (kind: PanelKind) => void;
  children: React.ReactNode;
}

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
    startX: 0,
    startY: 0,
  });
  const resizeStartRef = useRef<{ mouseX: number; mouseY: number; startW: number; startH: number }>({
    mouseX: 0,
    mouseY: 0,
    startW: 0,
    startH: 0,
  });

  useEffect(() => {
    if (!isDragging && !isResizing) return;

    function handleMouseMove(e: MouseEvent) {
      if (isDragging) {
        const dx = e.clientX - dragStartRef.current.mouseX;
        const dy = e.clientY - dragStartRef.current.mouseY;
        const maxX = Math.max(0, bounds.width - panel.rect.width);
        const maxY = Math.max(0, bounds.height - panel.rect.height);
        const nextX = Math.max(0, Math.min(maxX, dragStartRef.current.startX + dx));
        const nextY = Math.max(0, Math.min(maxY, dragStartRef.current.startY + dy));
        onUpdate({ ...panel, rect: { ...panel.rect, x: nextX, y: nextY } });
      } else if (isResizing) {
        const dx = e.clientX - resizeStartRef.current.mouseX;
        const dy = e.clientY - resizeStartRef.current.mouseY;
        const nextW = Math.max(260, Math.min(bounds.width - panel.rect.x, resizeStartRef.current.startW + dx));
        const nextH = Math.max(180, Math.min(bounds.height - panel.rect.y, resizeStartRef.current.startH + dy));
        onUpdate({ ...panel, rect: { ...panel.rect, width: nextW, height: nextH } });
      }
    }

    function handleMouseUp() {
      setIsDragging(false);
      setIsResizing(false);
    }

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, isResizing, panel, bounds, onUpdate]);

  return (
    <section
      onMouseDown={() => onFocus(panel.kind)}
      style={{
        left: panel.rect.x,
        top: panel.rect.y,
        width: panel.rect.width,
        height: panel.rect.height,
        zIndex: panel.z,
      }}
      className="absolute flex flex-col overflow-hidden rounded-lg border border-border-strong bg-surface text-fg shadow-card"
    >
      <div
        onMouseDown={(e) => {
          e.stopPropagation();
          onFocus(panel.kind);
          dragStartRef.current = {
            mouseX: e.clientX,
            mouseY: e.clientY,
            startX: panel.rect.x,
            startY: panel.rect.y,
          };
          setIsDragging(true);
        }}
        className="flex h-9 shrink-0 cursor-move items-center justify-between border-b border-border bg-surface-2 px-3 select-none"
      >
        <div className="flex items-center gap-2 text-[12px] font-semibold">
          <GripHorizontal className="h-3.5 w-3.5 text-muted" />
          <span className="capitalize">{panel.kind.replace('_', ' ')}</span>
        </div>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onClose(panel.kind);
          }}
          aria-label={`Close ${panel.kind} panel`}
          className="inline-flex h-6 w-6 items-center justify-center rounded text-muted hover:bg-hover hover:text-fg cursor-pointer"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-3">{children}</div>

      <div
        onMouseDown={(e) => {
          e.stopPropagation();
          onFocus(panel.kind);
          resizeStartRef.current = {
            mouseX: e.clientX,
            mouseY: e.clientY,
            startW: panel.rect.width,
            startH: panel.rect.height,
          };
          setIsResizing(true);
        }}
        title="Resize panel"
        className="absolute bottom-0 right-0 h-3 w-3 cursor-nwse-resize border-r-2 border-b-2 border-border-strong"
      />
    </section>
  );
}
