import { forwardRef, useId } from 'react';

export interface SliderProps {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  label?: string;
  /** Accessible name when no visible label is rendered (e.g. transport scrubbers). */
  ariaLabel?: string;
  unit?: string;
  formatValue?: (val: number) => string;
  disabled?: boolean;
  className?: string;
}

export const Slider = forwardRef<HTMLInputElement, SliderProps>(
  (
    {
      value,
      min,
      max,
      step = 0.01,
      onChange,
      label,
      ariaLabel,
      unit = '',
      formatValue,
      disabled = false,
      className = '',
    },
    ref,
  ) => {
    const id = useId();
    const span = max - min;
    const pct = span > 0 ? Math.max(0, Math.min(100, ((value - min) / span) * 100)) : 0;
    const displayVal = formatValue ? formatValue(value) : `${value}${unit}`;

    return (
      <div className={`flex flex-col gap-1.5 ${className}`}>
        {(label || ariaLabel) && label && (
          <div className="flex items-center justify-between text-[12px]">
            <label htmlFor={id} className="font-medium text-[var(--text-muted)]">
              {label}
            </label>
            <span className="font-sans text-[12px] font-medium text-[var(--text)] tabular-nums">
              {displayVal}
            </span>
          </div>
        )}
        <div className="relative flex h-5 items-center">
          <div className="h-1.5 w-full overflow-hidden rounded bg-[var(--bg-bar-idle)]">
            <div
              className="h-full bg-[var(--accent-blue)]"
              style={{ width: `${pct}%` }}
            />
          </div>
          <div
            aria-hidden="true"
            className="pointer-events-none absolute top-1/2 h-3.5 w-3.5 -translate-x-1/2 -translate-y-1/2 rounded border border-[var(--border-strong)] bg-[var(--bg-surface)]"
            style={{ left: `${pct}%` }}
          />
          <input
            ref={ref}
            id={id}
            type="range"
            min={min}
            max={max}
            step={step}
            value={value}
            disabled={disabled}
            aria-label={ariaLabel ?? label}
            onChange={(e) => onChange(parseFloat(e.target.value))}
            className="absolute inset-0 h-full w-full cursor-pointer opacity-0 disabled:cursor-not-allowed"
          />
        </div>
      </div>
    );
  },
);

Slider.displayName = 'Slider';
