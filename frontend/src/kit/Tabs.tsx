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
        className={`inline-flex items-center gap-1 rounded-md border border-border bg-surface-2 p-0.5 ${className}`}
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
                  ? 'border border-border-strong bg-active text-fg'
                  : 'border border-transparent text-muted hover:text-fg'
              }`}
            >
              {item.icon}
              <span>{item.label}</span>
              {item.badge !== undefined && (
                <span className="rounded border border-border bg-surface-2 px-1.5 py-0.5 text-[10px] font-medium leading-none tabular-nums text-muted">
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
      className={`flex items-center gap-1 border-b border-border ${className}`}
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
                ? 'border-primary-text text-primary-text'
                : 'border-transparent text-muted hover:text-fg'
            }`}
          >
            {item.icon}
            <span>{item.label}</span>
            {item.badge !== undefined && (
              <span className="rounded border border-border bg-surface-2 px-1.5 py-0.5 text-[10px] font-medium leading-none tabular-nums text-muted">
                {item.badge}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}
