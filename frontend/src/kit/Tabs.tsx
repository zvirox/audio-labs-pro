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
  size = 'sm',
}: TabsProps) {
  const sizeStyles = {
    sm: 'text-xs h-7 px-2.5 gap-1.5',
    md: 'text-xs h-8 px-3.5 gap-2',
  };

  if (variant === 'segmented') {
    return (
      <div
        role="tablist"
        className={`inline-flex p-0.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] select-none ${className}`}
      >
        {items.map((tab) => {
          const isActive = tab.id === activeId;
          return (
            <button
              key={tab.id}
              role="tab"
              aria-selected={isActive}
              disabled={tab.disabled}
              onClick={() => onChange(tab.id)}
              className={`inline-flex items-center justify-center font-medium rounded-md transition-all cursor-pointer ${
                sizeStyles[size]
              } ${
                isActive
                  ? 'bg-[var(--color-surface)] text-[var(--color-fg)] shadow-xs font-semibold'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-fg)]'
              } disabled:opacity-40 disabled:pointer-events-none`}
            >
              {tab.icon}
              <span>{tab.label}</span>
              {tab.badge !== undefined && (
                <span className="px-1.5 py-0.2 rounded-full text-[10px] font-mono bg-[var(--color-surface-3)] text-[var(--color-fg-muted)]">
                  {tab.badge}
                </span>
              )}
            </button>
          );
        })}
      </div>
    );
  }

  // Subtab horizontal bar style (standard in 40px top bar). Single line, scrolls
  // horizontally instead of wrapping so tab labels never stack or duplicate.
  return (
    <div
      role="tablist"
      className={`inline-flex flex-nowrap items-center gap-1 select-none overflow-x-auto [scrollbar-width:none] ${className}`}
    >
      {items.map((tab) => {
        const isActive = tab.id === activeId;
        return (
          <button
            key={tab.id}
            role="tab"
            aria-selected={isActive}
            disabled={tab.disabled}
            onClick={() => onChange(tab.id)}
            className={`relative inline-flex shrink-0 items-center justify-center whitespace-nowrap font-medium transition-colors cursor-pointer rounded-md ${
              sizeStyles[size]
            } ${
              isActive
                ? 'bg-[var(--color-surface)] text-[var(--color-fg)] border border-[var(--color-border)]'
                : 'text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] hover:bg-[var(--color-surface-2)]'
            } disabled:opacity-40 disabled:pointer-events-none`}
          >
            {tab.icon}
            <span>{tab.label}</span>
            {tab.badge !== undefined && (
              <span className="px-1.5 py-0.2 rounded-full text-[10px] font-mono bg-[var(--color-surface-3)] text-[var(--color-fg-muted)]">
                {tab.badge}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}
