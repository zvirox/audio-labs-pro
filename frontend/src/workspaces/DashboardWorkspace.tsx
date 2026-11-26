import { useMemo } from 'react';
import {
  Activity,
  ArrowRight,
  Command as CommandIcon,
  Cpu,
  Gauge,
  HelpCircle,
  Layers,
  ShieldCheck,
  Sliders,
} from 'lucide-react';
import { WORKSPACE_LIST } from '../layout/PrimaryRail';
import {
  StatCard,
  LinearCard,
  LinearButton,
  ProgressRow,
  DonutLegend,
  DataTable,
  StatusPill,
} from '../components/linear';

interface DashboardWorkspaceProps {
  theme: 'dark' | 'light';
  onOpenWorkspace: (id: string) => void;
  onOpenPalette: () => void;
  onOpenShortcuts: () => void;
}

const ENGINE_PIPELINES = [
  {
    name: 'Real-Time Mixer Graph',
    detail: '48.0 kHz / 32-bit float lock-free bus',
    status: 'Online' as const,
  },
  {
    name: 'Neural Inference Subprocess',
    detail: 'Matcha-TTS & RNNoise offline daemon ready',
    status: 'Ready' as const,
  },
  {
    name: 'MCP Tool Registry & Socket',
    detail: '68 typed tools behind Observe/Edit/Export tiers',
    status: 'Active' as const,
  },
  {
    name: 'Plugin Host (CLAP / LV2)',
    detail: 'System audio effect discovery and preset host',
    status: 'Ready' as const,
  },
];

export function DashboardWorkspace({
  onOpenWorkspace,
  onOpenPalette,
  onOpenShortcuts,
}: DashboardWorkspaceProps) {
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
          <h1 className="text-[28px] font-semibold leading-tight tracking-tight text-fg">
            Studio Overview
          </h1>
          <p className="mt-1 text-[13px] text-muted">
            Local neural synthesis, non-linear multitrack editing, and real-time DSP workstation.
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
            <kbd className="rounded border border-border bg-surface px-1 font-mono text-[10px] text-muted">
              Ctrl+K
            </kbd>
          </LinearButton>
          <LinearButton variant="ghost" size="sm" onClick={onOpenShortcuts}>
            <HelpCircle className="h-3.5 w-3.5" />
            <span>Shortcuts</span>
          </LinearButton>
        </div>
      </header>

      {/* Primary KPI Stat Row (no charts/sparklines) */}
      <section className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard
          label="Active Studios"
          value={WORKSPACE_LIST.length}
          subtitle="Unified single-window shell"
          delta={{ value: 'Magenta v2', trend: 'neutral' }}
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

      {/* Audio Engine & Pipeline Status (chart-free per ALP plan) */}
      <section className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <LinearCard level={1} className="flex flex-col gap-3 p-4 lg:col-span-2">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <Activity className="h-4 w-4 text-primary-text" />
              <div>
                <h2 className="text-[14px] font-semibold text-fg">
                  Audio Engine &amp; Subsystems
                </h2>
                <p className="text-[12px] text-muted">
                  Real-time DSP graph, local inference, and MCP readiness
                </p>
              </div>
            </div>
            <StatusPill variant="success" label="All Systems Online" />
          </div>

          <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
            {ENGINE_PIPELINES.map((item) => (
              <div
                key={item.name}
                className="flex flex-col justify-between gap-2 rounded-lg border border-border bg-surface-2 p-3"
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="inline-flex items-center gap-1.5 text-[13px] font-medium text-fg">
                    <Sliders className="h-3.5 w-3.5 text-primary-text" />
                    {item.name}
                  </span>
                  <StatusPill variant="success" label={item.status} />
                </div>
                <p className="text-[12px] text-muted">{item.detail}</p>
              </div>
            ))}
          </div>
        </LinearCard>

        <LinearCard level={1} className="flex flex-col justify-between gap-4 p-4">
          <div className="space-y-3">
            <h2 className="text-[14px] font-semibold text-fg">
              Subsystem Allocation
            </h2>
            <ProgressRow
              label="Neural Voice Engine"
              value={62}
              displayValue="62%"
              color="var(--primary)"
            />
            <ProgressRow
              label="Spectral De-Noiser"
              value={38}
              displayValue="38%"
              color="var(--sky)"
            />
            <ProgressRow
              label="Multitrack Cache"
              value={45}
              displayValue="45%"
              color="var(--amber)"
            />
            <ProgressRow
              label="Plugin Sandbox"
              value={24}
              displayValue="24%"
              color="var(--purple)"
            />
          </div>

          <div className="border-t border-border pt-3">
            <DonutLegend
              items={[
                { label: 'Core Engine (Rust)', value: '12 crates', percent: 55, color: 'var(--primary)' },
                { label: 'Tauri Web UI', value: 'React 19', percent: 30, color: 'var(--sky)' },
                { label: 'Model Weights', value: 'Offline', percent: 15, color: 'var(--amber)' },
              ]}
            />
          </div>
        </LinearCard>
      </section>

      {/* Studio Workspaces Table */}
      <section className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <h2 className="text-[14px] font-semibold text-fg">Workspaces</h2>
          <span className="text-[12px] text-muted">
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
                <span className="font-semibold text-fg">{r.name}</span>
              ),
            },
            {
              key: 'desc',
              header: 'Capabilities',
              render: (r) => (
                <span className="text-muted">{r.desc}</span>
              ),
            },
            {
              key: 'shortcut',
              header: 'Shortcut',
              render: (r) => (
                <kbd className="rounded border border-border bg-surface-2 px-1.5 py-0.5 font-mono text-[11px] text-muted">
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
                <ArrowRight className="ml-auto h-3.5 w-3.5 text-muted" />
              ),
            },
          ]}
        />
      </section>
    </div>
  );
}
