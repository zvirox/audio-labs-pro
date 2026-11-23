import React, { useState, useRef, useEffect } from 'react';

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
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    }
    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [isOpen]);

  // Keyboard operability: Esc closes and returns focus to the trigger;
  // Arrow keys move between enabled items.
  useEffect(() => {
    if (!isOpen) return;
    function handleKeyDown(e: KeyboardEvent) {
      const itemsEls = Array.from(
        menuRef.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not([disabled])') ?? []
      );
      if (e.key === 'Escape') {
        e.preventDefault();
        setIsOpen(false);
        triggerWrapRef.current?.querySelector<HTMLButtonElement>('button')?.focus();
        return;
      }
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        if (itemsEls.length === 0) return;
        const idx = itemsEls.indexOf(document.activeElement as HTMLButtonElement);
        const next =
          e.key === 'ArrowDown'
            ? (idx + 1) % itemsEls.length
            : (idx - 1 + itemsEls.length) % itemsEls.length;
        itemsEls[next]?.focus();
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    // Move focus into the menu when it opens.
    const first = menuRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]:not([disabled])');
    first?.focus();
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen]);

  return (
    <div className="relative inline-block text-left" ref={menuRef}>
      <div ref={triggerWrapRef} className="inline-block">
        {React.cloneElement(trigger, {
          onClick: () => setIsOpen((prev) => !prev),
        })}
      </div>

      {isOpen && (
        <div
          role="menu"
          className={`absolute z-50 mt-1 min-w-[180px] rounded-xl bg-[var(--color-surface)] border border-[var(--color-border)] shadow-xl p-1 animate-in fade-in zoom-in-95 duration-100 ${
            align === 'right' ? 'right-0' : 'left-0'
          }`}
        >
          {items.map((item) => (
            <React.Fragment key={item.id}>
              {item.dividerBefore && (
                <div className="my-1 h-[1px] bg-[var(--color-border)]" />
              )}
              <button
                type="button"
                role="menuitem"
                disabled={item.disabled}
                onClick={() => {
                  item.onClick?.();
                  setIsOpen(false);
                }}
                className={`w-full flex items-center justify-between gap-2 px-2.5 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer disabled:opacity-40 disabled:pointer-events-none ${
                  item.danger
                    ? 'text-red-400 hover:bg-red-500/20'
                    : 'text-[var(--color-fg)] hover:bg-[var(--color-surface-2)]'
                }`}
              >
                <div className="flex items-center gap-2">
                  {item.icon}
                  <span>{item.label}</span>
                </div>
                {item.shortcut && (
                  <kbd className="text-[10px] font-mono text-[var(--color-fg-subtle)]">
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
