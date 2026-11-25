import { useEffect, useMemo, useRef, useState } from 'react';
import * as echarts from 'echarts';
import {
  Activity,
  ArrowRight,
  Command as CommandIcon,
  Cpu,
  Gauge,
  HelpCircle,
  Layers,
  ShieldCheck,
} from 'lucide-react';
import { WORKSPACE_LIST } from '../layout/PrimaryRail';
import {
  StatCard,
  LinearCard,
  LinearButton,
  RangeTabs,
  ProgressRow,
  DonutLegend,
  DataTable,
  StatusPill,
  buildLinearEChartsOption,
  getLinearChartPalette,
} from '../components/linear';

interface DashboardWorkspaceProps {
  theme: 'dark' | 'light';
  onOpenWorkspace: (id: string) => void;
  onOpenPalette: () => void;
  onOpenShortcuts: () => void;
}

type RangeId = '15m' | '1h' | '24h';

const SERIES_BY_RANGE: Record<RangeId, { labels: string[]; dsp: number[]; io: number[] }> = {
  '15m': {
    labels: ['-15m', '-12m', '-9m', '-6m', '-3m', 'Now'],
    dsp: [14, 18, 16, 22, 19, 17],
    io: [9, 11, 10, 14, 12, 11],
  },
  '1h': {
    labels: ['-60m', '-48m', '-36m', '-24m', '-12m', 'Now'],
    dsp: [21, 17, 25, 29, 20, 17],
    io: [12, 11, 15, 16, 12, 11],
  },
  '24h': {
    labels: ['00:00', '04:00', '08:00', '12:00', '16:00', 'Now'],
    dsp: [11, 14, 28, 34, 24, 17],
    io: [8, 9, 16, 19, 14, 11],
  },
};

export function DashboardWorkspace({
  theme,
  onOpenWorkspace,
  onOpenPalette,
  onOpenShortcuts,
}: DashboardWorkspaceProps) {
  const [range, setRange] = useState<RangeId>('1h');
  const chartContainerRef = useRef<HTMLDivElement | null>(null);
  const chartInstanceRef = useRef<echarts.ECharts | null>(null);

  const seriesData = SERIES_BY_RANGE[range];

  useEffect(() => {
    const el = chartContainerRef.current;
    if (!el) return;

    const chart = chartInstanceRef.current ?? echarts.init(el);
    chartInstanceRef.current = chart;

    const p = getLinearChartPalette(theme);
    const option = buildLinearEChartsOption(theme, {
      legend: {
        top: 0,
        right: 12,
        textStyle: { color: p.label, fontSize: 12 },
        data: ['DSP Graph Load (%)', 'Audio I/O Buffer (%)'],
      },
      grid: { left: 40, right: 20, top: 34, bottom: 26 },
      xAxis: {
        type: 'category',
        data: seriesData.labels,
        axisLine: { lineStyle: { color: p.axisLine } },
        axisLabel: { color: p.label, fontSize: 11 },
      },
      yAxis: {
        type: 'value',
        max: 45,
        splitLine: { lineStyle: { color: p.grid, type: 'dashed' } },
        axisLabel: { color: p.label, fontSize: 11, formatter: '{value}%' },
      },
      series: [
        {
          name: 'DSP Graph Load (%)',
          type: 'line',
          smooth: false,
          symbol: 'circle',
          symbolSize: 5,
          lineStyle: { width: 2, color: p.series[0] },
          itemStyle: { color: p.series[0] },
          data: seriesData.dsp,
        },
        {
          name: 'Audio I/O Buffer (%)',
          type: 'line',
          smooth: false,
          symbol: 'circle',
          symbolSize: 5,
          lineStyle: { width: 2, color: p.series[1] },
          itemStyle: { color: p.series[1] },
          data: seriesData.io,
        },
      ],
    });

    chart.setOption(option, true);

    const observer = new ResizeObserver(() => {
      chart.resize();
    });
    observer.observe(el);

    return () => {
      observer.disconnect();
    };
  }, [theme, seriesData]);

  useEffect(() => {
    return () => {
      chartInstanceRef.current?.dispose();
      chartInstanceRef.current = null;
    };
  }, []);

  const studioRows = useMemo(
    () =>
      WORKSPACE_LIST.map((ws) => ({
        id: ws.id,
        name: ws.name,
        desc: ws.desc,
        shortcut: ws.shortcut,
        status: ws.id === 'settings' ? ('Config' as const) : ('Ready' as const),
      })),
    [],
  );

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 pb-8">
      {/* Header Row */}
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-[28px] font-semibold leading-tight tracking-tight text-[var(--text)]">
            Studio Overview
          </h1>
          <p className="mt-1 text-[13px] text-[var(--text-muted)]">
            Local neural synthesis, non-linear multitrack editing, and real-time DSP telemetry.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <LinearButton
            variant="secondary"
            size="sm"
            onClick={onOpenPalette}
            className="gap-2"
          >
            <CommandIcon className="h-3.5 w-3.5" />
            <span>Command Palette</span>
            <kbd className="rounded border border-[var(--border)] bg-[var(--bg-surface)] px-1 font-mono text-[10px] text-[var(--text-muted)]">
              Ctrl+K
            </kbd>
          </LinearButton>
          <LinearButton variant="ghost" size="sm" onClick={onOpenShortcuts}>
            <HelpCircle className="h-3.5 w-3.5" />
            <span>Shortcuts</span>
          </LinearButton>
        </div>
      </header>

      {/* Primary KPI Stat Row */}
      <section className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard
          label="Active Studios"
          value={WORKSPACE_LIST.length}
          subtitle="Unified single-window shell"
          delta={{ value: 'Linear v1', trend: 'neutral' }}
          icon={<Layers className="h-4 w-4" />}
        />
        <StatCard
          label="Engine Target"
          value="48.0 kHz"
          subtitle="32-bit float internal bus"
          delta={{ value: '2.6 ms', trend: 'up' }}
          icon={<Gauge className="h-4 w-4" />}
        />
        <StatCard
          label="MCP Capabilities"
          value={68}
          subtitle="Authenticated local tools"
          delta={{ value: '3 Tiers', trend: 'neutral' }}
          icon={<ShieldCheck className="h-4 w-4" />}
        />
        <StatCard
          label="Inference Ready"
          value="ONNX"
          subtitle="Matcha-TTS & RNNoise offline"
          delta={{ value: 'Local', trend: 'up' }}
          icon={<Cpu className="h-4 w-4" />}
        />
      </section>

      {/* Telemetry Chart + Subsystem Load */}
      <section className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <LinearCard level={1} className="flex flex-col gap-3 p-4 lg:col-span-2">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <Activity className="h-4 w-4 text-[var(--accent-blue)]" />
              <div>
                <h2 className="text-[14px] font-semibold text-[var(--text)]">
                  DSP & Buffer Telemetry
                </h2>
                <p className="text-[12px] text-[var(--text-muted)]">
                  Real-time graph utilization and audio buffer headroom
                </p>
              </div>
            </div>
            <RangeTabs
              value={range}
              onChange={(id) => setRange(id as RangeId)}
              options={[
                { id: '15m', label: '15m' },
                { id: '1h', label: '1h' },
                { id: '24h', label: '24h' },
              ]}
            />
          </div>
          <div ref={chartContainerRef} className="h-56 w-full" />
        </LinearCard>

        <LinearCard level={1} className="flex flex-col justify-between gap-4 p-4">
          <div className="space-y-3">
            <h2 className="text-[14px] font-semibold text-[var(--text)]">
              Subsystem Allocation
            </h2>
            <ProgressRow
              label="Neural Voice Engine"
              value={62}
              displayValue="62%"
              color="var(--accent-blue)"
            />
            <ProgressRow
              label="Spectral De-Noiser"
              value={38}
              displayValue="38%"
              color="var(--accent-sky)"
            />
            <ProgressRow
              label="Multitrack Cache"
              value={45}
              displayValue="45%"
              color="var(--accent-teal)"
            />
            <ProgressRow
              label="Plugin Sandbox"
              value={24}
              displayValue="24%"
              color="var(--accent-purple)"
            />
          </div>

          <div className="border-t border-[var(--border)] pt-3">
            <DonutLegend
              items={[
                { label: 'Core Engine (Rust)', value: '12 crates', percent: 55, color: 'var(--accent-blue)' },
                { label: 'Tauri Web UI', value: 'React 19', percent: 30, color: 'var(--accent-sky)' },
                { label: 'Model Weights', value: 'Offline', percent: 15, color: 'var(--accent-teal)' },
              ]}
            />
          </div>
        </LinearCard>
      </section>

      {/* Studio Workspaces Table */}
      <section className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <h2 className="text-[14px] font-semibold text-[var(--text)]">Workspaces</h2>
          <span className="text-[12px] text-[var(--text-muted)]">
            Click any studio row to launch immediately
          </span>
        </div>

        <DataTable
          rows={studioRows}
          rowKey={(r) => r.id}
          onRowClick={(r) => onOpenWorkspace(r.id)}
          columns={[
            {
              key: 'name',
              header: 'Workspace',
              render: (r) => (
                <span className="font-semibold text-[var(--text)]">{r.name}</span>
              ),
            },
            {
              key: 'desc',
              header: 'Capabilities',
              render: (r) => (
                <span className="text-[var(--text-muted)]">{r.desc}</span>
              ),
            },
            {
              key: 'shortcut',
              header: 'Shortcut',
              render: (r) => (
                <kbd className="rounded border border-[var(--border)] bg-[var(--bg-surface-2)] px-1.5 py-0.5 font-mono text-[11px] text-[var(--text-muted)]">
                  {r.shortcut}
                </kbd>
              ),
            },
            {
              key: 'status',
              header: 'Status',
              render: (r) => (
                <StatusPill
                  variant={r.status === 'Ready' ? 'success' : 'info'}
                  label={r.status}
                />
              ),
            },
            {
              key: 'action',
              header: '',
              align: 'right',
              render: () => (
                <ArrowRight className="ml-auto h-3.5 w-3.5 text-[var(--text-muted)]" />
              ),
            },
          ]}
        />
      </section>
    </div>
  );
}
