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
      level={1}
      onClick={onClick}
      className={`flex flex-col overflow-hidden ${
        accent ? 'border-[var(--accent-blue)]' : ''
      } ${onClick ? 'cursor-pointer' : ''} ${className}`}
    >
      {(title || subtitle || actions) && (
        <div className="flex items-start justify-between gap-4 border-b border-[var(--border)] px-4 py-3">
          <div className="min-w-0">
            {title && (
              <h3 className="truncate text-[14px] font-semibold text-[var(--text)]">{title}</h3>
            )}
            {subtitle && (
              <p className="mt-0.5 truncate text-[12px] text-[var(--text-muted)]">{subtitle}</p>
            )}
          </div>
          {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
        </div>
      )}
      <div className="flex-1 p-4">{children}</div>
      {footer && (
        <div className="border-t border-[var(--border)] bg-[var(--bg-surface-2)] px-4 py-2.5 text-[12px] text-[var(--text-muted)]">
          {footer}
        </div>
      )}
    </LinearCard>
  );
}
