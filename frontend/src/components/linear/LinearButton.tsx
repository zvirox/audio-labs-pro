import React, { forwardRef } from 'react';

export interface LinearButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'icon';
}

export const LinearButton = forwardRef<HTMLButtonElement, LinearButtonProps>(({
  variant = 'secondary',
  size = 'md',
  className = '',
  children,
  ...props
}, ref) => {
  const sizeStyles = {
    sm: 'h-7 px-2.5 text-[12px]',
    md: 'h-8 px-3 text-[13px]',
    icon: 'h-8 w-8 p-0',
  }[size];

  const variantStyles = {
    primary:
      'bg-primary hover:bg-primary-hover text-primary-fg font-medium border border-primary',
    secondary:
      'bg-surface-2 hover:bg-hover text-fg border border-border hover:border-border-strong shadow-card',
    ghost:
      'bg-transparent hover:bg-hover text-fg-2 hover:text-fg border border-transparent',
    danger:
      'bg-chip-red hover:bg-surface-2 text-red-text border border-red',
  }[variant];

  return (
    <button
      ref={ref}
      className={`inline-flex items-center justify-center gap-1.5 rounded-md font-sans transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-text disabled:opacity-50 disabled:pointer-events-none cursor-pointer ${sizeStyles} ${variantStyles} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
});

LinearButton.displayName = 'LinearButton';
