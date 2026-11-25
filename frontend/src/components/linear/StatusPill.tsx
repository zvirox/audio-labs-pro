import React from 'react';

export interface StatusPillProps {
  variant?: 'success' | 'warning' | 'danger' | 'info' | 'neutral';
  label: string;
  className?: string;
}

const VARIANT_STYLES: Record<NonNullable<StatusPillProps['variant']>, { chip: string; dot: string }> = {
  success: {
    chip: 'bg-[var(--tint-green)] text-[var(--accent-green)] border-[var(--border)]',
    dot: 'bg-[var(--accent-green)]',
  },
  warning: {
    chip: 'bg-[var(--tint-amber)] text-[var(--accent-amber)] border-[var(--border)]',
    dot: 'bg-[var(--accent-amber)]',
  },
  danger: {
    chip: 'bg-[var(--tint-red)] text-[var(--accent-red)] border-[var(--border)]',
    dot: 'bg-[var(--accent-red)]',
  },
  info: {
    chip: 'bg-[var(--tint-blue)] text-[var(--accent-blue)] border-[var(--border)]',
    dot: 'bg-[var(--accent-blue)]',
  },
  neutral: {
    chip: 'bg-[var(--bg-surface-2)] text-[var(--text-muted)] border-[var(--border)]',
    dot: 'bg-[var(--text-muted)]',
  },
};

export const StatusPill: React.FC<StatusPillProps> = ({
  variant = 'neutral',
  label,
  className = '',
}) => {
  const styles = VARIANT_STYLES[variant];
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded border px-2 py-0.5 text-[11px] font-medium leading-none tracking-wide ${styles.chip} ${className}`}
    >
      <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${styles.dot}`} />
      <span>{label}</span>
    </span>
  );
};
