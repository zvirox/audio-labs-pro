import React from 'react';

export interface LinearButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
}

export const LinearButton: React.FC<LinearButtonProps> = ({
  variant = 'secondary',
  size = 'md',
  className = '',
  children,
  ...props
}) => {
  const sizeStyles = {
    sm: 'h-7 px-2.5 text-xs',
    md: 'h-8 px-3 text-xs',
    lg: 'h-10 px-4 text-sm',
  }[size];

  const variantStyles = {
    primary:
      'bg-[var(--accent-blue)] hover:bg-[var(--accent-blue-hover)] active:bg-[var(--accent-blue-active)] text-[var(--text-on-accent)] font-medium border border-[var(--accent-blue)]',
    secondary:
      'bg-[var(--bg-surface-2)] hover:bg-[var(--bg-bar-idle)] text-[var(--text)] border border-[var(--border)] hover:border-[var(--border-strong)] shadow-[var(--shadow-card)]',
    ghost:
      'bg-transparent hover:bg-[var(--bg-surface-2)] text-[var(--text-muted)] hover:text-[var(--text)] border border-transparent',
    danger:
      'bg-[var(--tint-red)] hover:bg-[var(--bg-surface-2)] text-[var(--accent-red)] border border-[var(--accent-red)]',
  }[variant];

  return (
    <button
      className={`inline-flex items-center justify-center gap-1.5 rounded-md font-sans transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus)] disabled:opacity-50 disabled:pointer-events-none cursor-pointer ${sizeStyles} ${variantStyles} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
