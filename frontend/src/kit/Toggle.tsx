import React, { forwardRef, useId } from 'react';

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
      ...props
    },
    ref,
  ) => {
    const id = useId();
    const trackClass = size === 'sm' ? 'h-4 w-7' : 'h-5 w-9';
    const thumbClass = size === 'sm' ? 'h-2.5 w-2.5' : 'h-3.5 w-3.5';
    const translateClass =
      size === 'sm'
        ? checked
          ? 'translate-x-3.5'
          : 'translate-x-0.5'
        : checked
        ? 'translate-x-[18px]'
        : 'translate-x-0.5';

    return (
      <div className={`flex items-center justify-between gap-3 ${className}`}>
        {(label || description) && (
          <label htmlFor={id} className="flex min-w-0 flex-col cursor-pointer">
            {label && <span className="text-[13px] font-medium text-fg-2">{label}</span>}
            {description && (
              <span className="text-[12px] text-muted">{description}</span>
            )}
          </label>
        )}
        <button
          ref={ref}
          id={id}
          type="button"
          role="switch"
          aria-checked={checked}
          disabled={disabled}
          onClick={() => onChange(!checked)}
          className={`relative inline-flex shrink-0 items-center rounded-md border transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-text disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer ${trackClass} ${
            checked
              ? 'border-primary bg-primary'
              : 'border-border-strong bg-track'
          }`}
          {...props}
        >
          <span
            className={`inline-block rounded transition-transform duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] ${thumbClass} ${translateClass} ${
              checked ? 'bg-primary-fg' : 'bg-fg'
            }`}
          />
        </button>
      </div>
    );
  },
);

Toggle.displayName = 'Toggle';
