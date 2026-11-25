import React, { forwardRef } from 'react';
import { Loader2 } from 'lucide-react';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'outline' | 'danger' | 'icon';
  size?: 'xs' | 'sm' | 'md' | 'lg';
  isLoading?: boolean;
  shortcut?: string;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      children,
      variant = 'secondary',
      size = 'md',
      isLoading = false,
      shortcut,
      leftIcon,
      rightIcon,
      className = '',
      disabled,
      type = 'button',
      ...props
    },
    ref,
  ) => {
    const baseClasses =
      'inline-flex items-center justify-center font-medium font-sans select-none transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus)] disabled:opacity-50 disabled:pointer-events-none cursor-pointer';

    const sizeClasses = {
      xs: 'h-6 px-2 text-[11px] gap-1 rounded',
      sm: 'h-7 px-2.5 text-[12px] gap-1.5 rounded-md',
      md: 'h-8 px-3 text-[13px] gap-2 rounded-md',
      lg: 'h-10 px-4 text-[14px] gap-2 rounded-md',
    }[size];

    const iconOnlySize =
      variant === 'icon'
        ? { xs: 'w-6 px-0', sm: 'w-7 px-0', md: 'w-8 px-0', lg: 'w-10 px-0' }[size]
        : '';

    const variantClasses = {
      primary:
        'bg-[var(--accent-blue)] text-[var(--text-on-accent)] border border-[var(--accent-blue)] hover:bg-[var(--accent-blue-hover)] active:bg-[var(--accent-blue-active)]',
      secondary:
        'bg-[var(--bg-surface-2)] text-[var(--text)] border border-[var(--border)] shadow-[var(--shadow-card)] hover:bg-[var(--bg-bar-idle)] hover:border-[var(--border-strong)]',
      outline:
        'bg-transparent text-[var(--text)] border border-[var(--border-strong)] hover:bg-[var(--bg-surface-2)]',
      ghost:
        'bg-transparent text-[var(--text-muted)] border border-transparent hover:bg-[var(--bg-surface-2)] hover:text-[var(--text)]',
      danger:
        'bg-[var(--tint-red)] text-[var(--accent-red)] border border-[var(--accent-red)] hover:bg-[var(--bg-surface-2)]',
      icon:
        'bg-transparent text-[var(--text-muted)] border border-transparent hover:bg-[var(--bg-surface-2)] hover:text-[var(--text)]',
    }[variant];

    return (
      <button
        ref={ref}
        type={type}
        disabled={disabled || isLoading}
        className={`${baseClasses} ${sizeClasses} ${iconOnlySize} ${variantClasses} ${className}`}
        {...props}
      >
        {isLoading ? (
          <Loader2 className="h-3.5 w-3.5 animate-spin shrink-0" />
        ) : (
          leftIcon && <span className="shrink-0">{leftIcon}</span>
        )}
        {children}
        {!isLoading && rightIcon && <span className="shrink-0">{rightIcon}</span>}
        {shortcut && !isLoading && (
          <kbd className="ml-1 hidden rounded border border-[var(--border)] bg-[var(--bg-surface)] px-1 py-0.5 font-mono text-[10px] leading-none text-[var(--text-muted)] sm:inline-block">
            {shortcut}
          </kbd>
        )}
      </button>
    );
  },
);

Button.displayName = 'Button';
