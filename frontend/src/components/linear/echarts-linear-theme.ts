export type ChartPalette = {
  bg: string;
  grid: string;
  axisLine: string;
  label: string;
  tooltipBg: string;
  tooltipBorder: string;
  tooltipText: string;
  series: [string, string, string, string, string];
};

export const DARK_CHART_PALETTE: ChartPalette = {
  bg: 'transparent',
  grid: '#1F2125',
  axisLine: '#2C2E33',
  label: '#8A8B91',
  tooltipBg: '#111213',
  tooltipBorder: '#2C2E33',
  tooltipText: '#F5F5F6',
  series: ['#3355FF', '#38B6F0', '#1FB5A5', '#7C5CE6', '#F97316'],
};

export const LIGHT_CHART_PALETTE: ChartPalette = {
  bg: 'transparent',
  grid: '#E5E7EB',
  axisLine: '#D1D5DB',
  label: '#6B7280',
  tooltipBg: '#FFFFFF',
  tooltipBorder: '#D1D5DB',
  tooltipText: '#0F1115',
  series: ['#3355FF', '#38B6F0', '#1FB5A5', '#7C5CE6', '#F97316'],
};

export function getLinearChartPalette(theme: 'dark' | 'light'): ChartPalette {
  return theme === 'light' ? LIGHT_CHART_PALETTE : DARK_CHART_PALETTE;
}

export function buildLinearEChartsOption(
  theme: 'dark' | 'light',
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  const p = getLinearChartPalette(theme);
  return {
    backgroundColor: p.bg,
    color: p.series,
    textStyle: {
      fontFamily: 'Inter, -apple-system, BlinkMacSystemFont, sans-serif',
      color: p.label,
      fontSize: 12,
    },
    grid: {
      left: 40,
      right: 16,
      top: 18,
      bottom: 28,
      containLabel: false,
    },
    tooltip: {
      trigger: 'axis',
      backgroundColor: p.tooltipBg,
      borderColor: p.tooltipBorder,
      borderWidth: 1,
      textStyle: {
        color: p.tooltipText,
        fontSize: 12,
      },
    },
    ...overrides,
  };
}
