import React, { useEffect, useRef } from 'react';
import { X } from 'lucide-react';
import { Button } from './Button';
import { LinearCard } from '../components/linear/LinearCard';

export interface DialogProps {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
  size?: 'sm' | 'md' | 'lg' | 'xl';
}

export function Dialog({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  size = 'md',
}: DialogProps) {
  const dialogRef = useRef<HTMLDivElement>(null);

  // Close on Escape key press
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape' && open) {
        onClose();
      }
    }
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  const sizeWidths = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-lg',
    xl: 'max-w-2xl',
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 animate-in fade-in duration-150"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <LinearCard
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="dialog-title"
        level="modal"
        className={`w-full ${sizeWidths[size]} p-0 overflow-hidden flex flex-col max-h-[90vh]`}
      >
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-[var(--color-border)]">
          <div>
            <h2 id="dialog-title" className="text-sm font-medium text-[var(--color-fg)]">
              {title}
            </h2>
            {description && (
              <p className="text-xs text-[var(--color-fg-muted)] mt-0.5">{description}</p>
            )}
          </div>
          <Button
            variant="icon"
            size="xs"
            onClick={onClose}
            aria-label="Close dialog"
            className="text-[var(--color-fg-muted)] hover:text-[var(--color-fg)]"
          >
            <X className="w-4 h-4" />
          </Button>
        </div>

        {/* Body content */}
        <div className="p-5 overflow-y-auto flex-1 text-xs text-[var(--color-fg)] space-y-4">
          {children}
        </div>

        {/* Optional footer */}
        {footer && (
          <div className="flex items-center justify-end gap-2 p-4 bg-[var(--color-surface-2)] border-t border-[var(--color-border)]">
            {footer}
          </div>
        )}
      </LinearCard>
    </div>
  );
}
