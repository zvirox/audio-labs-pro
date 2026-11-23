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
      size = 'sm',
      isLoading = false,
      shortcut,
      leftIcon,
      rightIcon,
      className = '',
      disabled,
      ...props
    },
    ref
  ) => {
    const baseStyles =
      'relative inline-flex items-center justify-center font-medium rounded-lg transition-all select-none focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus)] disabled:opacity-40 disabled:pointer-events-none active:scale-[0.98] cursor-pointer';

    const sizeStyles = {
      xs: 'text-[11px] h-6 px-2 gap-1.5',
      sm: 'text-xs h-8 px-3 gap-2',
      md: 'text-sm h-9 px-4 gap-2.5',
      lg: 'text-base h-11 px-5 gap-3',
    };

    const variantStyles = {
      primary:
        'bg-[var(--color-accent)] text-[var(--color-accent-fg)] hover:opacity-90 shadow-sm shadow-[var(--color-accent)]/20 font-semibold',
      secondary:
        'bg-[var(--color-surface-2)] text-[var(--color-fg)] hover:bg-[var(--color-surface-3)] border border-[var(--color-border)] shadow-xs',
      ghost:
        'bg-transparent text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] hover:bg-[var(--color-surface-2)]',
      outline:
        'bg-transparent border border-[var(--color-border-strong)] text-[var(--color-fg)] hover:bg-[var(--color-surface-2)]',
      danger:
        'bg-red-500/20 text-red-300 border border-red-500/40 hover:bg-red-500/30',
      icon:
        'p-0 aspect-square text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] hover:bg-[var(--color-surface-2)] rounded-md border border-transparent hover:border-[var(--color-border)]',
    };

    return (
      <button
        ref={ref}
        disabled={disabled || isLoading}
        className={`${baseStyles} ${sizeStyles[size]} ${variantStyles[variant]} ${className}`}
        {...props}
      >
        {isLoading ? (
          <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" />
        ) : (
          leftIcon
        )}
        {children && <span>{children}</span>}
        {!isLoading && rightIcon}
        {shortcut && (
          <kbd
            className="ml-auto pl-2 text-[10px] font-mono opacity-60 tracking-wider hidden sm:inline-block"
            aria-hidden="true"
          >
            {shortcut}
          </kbd>
        )}
      </button>
    );
  }
);

Button.displayName = 'Button';
