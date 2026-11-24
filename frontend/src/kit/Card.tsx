import React from 'react';
import { LinearCard } from '../components/linear/LinearCard';

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
    <LinearCard
      level="raised"
      onClick={onClick}
      className={`p-0 overflow-hidden ${
        accent ? '!border-[var(--color-accent)]' : ''
      } ${onClick ? 'cursor-pointer hover:border-[var(--color-border-strong)]' : ''} ${className}`}
    >
      {(title || subtitle || actions) && (
        <div className="flex items-center justify-between p-4 border-b border-[var(--color-border)]">
          <div>
            {title && <h3 className="text-xs font-medium text-[var(--color-fg)]">{title}</h3>}
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
    </LinearCard>
  );
}
