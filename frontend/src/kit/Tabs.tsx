import React from 'react';

export interface TabItem {
  id: string;
  label: string;
  icon?: React.ReactNode;
  badge?: string | number;
  disabled?: boolean;
}

export interface TabsProps {
  items: TabItem[];
  activeId: string;
  onChange: (id: string) => void;
  variant?: 'subtab' | 'pills' | 'segmented';
  className?: string;
  size?: 'sm' | 'md';
}

export function Tabs({
  items,
  activeId,
  onChange,
  variant = 'subtab',
  className = '',
  size = 'md',
}: TabsProps) {
  const heightClass = size === 'sm' ? 'h-7 text-[12px] px-2.5' : 'h-8 text-[13px] px-3';

  if (variant === 'segmented' || variant === 'pills') {
    return (
      <div
        role="tablist"
        className={`inline-flex items-center gap-1 rounded-md border border-[var(--border)] bg-[var(--bg-surface-2)] p-0.5 ${className}`}
      >
        {items.map((item) => {
          const isActive = item.id === activeId;
          return (
            <button
              key={item.id}
              type="button"
              role="tab"
              aria-selected={isActive}
              disabled={item.disabled}
              onClick={() => !item.disabled && onChange(item.id)}
              className={`inline-flex items-center gap-1.5 rounded font-medium transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] disabled:opacity-40 cursor-pointer ${heightClass} ${
                isActive
                  ? 'border border-[var(--border)] bg-[var(--bg-surface)] text-[var(--text)]'
                  : 'border border-transparent text-[var(--text-muted)] hover:text-[var(--text)]'
              }`}
            >
              {item.icon}
              <span>{item.label}</span>
              {item.badge !== undefined && (
                <span className="rounded border border-[var(--border)] bg-[var(--bg-surface-2)] px-1.5 py-0.5 text-[10px] font-medium leading-none tabular-nums text-[var(--text-muted)]">
                  {item.badge}
                </span>
              )}
            </button>
          );
        })}
      </div>
    );
  }

  return (
    <div
      role="tablist"
      className={`flex items-center gap-1 border-b border-[var(--border)] ${className}`}
    >
      {items.map((item) => {
        const isActive = item.id === activeId;
        return (
          <button
            key={item.id}
            type="button"
            role="tab"
            aria-selected={isActive}
            disabled={item.disabled}
            onClick={() => !item.disabled && onChange(item.id)}
            className={`relative -mb-px inline-flex items-center gap-1.5 border-b-2 px-3 py-2 text-[13px] font-medium transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] disabled:opacity-40 cursor-pointer ${
              isActive
                ? 'border-[var(--accent-blue)] text-[var(--text)]'
                : 'border-transparent text-[var(--text-muted)] hover:text-[var(--text)]'
            }`}
          >
            {item.icon}
            <span>{item.label}</span>
            {item.badge !== undefined && (
              <span className="rounded border border-[var(--border)] bg-[var(--bg-surface-2)] px-1.5 py-0.5 text-[10px] font-medium leading-none tabular-nums text-[var(--text-muted)]">
                {item.badge}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}
