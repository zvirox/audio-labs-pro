import React from 'react';
import { LinearDialog } from '../components/linear/LinearDialog';

export interface DialogProps {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
  size?: 'sm' | 'md' | 'lg' | 'xl';
}

const SIZE_MAP: Record<NonNullable<DialogProps['size']>, string> = {
  sm: 'max-w-sm',
  md: 'max-w-md',
  lg: 'max-w-lg',
  xl: 'max-w-2xl',
};

export function Dialog({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  size = 'md',
}: DialogProps) {
  return (
    <LinearDialog
      open={open}
      onClose={onClose}
      title={title}
      description={description}
      footer={footer}
      maxWidthClass={SIZE_MAP[size]}
    >
      {children}
    </LinearDialog>
  );
}
