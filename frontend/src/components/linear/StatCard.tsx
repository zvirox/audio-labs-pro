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
      ? 'bg-chip-sky text-sky-text'
      : delta?.trend === 'down'
      ? 'bg-chip-red text-red-text'
      : 'bg-surface-2 text-muted';

  return (
    <LinearCard level={1} className={`flex flex-col justify-between gap-2 p-4 ${className}`}>
      <div className="flex items-center justify-between gap-2">
        <span className="text-[12px] font-medium uppercase tracking-wider text-muted">
          {label}
        </span>
        {icon && (
          <div className="flex h-7 w-7 items-center justify-center rounded-md border border-border bg-primary-tint text-primary-text">
            {icon}
          </div>
        )}
      </div>

      <div className="flex items-baseline justify-between gap-2">
        <span className="whitespace-nowrap font-sans text-[24px] xl:text-[28px] font-semibold leading-none tracking-tight text-fg tabular-nums">
          {value}
        </span>
        {delta && (
          <span
            className={`inline-flex shrink-0 whitespace-nowrap items-center rounded border border-border px-1.5 py-0.5 text-[11px] font-medium leading-none tabular-nums ${deltaStyle}`}
          >
            {delta.value}
          </span>
        )}
      </div>

      {subtitle && (
        <span className="truncate text-[12px] text-muted">{subtitle}</span>
      )}
    </LinearCard>
  );
};
