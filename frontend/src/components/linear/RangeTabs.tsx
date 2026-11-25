import React from 'react';

export interface RangeTabsProps {
  options: { id: string; label: string }[];
  value: string;
  onChange: (id: string) => void;
  className?: string;
}

export const RangeTabs: React.FC<RangeTabsProps> = ({
  options,
  value,
  onChange,
  className = '',
}) => {
  return (
    <div
      className={`inline-flex items-center rounded-md border border-[var(--border)] bg-[var(--bg-surface-2)] p-0.5 ${className}`}
      role="tablist"
    >
      {options.map((opt) => {
        const isActive = opt.id === value;
        return (
          <button
            key={opt.id}
            type="button"
            role="tab"
            aria-selected={isActive}
            onClick={() => onChange(opt.id)}
            className={`rounded px-2.5 py-1 text-[12px] font-medium transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] cursor-pointer ${
              isActive
                ? 'bg-[var(--bg-surface)] text-[var(--text)] border border-[var(--border)]'
                : 'border border-transparent text-[var(--text-muted)] hover:text-[var(--text)]'
            }`}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
};
