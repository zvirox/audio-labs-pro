import React, { useEffect, useRef, useState } from 'react';

export interface MenuItem {
  id: string;
  label: string;
  icon?: React.ReactNode;
  shortcut?: string;
  danger?: boolean;
  disabled?: boolean;
  onClick?: () => void;
  dividerBefore?: boolean;
}

export interface MenuProps {
  trigger: React.ReactElement<{ onClick?: () => void }>;
  items: MenuItem[];
  align?: 'left' | 'right';
}

export function Menu({ trigger, items, align = 'left' }: MenuProps) {
  const [isOpen, setIsOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const triggerWrapRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (
        menuRef.current &&
        !menuRef.current.contains(e.target as Node) &&
        triggerWrapRef.current &&
        !triggerWrapRef.current.contains(e.target as Node)
      ) {
        setIsOpen(false);
      }
    }
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') setIsOpen(false);
    }
    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleKeyDown);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

  return (
    <div className="relative inline-block">
      <div
        ref={triggerWrapRef}
        onClick={() => setIsOpen((prev) => !prev)}
        className="inline-flex"
      >
        {trigger}
      </div>
      {isOpen && (
        <div
          ref={menuRef}
          role="menu"
          className={`absolute top-full z-40 mt-1.5 min-w-[200px] rounded-lg border border-[var(--border-strong)] bg-[var(--bg-surface)] p-1 shadow-[var(--shadow-card)] ${
            align === 'right' ? 'right-0' : 'left-0'
          }`}
        >
          {items.map((item) => (
            <React.Fragment key={item.id}>
              {item.dividerBefore && <div className="my-1 h-px bg-[var(--border)]" />}
              <button
                type="button"
                role="menuitem"
                disabled={item.disabled}
                onClick={() => {
                  if (item.disabled) return;
                  item.onClick?.();
                  setIsOpen(false);
                }}
                className={`flex w-full items-center justify-between gap-3 rounded-md px-2.5 py-1.5 text-left text-[13px] transition-colors duration-[var(--duration-quick)] disabled:opacity-50 disabled:pointer-events-none cursor-pointer ${
                  item.danger
                    ? 'text-[var(--accent-red)] hover:bg-[var(--tint-red)]'
                    : 'text-[var(--text)] hover:bg-[var(--bg-surface-2)]'
                }`}
              >
                <span className="flex items-center gap-2">
                  {item.icon && <span className="shrink-0 text-[var(--text-muted)]">{item.icon}</span>}
                  <span>{item.label}</span>
                </span>
                {item.shortcut && (
                  <kbd className="rounded border border-[var(--border)] bg-[var(--bg-surface-2)] px-1 py-0.5 font-mono text-[10px] text-[var(--text-muted)]">
                    {item.shortcut}
                  </kbd>
                )}
              </button>
            </React.Fragment>
          ))}
        </div>
      )}
    </div>
  );
}
