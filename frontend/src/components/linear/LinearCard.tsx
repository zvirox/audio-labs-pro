import React, { forwardRef } from 'react';

export interface LinearCardProps extends React.HTMLAttributes<HTMLDivElement> {
  level?: 1 | 2 | 3 | 'pane' | 'raised' | 'modal' | 'overlay';
}

export const LinearCard = forwardRef<HTMLDivElement, LinearCardProps>(({
  level = 1,
  className = '',
  children,
  ...props
}, ref) => {
  const normalizedLevel: 1 | 2 | 3 =
    typeof level === 'number'
      ? level
      : level === 'raised'
      ? 2
      : level === 'modal' || level === 'overlay'
      ? 3
      : 1;

  // Solid, flat, border-only surfaces per Linear specification. No shadow in dark mode.
  const levelStyles = {
    1: 'bg-[var(--bg-surface)] border-[var(--border)] shadow-[var(--shadow-card)]',
    2: 'bg-[var(--bg-surface-2)] border-[var(--border)] shadow-[var(--shadow-card)]',
    3: 'bg-[var(--bg-surface-2)] border-[var(--border-strong)] shadow-[var(--shadow-card)]',
  }[normalizedLevel];

  return (
    <div
      ref={ref}
      className={`rounded-lg border transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] hover:border-[var(--border-strong)] text-[var(--text)] ${levelStyles} ${className}`}
      {...props}
    >
      {children}
    </div>
  );
});

LinearCard.displayName = 'LinearCard';
