import React from 'react';

export interface ProgressRowProps {
  label: string;
  value: number;
  max?: number;
  displayValue?: string;
  color?: string;
  className?: string;
}

export const ProgressRow: React.FC<ProgressRowProps> = ({
  label,
  value,
  max = 100,
  displayValue,
  color = 'var(--primary)',
  className = '',
}) => {
  const percent = max > 0 ? Math.min(100, Math.max(0, (value / max) * 100)) : 0;

  return (
    <div className={`flex flex-col gap-1.5 ${className}`}>
      <div className="flex items-center justify-between text-[13px]">
        <span className="truncate font-medium text-fg-2">{label}</span>
        <span className="font-sans text-[12px] text-muted tabular-nums">
          {displayValue ?? `${Math.round(percent)}%`}
        </span>
      </div>
      <div className="h-1 w-full overflow-hidden rounded bg-track">
        <div
          className="h-full rounded transition-all duration-[var(--duration-standard)] [transition-timing-function:var(--ease-out-expo)]"
          style={{ width: `${percent}%`, backgroundColor: color }}
        />
      </div>
    </div>
  );
};
