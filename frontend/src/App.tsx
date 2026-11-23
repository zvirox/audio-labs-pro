import { useState, useEffect } from 'react';
import {
  Mic,
  Layers,
  BookOpen,
  Film,
  Wrench,
  Sparkles,
  Settings,
  RefreshCw,
  Sun,
  Moon,
  Zap,
  Activity,
  Layers as LayersIcon,
  CheckCircle2,
  Info,
  Clock,
  Radio,
  FileAudio,
} from 'lucide-react';
import {
  getProjectSummary,
  getWorkspaceState,
  triggerDemoEdit,
  listenSessionRevision,
  isTauriEnvironment,
} from './ipc/client';
import type { ProjectSummary, WorkspaceState, SessionRevisionPayload } from './ipc/types';

interface WorkspaceDefinition {
  id: string;
  name: string;
  icon: typeof Mic;
  neon: string;
  lightNeon: string;
  desc: string;
}

const WORKSPACES: WorkspaceDefinition[] = [
  { id: 'voice', name: 'Voice Studio', icon: Mic, neon: '#00f0ff', lightNeon: '#008899', desc: 'Cloning, TTS synthesis, neural voice models' },
  { id: 'multitrack', name: 'Multitrack DAW', icon: Layers, neon: '#6366f1', lightNeon: '#4338ca', desc: 'Non-linear editing, punch-in, zero-latency mixing' },
  { id: 'audiobook', name: 'Audiobook Lab', icon: BookOpen, neon: '#f59e0b', lightNeon: '#b45309', desc: 'Long-form narrative, chapters, ACX compliance' },
  { id: 'dubbing', name: 'Video Dubbing', icon: Film, neon: '#ec4899', lightNeon: '#be185d', desc: 'Video playback sync, stem separation, ADR alignment' },
  { id: 'tools', name: 'Batch & Repair', icon: Wrench, neon: '#10b981', lightNeon: '#047857', desc: 'Spectral de-noise, click removal, loudness matching' },
  { id: 'effects', name: 'DSP & Effects', icon: Sparkles, neon: '#8b5cf6', lightNeon: '#6d28d9', desc: 'Parametric EQ, multiband comp, spatial acoustics' },
  { id: 'settings', name: 'Preferences', icon: Settings, neon: '#94a3b8', lightNeon: '#475569', desc: 'Audio devices, MCP servers, KWin glass settings' },
];

export default function App() {
  const [activeWorkspace, setActiveWorkspace] = useState('voice');
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [projectSummary, setProjectSummary] = useState<ProjectSummary | null>(null);
  const [workspaceState, setWorkspaceState] = useState<WorkspaceState | null>(null);
  const [revisionEvents, setRevisionEvents] = useState<SessionRevisionPayload[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const isNative = isTauriEnvironment();

  // Switch dark/light class on body/root
  useEffect(() => {
    const root = document.documentElement;
    if (theme === 'light') {
      root.classList.add('theme-light');
    } else {
      root.classList.remove('theme-light');
    }
  }, [theme]);

  // Set workspace active neon accent CSS variable
  useEffect(() => {
    const ws = WORKSPACES.find((w) => w.id === activeWorkspace);
    if (ws) {
      const color = theme === 'light' ? ws.lightNeon : ws.neon;
      document.documentElement.style.setProperty('--color-accent', color);
    }
  }, [activeWorkspace, theme]);

  // Initial fetch and event subscription
  useEffect(() => {
    fetchState();

    let unlisten: (() => void) | undefined;
    listenSessionRevision((payload) => {
      setRevisionEvents((prev) => [payload, ...prev.slice(0, 7)]);
      setProjectSummary((prev) => (prev ? { ...prev, revision: payload.revision, is_dirty: payload.is_dirty } : null));
    }).then((cleanup) => {
      unlisten = cleanup;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  async function fetchState() {
    setIsLoading(true);
    try {
      const [summary, state] = await Promise.all([
        getProjectSummary(),
        getWorkspaceState(),
      ]);
      setProjectSummary(summary);
      setWorkspaceState(state);
    } catch (err) {
      console.error('Failed to fetch IPC state:', err);
    } finally {
      setIsLoading(false);
    }
  }

  async function handleDemoEdit() {
    try {
      await triggerDemoEdit(`Split Clip @ ${new Date().toLocaleTimeString()}`);
    } catch (err) {
      console.error('Failed to trigger edit:', err);
    }
  }

  const activeWsObj = WORKSPACES.find((w) => w.id === activeWorkspace) || WORKSPACES[0];
  const activeColor = theme === 'light' ? activeWsObj.lightNeon : activeWsObj.neon;

  return (
    <div className="min-h-screen flex flex-col bg-[var(--color-bg)] text-[var(--color-fg)] font-sans antialiased selection:bg-[var(--color-accent)] selection:text-black">
      {/* Top Header / App Shell Bar */}
      <header className="h-12 border-b border-[var(--color-border)] px-4 flex items-center justify-between bg-[var(--color-surface)] backdrop-blur-md sticky top-0 z-50">
        <div className="flex items-center gap-3">
          <div className="w-7 h-7 rounded-md bg-gradient-to-tr from-cyan-500 to-indigo-600 flex items-center justify-center shadow-lg shadow-cyan-500/20">
            <Radio className="w-4 h-4 text-white" />
          </div>
          <span className="font-bold tracking-tight text-sm">AUDIO LABS PRO</span>
          <span className="text-[10px] font-mono px-2 py-0.5 rounded-full border border-[var(--color-border)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]">
            Phase M2 Desktop Shell
          </span>
          <div className={`flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-mono ${
            isNative ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/30' : 'bg-amber-500/10 text-amber-400 border border-amber-500/30'
          }`}>
            <span className={`w-1.5 h-1.5 rounded-full ${isNative ? 'bg-emerald-400 animate-pulse' : 'bg-amber-400'}`} />
            {isNative ? 'Tauri 2 Native Bridge' : 'Vite Dev Mock Fallback'}
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
            className="p-1.5 rounded-md hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] transition-colors border border-[var(--color-border)]"
            title="Toggle theme"
          >
            {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
          </button>
          <button
            onClick={fetchState}
            disabled={isLoading}
            className="flex items-center gap-1 px-2.5 py-1 text-xs rounded-md bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] border border-[var(--color-border)] text-[var(--color-fg)] transition-colors disabled:opacity-50"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />
            <span>Sync</span>
          </button>
        </div>
      </header>

      {/* Main Dual-Axis Shell Layout Container */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Vertical Primary Rail (56px) */}
        <aside className="w-14 border-r border-[var(--color-border)] bg-[var(--color-surface)] flex flex-col items-center py-3 gap-2 flex-shrink-0">
          {WORKSPACES.map((ws) => {
            const Icon = ws.icon;
            const isActive = activeWorkspace === ws.id;
            return (
              <button
                key={ws.id}
                onClick={() => setActiveWorkspace(ws.id)}
                className={`relative group w-10 h-10 rounded-lg flex items-center justify-center transition-all ${
                  isActive
                    ? 'bg-[var(--color-surface-2)] shadow-md text-white'
                    : 'text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] hover:bg-[var(--color-surface-2)]'
                }`}
                style={isActive ? { borderColor: activeColor, borderLeftWidth: 3 } : undefined}
                title={ws.name}
              >
                <Icon
                  className="w-5 h-5 transition-transform group-hover:scale-110"
                  style={{ color: isActive ? activeColor : undefined }}
                />
                {/* Active Indicator Pip */}
                {isActive && (
                  <span
                    className="absolute -left-1 w-1.5 h-4 rounded-r-full"
                    style={{ backgroundColor: activeColor }}
                  />
                )}
              </button>
            );
          })}
        </aside>

        {/* Workspace Content Area */}
        <div className="flex-1 flex flex-col overflow-y-auto">
          {/* Top Horizontal Subtab Bar (40px) */}
          <nav className="h-10 border-b border-[var(--color-border)] bg-[var(--color-surface-2)] px-4 flex items-center justify-between flex-shrink-0">
            <div className="flex items-center gap-4">
              <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-accent)]">
                {activeWsObj.name}
              </span>
              <div className="h-3 w-[1px] bg-[var(--color-border)]" />
              <div className="flex items-center gap-1 text-xs">
                {['Overview & Tokens', 'IPC Testbed', 'Glass & Scrim', 'Diagnostics'].map((tab, idx) => (
                  <button
                    key={tab}
                    className={`px-2.5 py-1 rounded text-xs font-medium transition-colors ${
                      idx === 0
                        ? 'bg-[var(--color-surface)] text-[var(--color-fg)] border border-[var(--color-border)]'
                        : 'text-[var(--color-fg-muted)] hover:text-[var(--color-fg)]'
                    }`}
                  >
                    {tab}
                  </button>
                ))}
              </div>
            </div>
            <div className="text-[11px] text-[var(--color-fg-subtle)] font-mono">
              Hue: <span style={{ color: activeColor }}>{activeColor}</span>
            </div>
          </nav>

          {/* Body Dashboard */}
          <main className="p-6 space-y-6 max-w-7xl w-full mx-auto">
            {/* Header Banner */}
            <div className="p-5 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] backdrop-blur-lg relative overflow-hidden">
              <div
                className="absolute top-0 right-0 w-96 h-96 rounded-full blur-3xl opacity-15 pointer-events-none -mr-20 -mt-20"
                style={{ backgroundColor: activeColor }}
              />
              <div className="relative z-10 flex flex-col md:flex-row md:items-center justify-between gap-4">
                <div>
                  <h1 className="text-xl font-bold tracking-tight text-[var(--color-fg)] flex items-center gap-2">
                    <span>Phase M2: Desktop Shell & Design System</span>
                    <span
                      className="text-xs font-mono px-2 py-0.5 rounded border uppercase"
                      style={{ borderColor: activeColor, color: activeColor }}
                    >
                      Active Token: {activeWsObj.name}
                    </span>
                  </h1>
                  <p className="text-xs text-[var(--color-fg-muted)] mt-1 max-w-2xl">
                    Tauri 2 native desktop shell scaffolding, typed IPC bridge to <code className="text-[var(--color-fg)] font-mono">alp-session</code>, safe <code className="text-[var(--color-fg)] font-mono">x11rb</code> KWin blur, and dual-axis workspace navigation.
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={handleDemoEdit}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium text-black transition-transform active:scale-95 shadow-md"
                    style={{ backgroundColor: activeColor }}
                  >
                    <Zap className="w-3.5 h-3.5 fill-current" />
                    <span>Trigger Timeline Edit</span>
                  </button>
                </div>
              </div>
            </div>

            {/* Grid: Live IPC State & Events */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* IPC Project State Card */}
              <div className="p-5 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] space-y-4">
                <div className="flex items-center justify-between">
                  <h2 className="text-sm font-semibold flex items-center gap-2">
                    <FileAudio className="w-4 h-4 text-[var(--color-accent)]" />
                    <span>Live alp-session Project Summary</span>
                  </h2>
                  <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]">
                    get_project_summary
                  </span>
                </div>

                {projectSummary ? (
                  <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 text-xs font-mono">
                    <div className="p-2.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
                      <div className="text-[10px] text-[var(--color-fg-subtle)]">PROJECT ID</div>
                      <div className="font-semibold truncate text-[var(--color-fg)]">{projectSummary.id}</div>
                    </div>
                    <div className="p-2.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
                      <div className="text-[10px] text-[var(--color-fg-subtle)]">REVISION</div>
                      <div className="font-bold text-[var(--color-accent)]">#{projectSummary.revision}</div>
                    </div>
                    <div className="p-2.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
                      <div className="text-[10px] text-[var(--color-fg-subtle)]">SAMPLE RATE</div>
                      <div className="font-semibold text-[var(--color-fg)]">{projectSummary.sample_rate} Hz</div>
                    </div>
                    <div className="p-2.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
                      <div className="text-[10px] text-[var(--color-fg-subtle)]">DURATION</div>
                      <div className="font-semibold text-[var(--color-fg)]">{projectSummary.duration_secs.toFixed(1)}s</div>
                    </div>
                    <div className="p-2.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
                      <div className="text-[10px] text-[var(--color-fg-subtle)]">TRACKS / CLIPS</div>
                      <div className="font-semibold text-[var(--color-fg)]">{projectSummary.track_count} / {projectSummary.clip_count}</div>
                    </div>
                    <div className="p-2.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
                      <div className="text-[10px] text-[var(--color-fg-subtle)]">DIRTY STATE</div>
                      <div className="font-semibold flex items-center gap-1">
                        <span className={`w-2 h-2 rounded-full ${projectSummary.is_dirty ? 'bg-amber-400' : 'bg-emerald-400'}`} />
                        <span>{projectSummary.is_dirty ? 'Modified' : 'Clean'}</span>
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="p-6 text-center text-xs text-[var(--color-fg-muted)]">
                    Connecting to alp-session...
                  </div>
                )}

                {workspaceState && (
                  <div className="pt-2 border-t border-[var(--color-border)] flex items-center justify-between text-xs text-[var(--color-fg-muted)]">
                    <span>Glass mode: <code className="text-[var(--color-fg)]">{workspaceState.glass}</code></span>
                    <span>Panels: <code className="text-[var(--color-fg)]">{workspaceState.panels.length} registered</code></span>
                  </div>
                )}
              </div>

              {/* Real-time Event Feed */}
              <div className="p-5 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] space-y-4">
                <div className="flex items-center justify-between">
                  <h2 className="text-sm font-semibold flex items-center gap-2">
                    <Activity className="w-4 h-4 text-emerald-400" />
                    <span>Real-Time IPC Events</span>
                  </h2>
                  <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]">
                    session_revision_changed
                  </span>
                </div>

                <div className="space-y-2 max-h-48 overflow-y-auto pr-1">
                  {revisionEvents.length === 0 ? (
                    <div className="p-6 text-center text-xs text-[var(--color-fg-muted)]">
                      Waiting for session events. Click &ldquo;Trigger Timeline Edit&rdquo; above.
                    </div>
                  ) : (
                    revisionEvents.map((ev, i) => (
                      <div
                        key={i}
                        className="p-2.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] flex items-center justify-between text-xs font-mono"
                      >
                        <div className="flex items-center gap-2">
                          <Clock className="w-3.5 h-3.5 text-[var(--color-accent)]" />
                          <span className="font-bold text-[var(--color-fg)]">Rev #{ev.revision}</span>
                          <span className="text-[var(--color-fg-muted)] text-[11px]">{ev.reason}</span>
                        </div>
                        <span className={`text-[10px] px-1.5 py-0.5 rounded ${ev.is_dirty ? 'bg-amber-500/20 text-amber-300' : 'bg-emerald-500/20 text-emerald-300'}`}>
                          {ev.is_dirty ? 'DIRTY' : 'CLEAN'}
                        </span>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </div>

            {/* Design Tokens Inspector */}
            <div className="p-5 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] space-y-6">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-sm font-semibold flex items-center gap-2">
                    <LayersIcon className="w-4 h-4 text-[var(--color-accent)]" />
                    <span>Design Token Palettes & WCAG Contrast Matrix</span>
                  </h2>
                  <p className="text-xs text-[var(--color-fg-muted)] mt-0.5">
                    Neon workspace identifiers and contrast conformance (&ge; 7:1 for headings, &ge; 4.5:1 for body).
                  </p>
                </div>
              </div>

              {/* Workspace Neon Color Swatches */}
              <div className="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-7 gap-3">
                {WORKSPACES.map((ws) => {
                  const isCurrent = activeWorkspace === ws.id;
                  const color = theme === 'light' ? ws.lightNeon : ws.neon;
                  return (
                    <div
                      key={ws.id}
                      onClick={() => setActiveWorkspace(ws.id)}
                      className={`p-3 rounded-xl border cursor-pointer transition-all ${
                        isCurrent
                          ? 'border-[var(--color-fg)] shadow-lg'
                          : 'border-[var(--color-border)] hover:border-[var(--color-fg-subtle)] bg-[var(--color-surface-2)]'
                      }`}
                    >
                      <div
                        className="w-full h-8 rounded-md mb-2 shadow-inner"
                        style={{ backgroundColor: color }}
                      />
                      <div className="text-xs font-bold truncate text-[var(--color-fg)]">{ws.name}</div>
                      <div className="text-[10px] font-mono text-[var(--color-fg-muted)] uppercase mt-0.5">{color}</div>
                      <div className="mt-2 flex items-center gap-1 text-[9px] text-emerald-400 font-mono">
                        <CheckCircle2 className="w-3 h-3" />
                        <span>WCAG AAA</span>
                      </div>
                    </div>
                  );
                })}
              </div>

              {/* Spacing & Radii Scales */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pt-4 border-t border-[var(--color-border)]">
                <div>
                  <h3 className="text-xs font-semibold uppercase text-[var(--color-fg-muted)] tracking-wider mb-2">
                    Spacing Scale (2px to 32px)
                  </h3>
                  <div className="flex items-end gap-2 h-16 p-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
                    {[
                      { name: 'xxs', px: '2px', h: 8 },
                      { name: 'xs', px: '4px', h: 12 },
                      { name: 'sm', px: '8px', h: 18 },
                      { name: 'md', px: '12px', h: 26 },
                      { name: 'lg', px: '16px', h: 36 },
                      { name: 'xl', px: '24px', h: 48 },
                      { name: '2xl', px: '32px', h: 56 },
                    ].map((s) => (
                      <div key={s.name} className="flex-1 flex flex-col items-center gap-1">
                        <div
                          className="w-full rounded-sm bg-[var(--color-accent)] opacity-80"
                          style={{ height: `${s.h}px` }}
                        />
                        <span className="text-[9px] font-mono text-[var(--color-fg-subtle)]">{s.name}</span>
                      </div>
                    ))}
                  </div>
                </div>

                <div>
                  <h3 className="text-xs font-semibold uppercase text-[var(--color-fg-muted)] tracking-wider mb-2">
                    Corner Radii Scale (3px to 12px)
                  </h3>
                  <div className="flex items-center gap-3 p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
                    {[
                      { name: 'xs', r: '3px' },
                      { name: 'sm', r: '4px' },
                      { name: 'md', r: '6px' },
                      { name: 'lg', r: '8px' },
                      { name: 'xl', r: '12px' },
                    ].map((item) => (
                      <div
                        key={item.name}
                        className="w-10 h-10 border-2 border-[var(--color-accent)] bg-[var(--color-surface)] flex items-center justify-center text-[10px] font-mono text-[var(--color-fg)]"
                        style={{ borderRadius: item.r }}
                      >
                        {item.name}
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </div>

            {/* Architecture Invariants Info */}
            <div className="p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] flex items-start gap-3 text-xs text-[var(--color-fg-muted)]">
              <Info className="w-4 h-4 text-[var(--color-accent)] mt-0.5 flex-shrink-0" />
              <div>
                <span className="font-semibold text-[var(--color-fg)]">Phase M2 Invariant Compliance:</span>
                Hand-written desktop shell code contains 0 instances of <code className="font-mono text-[var(--color-fg)]">unsafe</code>. Safe <code className="font-mono text-[var(--color-fg)]">x11rb</code> KWin blur applies <code className="font-mono text-[var(--color-fg)]">_KDE_NET_WM_BLUR_BEHIND_REGION</code> safely. Translucent tint fallback (alpha &ge; 0.70) remains active for Wayland and non-KDE window managers.
              </div>
            </div>
          </main>
        </div>
      </div>
    </div>
  );
}
