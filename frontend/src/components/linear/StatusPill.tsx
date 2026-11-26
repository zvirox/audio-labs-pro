import React from 'react';

export interface StatusPillProps {
  variant?: 'success' | 'warning' | 'danger' | 'info' | 'neutral';
  label: string;
  className?: string;
}

const VARIANT_STYLES: Record<NonNullable<StatusPillProps['variant']>, { chip: string; dot: string }> = {
  success: {
    chip: 'bg-chip-sky text-sky-text border-border',
    dot: 'bg-sky',
  },
  warning: {
    chip: 'bg-surface-2 text-amber border-border',
    dot: 'bg-amber',
  },
  danger: {
    chip: 'bg-chip-red text-red-text border-border',
    dot: 'bg-red',
  },
  info: {
    chip: 'bg-primary-tint text-primary-text border-border',
    dot: 'bg-primary',
  },
  neutral: {
    chip: 'bg-surface-2 text-muted border-border',
    dot: 'bg-faint',
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
