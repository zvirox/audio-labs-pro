import { forwardRef, useId } from 'react';

export interface SliderProps {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  label?: string;
  unit?: string;
  formatValue?: (val: number) => string;
  disabled?: boolean;
  className?: string;
  showTicks?: boolean;
}

export const Slider = forwardRef<HTMLInputElement, SliderProps>(
  (
    {
      value,
      min,
      max,
      step = 1,
      onChange,
      label,
      unit = '',
      formatValue,
      disabled = false,
      className = '',
      showTicks = false,
    },
    ref
  ) => {
    const inputId = useId();
    const percentage = Math.max(0, Math.min(100, ((value - min) / (max - min)) * 100));

    const displayValue = formatValue
      ? formatValue(value)
      : `${value.toLocaleString(undefined, { maximumFractionDigits: 2 })}${unit ? ` ${unit}` : ''}`;

    return (
      <div className={`space-y-1.5 select-none ${disabled ? 'opacity-40 pointer-events-none' : ''} ${className}`}>
        {(label || displayValue) && (
          <div className="flex items-center justify-between text-xs">
            {label && (
              <label htmlFor={inputId} className="font-medium text-[var(--color-fg-muted)]">
                {label}
              </label>
            )}
            <span className="font-mono text-[11px] text-[var(--color-accent)] font-semibold">
              {displayValue}
            </span>
          </div>
        )}

        <div className="relative flex items-center h-5 group">
          {/* Track background */}
          <div className="w-full h-1.5 bg-[var(--color-surface-3)] rounded-full overflow-hidden border border-[var(--color-border)]">
            {/* Fill track */}
            <div
              className="h-full bg-[var(--color-accent)] transition-all duration-75"
              style={{ width: `${percentage}%` }}
            />
          </div>

          {/* Native range input with invisible slider styling overlay */}
          <input
            ref={ref}
            id={inputId}
            type="range"
            min={min}
            max={max}
            step={step}
            value={value}
            disabled={disabled}
            onChange={(e) => onChange(parseFloat(e.target.value))}
            className="absolute inset-0 w-full h-full opacity-0 cursor-ew-resize focus:outline-none"
            aria-valuemin={min}
            aria-valuemax={max}
            aria-valuenow={value}
            aria-valuetext={displayValue}
          />

          {/* Custom draggable thumb indicator */}
          <div
            className="pointer-events-none absolute w-3.5 h-3.5 rounded-full bg-white shadow-md border border-[var(--color-border)] transition-transform duration-75 transform -translate-x-1/2 group-hover:scale-125"
            style={{ left: `${percentage}%` }}
          />
        </div>

        {showTicks && (
          <div className="flex justify-between text-[9px] font-mono text-[var(--color-fg-subtle)] px-0.5">
            <span>{min}</span>
            <span>{((max + min) / 2).toFixed(0)}</span>
            <span>{max}</span>
          </div>
        )}
      </div>
    );
  }
);

Slider.displayName = 'Slider';
