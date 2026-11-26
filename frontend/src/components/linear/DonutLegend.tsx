import React from 'react';

export interface DonutLegendItem {
  label: string;
  value: string | number;
  color: string;
  percent?: number;
}

export interface DonutLegendProps {
  items: DonutLegendItem[];
  className?: string;
}

export const DonutLegend: React.FC<DonutLegendProps> = ({ items, className = '' }) => {
  return (
    <div className={`flex flex-col divide-y divide-dashed divide-border ${className}`}>
      {items.map((item, idx) => (
        <div
          key={`${item.label}-${idx}`}
          className="flex items-center justify-between gap-4 py-1.5 first:pt-0 last:pb-0 text-[13px]"
        >
          <div className="flex min-w-0 items-center gap-2">
            <span
              className="h-2.5 w-2.5 shrink-0 rounded-[2px]"
              style={{ backgroundColor: item.color }}
            />
            <span className="truncate font-medium text-fg-2">{item.label}</span>
          </div>
          <div className="flex shrink-0 items-center gap-2 font-sans tabular-nums">
            <span className="font-semibold text-fg">{item.value}</span>
            {item.percent !== undefined && (
              <span className="w-10 text-right text-[12px] text-muted">
                {item.percent}%
              </span>
            )}
          </div>
        </div>
      ))}
    </div>
  );
};
