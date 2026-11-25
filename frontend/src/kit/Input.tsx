import React, { forwardRef, useId } from 'react';
import { X } from 'lucide-react';

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  helperText?: string;
  errorMessage?: string;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
  onClear?: () => void;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  (
    {
      label,
      helperText,
      errorMessage,
      leftIcon,
      rightIcon,
      onClear,
      className = '',
      id,
      value,
      ...props
    },
    ref,
  ) => {
    const autoId = useId();
    const inputId = id ?? autoId;
    const hasValue = value !== undefined && value !== '';

    return (
      <div className={`flex flex-col gap-1.5 ${className}`}>
        {label && (
          <label
            htmlFor={inputId}
            className="text-[12px] font-medium text-[var(--text-muted)]"
          >
            {label}
          </label>
        )}
        <div className="relative flex items-center">
          {leftIcon && (
            <span className="pointer-events-none absolute left-2.5 text-[var(--text-muted)]">
              {leftIcon}
            </span>
          )}
          <input
            ref={ref}
            id={inputId}
            value={value}
            className={`h-8 w-full rounded-md border bg-[var(--bg-surface-2)] text-[13px] text-[var(--text)] placeholder:text-[var(--text-faint)] transition-colors duration-[var(--duration-quick)] focus:outline-none focus:ring-2 focus:ring-[var(--color-focus)] disabled:opacity-50 ${
              errorMessage
                ? 'border-[var(--accent-red)] focus:border-[var(--accent-red)]'
                : 'border-[var(--border)] hover:border-[var(--border-strong)] focus:border-[var(--accent-blue)]'
            } ${leftIcon ? 'pl-8' : 'px-3'} ${
              rightIcon || (onClear && hasValue) ? 'pr-8' : 'pr-3'
            }`}
            {...props}
          />
          {onClear && hasValue ? (
            <button
              type="button"
              onClick={onClear}
              aria-label="Clear input"
              className="absolute right-2 rounded p-0.5 text-[var(--text-muted)] hover:bg-[var(--bg-bar-idle)] hover:text-[var(--text)] cursor-pointer"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          ) : (
            rightIcon && (
              <span className="pointer-events-none absolute right-2.5 text-[var(--text-muted)]">
                {rightIcon}
              </span>
            )
          )}
        </div>
        {errorMessage ? (
          <p className="text-[12px] text-[var(--accent-red)]">{errorMessage}</p>
        ) : (
          helperText && <p className="text-[12px] text-[var(--text-faint)]">{helperText}</p>
        )}
      </div>
    );
  },
);

Input.displayName = 'Input';
