import React, { forwardRef, useId } from 'react';
import { XCircle } from 'lucide-react';

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
      disabled,
      ...props
    },
    ref
  ) => {
    const generatedId = useId();
    const inputId = id || generatedId;
    const hasError = Boolean(errorMessage);

    return (
      <div className={`space-y-1 text-left ${className}`}>
        {label && (
          <label
            htmlFor={inputId}
            className="block text-xs font-medium text-[var(--color-fg-muted)]"
          >
            {label}
          </label>
        )}

        <div className="relative flex items-center">
          {leftIcon && (
            <div className="absolute left-2.5 text-[var(--color-fg-subtle)] pointer-events-none flex items-center">
              {leftIcon}
            </div>
          )}

          <input
            ref={ref}
            id={inputId}
            value={value}
            disabled={disabled}
            className={`w-full h-8 rounded-lg bg-[var(--color-surface-2)] text-[var(--color-fg)] text-xs border transition-colors placeholder:text-[var(--color-fg-subtle)] focus:outline-none focus:ring-1 focus:ring-[var(--color-focus)] disabled:opacity-40 disabled:cursor-not-allowed ${
              leftIcon ? 'pl-8' : 'pl-3'
            } ${rightIcon || onClear ? 'pr-8' : 'pr-3'} ${
              hasError
                ? 'border-red-500/60 focus:border-red-500 focus:ring-red-500/30'
                : 'border-[var(--color-border)] focus:border-[var(--color-focus)]'
            }`}
            aria-invalid={hasError}
            aria-describedby={
              hasError
                ? `${inputId}-error`
                : helperText
                ? `${inputId}-helper`
                : undefined
            }
            {...props}
          />

          {onClear && value && !disabled && (
            <button
              type="button"
              onClick={onClear}
              className="absolute right-2 text-[var(--color-fg-subtle)] hover:text-[var(--color-fg)] p-0.5"
              title="Clear input"
            >
              <XCircle className="w-3.5 h-3.5" />
            </button>
          )}

          {rightIcon && !onClear && (
            <div className="absolute right-2.5 text-[var(--color-fg-subtle)] pointer-events-none flex items-center">
              {rightIcon}
            </div>
          )}
        </div>

        {hasError ? (
          <p id={`${inputId}-error`} className="text-[11px] text-red-400 font-medium">
            {errorMessage}
          </p>
        ) : helperText ? (
          <p id={`${inputId}-helper`} className="text-[11px] text-[var(--color-fg-subtle)]">
            {helperText}
          </p>
        ) : null}
      </div>
    );
  }
);

Input.displayName = 'Input';
