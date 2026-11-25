import React from 'react';
import { LinearCard } from './LinearCard';

export interface StatCardProps {
  label: string;
  value: string | number;
  icon?: React.ReactNode;
  delta?: {
    value: string;
    trend: 'up' | 'down' | 'neutral';
  };
  subtitle?: string;
  className?: string;
}

export const StatCard: React.FC<StatCardProps> = ({
  label,
  value,
  icon,
  delta,
  subtitle,
  className = '',
}) => {
  const deltaStyle =
    delta?.trend === 'up'
      ? 'bg-[var(--tint-green)] text-[var(--accent-green)]'
      : delta?.trend === 'down'
      ? 'bg-[var(--tint-red)] text-[var(--accent-red)]'
      : 'bg-[var(--bg-surface-2)] text-[var(--text-muted)]';

  return (
    <LinearCard level={1} className={`flex flex-col justify-between gap-2 p-4 ${className}`}>
      <div className="flex items-center justify-between gap-2">
        <span className="text-[12px] font-medium uppercase tracking-wider text-[var(--text-muted)]">
          {label}
        </span>
        {icon && (
          <div className="flex h-7 w-7 items-center justify-center rounded-md border border-[var(--border)] bg-[var(--tint-blue)] text-[var(--accent-blue)]">
            {icon}
          </div>
        )}
      </div>

      <div className="flex items-baseline justify-between gap-2">
        <span className="whitespace-nowrap font-sans text-[24px] xl:text-[28px] font-semibold leading-none tracking-tight text-[var(--text)] tabular-nums">
          {value}
        </span>
        {delta && (
          <span
            className={`inline-flex shrink-0 whitespace-nowrap items-center rounded border border-[var(--border)] px-1.5 py-0.5 text-[11px] font-medium leading-none tabular-nums ${deltaStyle}`}
          >
            {delta.value}
          </span>
        )}
      </div>

      {subtitle && (
        <span className="truncate text-[12px] text-[var(--text-muted)]">{subtitle}</span>
      )}
    </LinearCard>
  );
};
