import React from 'react';

export interface CardProps {
  title?: React.ReactNode;
  subtitle?: React.ReactNode;
  actions?: React.ReactNode;
  footer?: React.ReactNode;
  children: React.ReactNode;
  accent?: boolean;
  className?: string;
  onClick?: () => void;
}

export function Card({
  title,
  subtitle,
  actions,
  footer,
  children,
  accent = false,
  className = '',
  onClick,
}: CardProps) {
  return (
    <div
      onClick={onClick}
      className={`rounded-xl bg-[var(--color-surface)] border transition-all ${
        accent
          ? 'border-[var(--color-accent)] shadow-md shadow-[var(--color-accent)]/10'
          : 'border-[var(--color-border)]'
      } ${onClick ? 'cursor-pointer hover:border-[var(--color-border-strong)]' : ''} ${className}`}
    >
      {(title || subtitle || actions) && (
        <div className="flex items-center justify-between p-3.5 border-b border-[var(--color-border)]">
          <div>
            {title && <h3 className="text-xs font-bold text-[var(--color-fg)]">{title}</h3>}
            {subtitle && (
              <p className="text-[11px] text-[var(--color-fg-muted)] mt-0.5">{subtitle}</p>
            )}
          </div>
          {actions && <div className="flex items-center gap-1.5">{actions}</div>}
        </div>
      )}

      <div className="p-4 text-xs text-[var(--color-fg)]">{children}</div>

      {footer && (
        <div className="p-3 bg-[var(--color-surface-2)] border-t border-[var(--color-border)] rounded-b-xl flex items-center justify-between text-xs">
          {footer}
        </div>
      )}
    </div>
  );
}
