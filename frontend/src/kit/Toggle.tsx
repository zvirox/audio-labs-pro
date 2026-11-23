import React, { forwardRef } from 'react';

export interface ToggleProps extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'onChange'> {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  description?: string;
  size?: 'sm' | 'md';
}

export const Toggle = forwardRef<HTMLButtonElement, ToggleProps>(
  (
    {
      checked,
      onChange,
      label,
      description,
      size = 'md',
      disabled = false,
      className = '',
      id,
      ...props
    },
    ref
  ) => {
    const toggleId = id || (label ? `toggle-${label.toLowerCase().replace(/\s+/g, '-')}` : undefined);

    const switchSizes = {
      sm: { track: 'w-7 h-4', thumb: 'w-4 h-4', translate: 'translate-x-3' },
      md: { track: 'w-10 h-5.5', thumb: 'w-4 h-4', translate: 'translate-x-4.5' },
    };

    const currentSize = switchSizes[size];

    return (
      <div className={`inline-flex items-center justify-between gap-3 ${className}`}>
        {(label || description) && (
          <label
            htmlFor={toggleId}
            className={`cursor-pointer select-none text-left ${disabled ? 'opacity-40 pointer-events-none' : ''}`}
          >
            {label && <div className="text-xs font-medium text-[var(--color-fg)]">{label}</div>}
            {description && (
              <div className="text-[11px] text-[var(--color-fg-muted)] mt-0.5">{description}</div>
            )}
          </label>
        )}
        <button
          ref={ref}
          id={toggleId}
          type="button"
          role="switch"
          aria-checked={checked}
          disabled={disabled}
          onClick={() => onChange(!checked)}
          className={`relative inline-flex flex-shrink-0 cursor-pointer rounded-full transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus)] disabled:opacity-40 disabled:cursor-not-allowed ${
 currentSize.track
 } ${checked ? 'bg-[var(--color-accent)]' : 'bg-[var(--color-surface-3)] border border-[var(--color-border)]'}`}
          {...props}
        >
          <span
            className={`pointer-events-none inline-block rounded-full bg-white shadow-md transform ring-0 transition-transform duration-150 ease-out my-auto ml-0.5 ${
              currentSize.thumb
            } ${checked ? `${currentSize.translate} bg-[var(--color-accent-fg)]` : 'translate-x-0 bg-[var(--color-fg-muted)]'}`}
          />
        </button>
      </div>
    );
  }
);

Toggle.displayName = 'Toggle';
