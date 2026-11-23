import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import {
  Mic,
  Play,
  RefreshCw,
  Info,
  DownloadCloud,
} from 'lucide-react';

import { PrimaryRail, WORKSPACE_LIST, DASHBOARD_ITEM } from './layout/PrimaryRail';
import { SubtabTopBar } from './layout/SubtabTopBar';
import { InspectorDrawer } from './layout/InspectorDrawer';
import { DockedPlayer } from './layout/DockedPlayer';
import { WindowControls } from './layout/WindowControls';
import {
  clampPanels,
  tidyPanels,
  hasOverlap,
  estimateCanvasBounds,
  type CanvasBounds,
} from './layout/panelLayout';
import { PanelContainer } from './panels/PanelContainer';
import { TimelineWaveformPanel } from './panels/TimelineWaveformPanel';
import { CommandPalette } from './components/CommandPalette';
import { ShortcutsModal } from './components/ShortcutsModal';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { useTransport } from './hooks/useTransport';

import { Button } from './kit/Button';
import { Slider } from './kit/Slider';
import { Toggle } from './kit/Toggle';
import { Meter } from './kit/Meter';
import appIcon from './assets/app-icon.png';

import {
  SettingsWorkspace,
  ModelsWorkspace,
  VoicesWorkspace,
  GenerateWorkspace,
  EffectsWorkspace,
  CapturesWorkspace,
  StoriesWorkspace,
  MultitrackWorkspace,
  DashboardWorkspace,
} from './workspaces';

import {
  getProjectSummary,
  getWorkspaceState,
  saveWorkspaceState,
  resetWorkspaceLayout,
  triggerDemoEdit,
  listenSessionRevision,
  isTauriEnvironment,
} from './ipc/client';
import type {
  ProjectSummary,
  WorkspaceState,
  PanelState,
  PanelKind,
} from './ipc/types';

// Workspace contextual subtabs
const WORKSPACE_SUBTABS: Record<string, { id: string; label: string }[]> = {
  voice: [
    { id: 'cloning', label: 'Voice Profiles & Cloning' },
    { id: 'tts', label: 'Speech Generation' },
    { id: 'models', label: 'Neural Models' },
    { id: 'lexicon', label: 'Pronunciation Lexicon' },
    { id: 'batch', label: 'Batch Takes' },
  ],
  multitrack: [
    { id: 'arranger', label: 'Timeline Arranger' },
    { id: 'mixer', label: 'Console Mixer' },
    { id: 'markers', label: 'Cue Markers' },
    { id: 'automation', label: 'Envelopes' },
  ],
  audiobook: [
    { id: 'script', label: 'Chapter Script & Cast' },
    { id: 'characters', label: 'Cast & Voices' },
    { id: 'acx_qa', label: 'ACX Compliance QA' },
    { id: 'm4b_export', label: 'M4B Package' },
  ],
  dubbing: [
    { id: 'video_sync', label: 'Video Player & Sync' },
    { id: 'stem_sep', label: 'Stem Isolation' },
    { id: 'adr', label: 'ADR Dialog Fitting' },
    { id: 'burn_in', label: 'Subtitles & Remux' },
  ],
  tools: [
    { id: 'captures', label: 'Voice Captures & VAD' },
    { id: 'denoise', label: 'Spectral De-Noise' },
    { id: 'clicks', label: 'Click & Pop Removal' },
    { id: 'loudness', label: 'LUFS Normalizer' },
  ],
  effects: [
    { id: 'eq', label: 'Parametric EQ' },
    { id: 'dynamics', label: 'Multiband Compressor' },
    { id: 'spatial', label: 'Acoustics & Reverb' },
    { id: 'vst', label: 'Hosted CLAP / LV2' },
  ],
  settings: [
    { id: 'audio_dev', label: 'Audio Hardware & ALSA' },
    { id: 'models_cat', label: 'Model Catalogue' },
    { id: 'mcp_agents', label: 'MCP Agents & Tiers' },
    { id: 'glass', label: 'KWin Glass & Blur' },
    { id: 'shortcuts', label: 'Shortcuts Parity' },
  ],
};

export default function App() {
  const [activeWorkspace, setActiveWorkspace] = useState('dashboard');
  const [activeSubtab, setActiveSubtab] = useState('cloning');
  const [viewMode, setViewMode] = useState<'workspace' | 'panels'>('workspace');
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [reduceMotion, setReduceMotion] = useState(false);
  const [isInspectorOpen, setIsInspectorOpen] = useState(false);
  const [isPaletteOpen, setIsPaletteOpen] = useState(false);
  const [isShortcutsOpen, setIsShortcutsOpen] = useState(false);

  const [projectSummary, setProjectSummary] = useState<ProjectSummary | null>(null);
  const [workspaceState, setWorkspaceState] = useState<WorkspaceState | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const canvasRef = useRef<HTMLDivElement | null>(null);
  const [canvasBounds, setCanvasBounds] = useState<CanvasBounds>({ width: 0, height: 0 });
  const isNative = isTauriEnvironment();
  const transport = useTransport();

  // Measure the floating-panel canvas so panels can be clamped on-screen.
  useEffect(() => {
    const el = canvasRef.current;
    if (!el) return;
    const update = () =>
      setCanvasBounds({ width: el.clientWidth, height: el.clientHeight });
    update();
    const observer = new ResizeObserver(update);
    observer.observe(el);
    return () => observer.disconnect();
  }, [viewMode]);

  // Panels actually rendered: clamped inside the measured canvas. If clamping
  // (e.g. after a window resize) would stack panels, re-tidy to a clean grid;
  // deliberate user-created overlap is preserved.
  const displayPanels = useMemo(() => {
    const source = workspaceState?.panels ?? [];
    const clamped = clampPanels(source, canvasBounds);
    if (canvasBounds.width > 0 && hasOverlap(clamped) && !hasOverlap(source)) {
      return tidyPanels(clamped, canvasBounds);
    }
    return clamped;
  }, [workspaceState, canvasBounds]);

  // Active workspace metadata
  const currentWsObj =
    activeWorkspace === 'dashboard'
      ? DASHBOARD_ITEM
      : WORKSPACE_LIST.find((w) => w.id === activeWorkspace) || WORKSPACE_LIST[0];
  const activeColor = theme === 'light' ? currentWsObj.lightNeon : currentWsObj.neon;

  // Apply theme & reduce-motion to root
  useEffect(() => {
    const root = document.documentElement;
    if (theme === 'light') {
      root.classList.add('theme-light');
    } else {
      root.classList.remove('theme-light');
    }
    if (reduceMotion) {
      root.classList.add('reduce-motion');
    } else {
      root.classList.remove('reduce-motion');
    }
  }, [theme, reduceMotion]);

  // Apply active accent token
  useEffect(() => {
    document.documentElement.style.setProperty('--color-accent', activeColor);
  }, [activeColor]);

  // Load project & workspace from backend on startup
  useEffect(() => {
    fetchInitialState();

    let unlisten: (() => void) | undefined;
    listenSessionRevision((payload) => {
      setProjectSummary((prev) =>
        prev
          ? { ...prev, revision: payload.revision, is_dirty: payload.is_dirty }
          : null
      );
      getProjectSummary().then(setProjectSummary).catch(console.error);
    }).then((cleanup) => {
      unlisten = cleanup;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  async function fetchInitialState() {
    setIsLoading(true);
    try {
      const [summary, ws] = await Promise.all([
        getProjectSummary(),
        getWorkspaceState(),
      ]);
      setProjectSummary(summary);
      const bounds = estimateCanvasBounds();
      const panels = hasOverlap(ws.panels)
        ? tidyPanels(ws.panels, bounds)
        : clampPanels(ws.panels, bounds);
      setWorkspaceState({ ...ws, panels });
      if (ws.theme === 'light' || ws.theme === 'dark') {
        setTheme(ws.theme as 'dark' | 'light');
      }
      setReduceMotion(ws.reduce_motion);
    } catch (err) {
      console.error('Failed to load initial state:', err);
    } finally {
      setIsLoading(false);
    }
  }

  // Handle workspace change
  const handleSelectWorkspace = useCallback((id: string) => {
    setActiveWorkspace(id);
    const subtabs = WORKSPACE_SUBTABS[id];
    if (subtabs && subtabs.length > 0) {
      setActiveSubtab(subtabs[0].id);
    }
    if (id === 'multitrack') {
      setViewMode('panels');
    } else {
      setViewMode('workspace');
    }
  }, []);

  // Update a single panel's geometry/state and persist
  const handleUpdatePanel = useCallback(
    (updated: PanelState) => {
      setWorkspaceState((prev) => {
        if (!prev) return null;
        const nextPanels = prev.panels.map((p) =>
          p.kind === updated.kind ? updated : p
        );
        const nextState = { ...prev, panels: nextPanels };
        saveWorkspaceState(nextState).catch(console.error);
        return nextState;
      });
    },
    []
  );

  // Close panel
  const handleClosePanel = useCallback((kind: PanelKind) => {
    setWorkspaceState((prev) => {
      if (!prev) return null;
      const nextPanels = prev.panels.map((p) =>
        p.kind === kind ? { ...p, open: false } : p
      );
      const nextState = { ...prev, panels: nextPanels };
      saveWorkspaceState(nextState).catch(console.error);
      return nextState;
    });
  }, []);

  // Focus panel (brings to front by incrementing max Z)
  const handleFocusPanel = useCallback((kind: PanelKind) => {
    setWorkspaceState((prev) => {
      if (!prev) return null;
      const maxZ = Math.max(...prev.panels.map((p) => p.z), 1);
      const nextPanels = prev.panels.map((p) =>
        p.kind === kind ? { ...p, z: maxZ + 1 } : p
      );
      return { ...prev, panels: nextPanels };
    });
  }, []);

  // Toggle panel visibility
  const handleTogglePanel = useCallback((kind: PanelKind) => {
    setWorkspaceState((prev) => {
      if (!prev) return null;
      const target = prev.panels.find((p) => p.kind === kind);
      const nextPanels = target
        ? prev.panels.map((p) =>
            p.kind === kind ? { ...p, open: !p.open } : p
          )
        : [
            ...prev.panels,
            {
              kind,
              rect: { x: 120, y: 100, width: 440, height: 320 },
              z: 10,
              collapsed: false,
              docked: null,
              open: true,
            },
          ];
      const nextState = { ...prev, panels: nextPanels };
      saveWorkspaceState(nextState).catch(console.error);
      return nextState;
    });
  }, []);

  // Reset panels to default arrangement
  const handleResetLayout = useCallback(async () => {
    try {
      const reset = await resetWorkspaceLayout();
      const bounds =
        canvasBounds.width > 0 ? canvasBounds : estimateCanvasBounds();
      setWorkspaceState({ ...reset, panels: tidyPanels(reset.panels, bounds) });
    } catch (err) {
      console.error('Failed to reset layout:', err);
    }
  }, [canvasBounds]);

  // Theme cycle
  const handleToggleTheme = useCallback(() => {
    setTheme((t) => {
      const next = t === 'dark' ? 'light' : 'dark';
      setWorkspaceState((prev) => {
        if (!prev) return null;
        const nextState = { ...prev, theme: next };
        saveWorkspaceState(nextState).catch(console.error);
        return nextState;
      });
      return next;
    });
  }, []);

  // Toggle reduce motion
  const handleToggleReduceMotion = useCallback(() => {
    setReduceMotion((m) => {
      const next = !m;
      setWorkspaceState((prev) => {
        if (!prev) return null;
        const nextState = { ...prev, reduce_motion: next };
        saveWorkspaceState(nextState).catch(console.error);
        return nextState;
      });
      return next;
    });
  }, []);

  // Trigger demo edit
  const handleTriggerEdit = useCallback(async () => {
    try {
      await triggerDemoEdit(`Timeline split take @ ${new Date().toLocaleTimeString()}`);
    } catch (err) {
      console.error('Failed to trigger edit:', err);
    }
  }, []);

  // Register global keyboard shortcuts
  useKeyboardShortcuts({
    onSelectWorkspace: handleSelectWorkspace,
    onToggleCommandPalette: () => setIsPaletteOpen((prev) => !prev),
    onToggleShortcuts: () => setIsShortcutsOpen((prev) => !prev),
    onToggleTheme: handleToggleTheme,
    onToggleReduceMotion: handleToggleReduceMotion,
    onToggleInspector: () => setIsInspectorOpen((prev) => !prev),
  });

  return (
    <div className="h-screen w-screen flex flex-col bg-[var(--color-bg)] text-[var(--color-fg)] font-sans antialiased overflow-hidden select-none">
      {/* Top Application Bar */}
      <header className="h-10 border-b border-[var(--color-border)] bg-[var(--color-surface)] flex items-center gap-3 pl-3 pr-1 flex-shrink-0 z-40">
        <div className="flex items-center gap-2 flex-shrink-0">
          <img src={appIcon} alt="" aria-hidden="true" className="h-5 w-5 rounded" />
          <span className="font-bold tracking-tight text-xs text-[var(--color-fg)]">
            AUDIO LABS PRO
          </span>
        </div>

        <div
          className={`flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-mono flex-shrink-0 ${
            isNative
              ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/30'
              : 'bg-amber-500/10 text-amber-400 border border-amber-500/30'
          }`}
        >
          <span
            className={`w-1.5 h-1.5 rounded-full ${isNative ? 'bg-emerald-400' : 'bg-amber-400'}`}
          />
          {isNative ? 'Native' : 'Mock'}
        </div>

        {/* Draggable title-bar region (frameless window) */}
        <div className="flex-1 h-full" data-tauri-drag-region />

        <div className="flex items-center gap-2 text-xs flex-shrink-0">
          {projectSummary && (
            <div className="hidden md:flex items-center gap-2 text-[11px] font-mono text-[var(--color-fg-muted)]">
              <span>Rev #{projectSummary.revision}</span>
              <span className={projectSummary.is_dirty ? 'text-amber-400' : 'text-emerald-400'}>
                {projectSummary.is_dirty ? 'Modified' : 'Saved'}
              </span>
            </div>
          )}

          <Button
            variant="ghost"
            size="xs"
            onClick={fetchInitialState}
            disabled={isLoading}
            leftIcon={<RefreshCw className={`w-3 h-3 ${isLoading ? 'animate-spin' : ''}`} />}
            aria-label="Sync project state"
          >
            Sync
          </Button>
        </div>

        <WindowControls />
      </header>

      {/* Main Dual-Axis Workspace Layout */}
      <div className="flex-1 flex overflow-hidden relative">
        {/* Left Primary Rail (56px) */}
        <PrimaryRail
          activeWorkspace={activeWorkspace}
          onSelectWorkspace={handleSelectWorkspace}
          onOpenCommandPalette={() => setIsPaletteOpen(true)}
          onOpenShortcuts={() => setIsShortcutsOpen(true)}
          onToggleTheme={handleToggleTheme}
          theme={theme}
        />

        {/* Center Canvas Area: Subtab Top Bar + Floating Workspace Canvas */}
        <div className="flex-1 flex flex-col overflow-hidden relative bg-[var(--color-bg)]">
          {/* Top Subtab Bar (40px) */}
          <SubtabTopBar
            activeWorkspaceName={currentWsObj.name}
            activeColor={activeColor}
            activeSubtab={activeSubtab}
            onSelectSubtab={setActiveSubtab}
            subtabs={WORKSPACE_SUBTABS[activeWorkspace] || []}
            isInspectorOpen={isInspectorOpen}
            onToggleInspector={() => setIsInspectorOpen(!isInspectorOpen)}
            onTogglePanel={handleTogglePanel}
            onResetLayout={handleResetLayout}
            onTriggerEdit={handleTriggerEdit}
            isDirty={projectSummary?.is_dirty}
            viewMode={viewMode}
            onToggleViewMode={() =>
              setViewMode((v) => (v === 'workspace' ? 'panels' : 'workspace'))
            }
            hideStudioActions={activeWorkspace === 'dashboard'}
          />

          {/* Main Canvas Surface */}
          <main className="flex-1 relative overflow-hidden">
            {viewMode === 'workspace' ? (
              <div
                className={`h-full w-full overflow-hidden ${
                  activeWorkspace === 'multitrack' ? '' : 'overflow-y-auto p-4'
                }`}
              >
                {renderActiveWorkspace(
                  activeWorkspace,
                  activeSubtab,
                  theme,
                  handleToggleTheme,
                  reduceMotion,
                  () => setReduceMotion(!reduceMotion),
                  transport,
                  handleSelectWorkspace,
                  () => setIsPaletteOpen(true),
                  () => setIsShortcutsOpen(true)
                )}
              </div>
            ) : (
              <div ref={canvasRef} className="h-full w-full relative overflow-hidden p-4">
                {/* Background Workspace Wallpaper & Watermark */}
                <div className="absolute inset-0 pointer-events-none flex items-center justify-center opacity-5 select-none">
                  <span className="text-8xl font-black tracking-widest uppercase">
                    {currentWsObj.name}
                  </span>
                </div>

                {/* Render Floating Panels from Persisted Workspace (clamped on-screen) */}
                {displayPanels.map((panel) => (
                  <PanelContainer
                    key={panel.kind}
                    panel={panel}
                    bounds={canvasBounds}
                    onUpdate={handleUpdatePanel}
                    onClose={handleClosePanel}
                    onFocus={handleFocusPanel}
                  >
                    {renderPanelContent(panel.kind, transport)}
                  </PanelContainer>
                ))}
              </div>
            )}
          </main>
        </div>

        {/* Right Collapsible Inspector Drawer (280px, toggled via Ctrl+B) */}
        <InspectorDrawer
          open={isInspectorOpen}
          onClose={() => setIsInspectorOpen(false)}
          activeWorkspaceName={currentWsObj.name}
        />
      </div>

      {/* Bottom Docked Transport & Playback Player Bar (64px) */}
      <DockedPlayer
        durationSecs={transport.durationSecs}
        sampleRate={projectSummary?.sample_rate || 48000}
        playing={transport.playing}
        currentTime={transport.positionSecs}
        volume={transport.volume}
        loopEnabled={transport.loopEnabled}
        peakL={transport.peakL}
        peakR={transport.peakR}
        isClipping={transport.isClipping}
        onTogglePlay={transport.togglePlay}
        onStop={transport.stop}
        onSeek={transport.seek}
        onSetVolume={transport.setVolume}
        onToggleLoop={transport.toggleLoop}
      />

      {/* Command Palette Modal (Ctrl+K) */}
      <CommandPalette
        open={isPaletteOpen}
        onClose={() => setIsPaletteOpen(false)}
        onSelectWorkspace={handleSelectWorkspace}
        onTogglePanel={handleTogglePanel}
        onResetLayout={handleResetLayout}
        onToggleTheme={handleToggleTheme}
        onOpenShortcuts={() => {
          setIsPaletteOpen(false);
          setIsShortcutsOpen(true);
        }}
        onTriggerEdit={handleTriggerEdit}
        theme={theme}
      />

      {/* Keyboard Shortcuts Cheatsheet Modal (F1) */}
      <ShortcutsModal
        open={isShortcutsOpen}
        onClose={() => setIsShortcutsOpen(false)}
      />
    </div>
  );
}

/** Render active workspace view based on primary rail and subtabs */
function renderActiveWorkspace(
  workspace: string,
  subtab: string,
  theme: 'dark' | 'light',
  onToggleTheme: () => void,
  reduceMotion: boolean,
  onToggleReduceMotion: () => void,
  transport: ReturnType<typeof useTransport>,
  onOpenWorkspace: (id: string) => void,
  onOpenPalette: () => void,
  onOpenShortcuts: () => void
) {
  switch (workspace) {
    case 'dashboard':
      return (
        <DashboardWorkspace
          theme={theme}
          onOpenWorkspace={onOpenWorkspace}
          onOpenPalette={onOpenPalette}
          onOpenShortcuts={onOpenShortcuts}
        />
      );

    case 'settings':
      if (subtab === 'models_cat') {
        return <ModelsWorkspace />;
      }
      return (
        <SettingsWorkspace
          theme={theme}
          onToggleTheme={onToggleTheme}
          reduceMotion={reduceMotion}
          onToggleReduceMotion={onToggleReduceMotion}
        />
      );

    case 'voice':
      if (subtab === 'tts' || subtab === 'batch') {
        return <GenerateWorkspace />;
      }
      if (subtab === 'models') {
        return <ModelsWorkspace />;
      }
      return <VoicesWorkspace />;

    case 'effects':
      return <EffectsWorkspace />;

    case 'audiobook':
      return <StoriesWorkspace />;

    case 'tools':
      return <CapturesWorkspace onSeek={transport.seek} />;

    case 'dubbing':
      return <GenerateWorkspace />;

    case 'multitrack':
    default:
      return (
        <MultitrackWorkspace
          currentTimeSecs={transport.positionSecs}
          isPlaying={transport.playing}
          onSeek={transport.seek}
          onTogglePlay={transport.togglePlay}
        />
      );
  }
}

/** Render distinct internal content for each studio floating panel using kit primitives */
function renderPanelContent(
  kind: PanelKind,
  transport: ReturnType<typeof useTransport>
) {
  switch (kind) {
    case 'timeline':
      return (
        <TimelineWaveformPanel
          currentTime={transport.positionSecs}
          duration={transport.durationSecs}
          isPlaying={transport.playing}
          loopEnabled={transport.loopEnabled}
          loopStart={transport.loopStartSecs}
          loopEnd={transport.loopEndSecs}
          onSeek={transport.seek}
          onTogglePlay={transport.togglePlay}
          onToggleLoop={transport.toggleLoop}
        />
      );

    case 'mixer':
      return (
        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-3">
            <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] space-y-2">
              <div className="text-[11px] font-bold text-[var(--color-accent)]">Track 1</div>
              <Slider label="Fader" value={-2.5} min={-36} max={12} onChange={() => {}} unit="dB" />
              <div className="h-14 flex justify-center">
                <Meter leftLevel={0.72} rightLevel={0.68} orientation="vertical" showLabels={false} />
              </div>
            </div>
            <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] space-y-2">
              <div className="text-[11px] font-bold text-emerald-400">Master</div>
              <Slider label="Fader" value={0.0} min={-36} max={12} onChange={() => {}} unit="dB" />
              <div className="h-14 flex justify-center">
                <Meter leftLevel={0.84} rightLevel={0.82} orientation="vertical" showLabels={false} />
              </div>
            </div>
          </div>
        </div>
      );

    case 'transport':
      return (
        <div className="flex flex-col items-center justify-center gap-3 py-2">
          <div className="text-xl font-mono font-bold text-[var(--color-accent)]">
            00:00:12.45
          </div>
          <div className="flex items-center gap-2">
            <Button variant="secondary" size="xs">0:00</Button>
            <Button variant="primary" size="sm" leftIcon={<Play className="w-3.5 h-3.5 fill-current" />}>
              Play
            </Button>
            <Button variant="secondary" size="xs">Loop</Button>
          </div>
        </div>
      );

    case 'voice_lab':
      return (
        <div className="space-y-3">
          <div className="text-xs font-semibold text-[var(--color-fg)]">
            Zero-Shot Voice Cloning
          </div>
          <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] space-y-2">
            <div className="flex items-center gap-2 text-xs">
              <Mic className="w-4 h-4 text-cyan-400" />
              <span>Reference Sample: <code>host_reference.wav</code></span>
            </div>
            <div className="text-[11px] text-[var(--color-fg-muted)]">
              Quality Score: <span className="text-emerald-400 font-bold">94/100 (Clean)</span>
            </div>
            <Button variant="primary" size="xs" className="w-full">
              Generate Neural Embedding
            </Button>
          </div>
        </div>
      );

    case 'effects':
      return (
        <div className="space-y-2">
          {['Parametric 8-Band EQ', 'Multiband Dynamics', 'Convolution Reverb'].map((fx) => (
            <div
              key={fx}
              className="p-2.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] flex items-center justify-between"
            >
              <span className="font-medium text-xs">{fx}</span>
              <Toggle checked={true} onChange={() => {}} size="sm" />
            </div>
          ))}
        </div>
      );

    case 'models':
      return (
        <div className="space-y-2">
          <div className="p-2.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] flex items-center justify-between text-xs">
            <div>
              <div className="font-semibold">Matcha-TTS Voice Model</div>
              <div className="text-[11px] text-[var(--color-fg-subtle)] font-mono">ONNX Runtime &bull; 85 MB</div>
            </div>
            <span className="text-[10px] text-emerald-400 font-bold">ACTIVE</span>
          </div>
        </div>
      );

    case 'media_bin':
      return (
        <div className="space-y-1.5 font-mono text-[11px]">
          {['take-01-lead.wav', 'intro-acoustic.wav', 'voice-cloned-take.wav'].map((f) => (
            <div
              key={f}
              className="p-2 rounded bg-[var(--color-surface-2)] border border-[var(--color-border)] flex items-center justify-between"
            >
              <span>{f}</span>
              <span className="text-[var(--color-fg-subtle)]">48kHz &bull; 32f</span>
            </div>
          ))}
        </div>
      );

    case 'ai_activity':
      return (
        <div className="space-y-2 font-mono text-[11px]">
          <div className="p-2 rounded bg-[var(--color-surface-2)] border border-[var(--color-border)] text-emerald-400">
            [MCP Attached]: Edit tier authenticated
          </div>
          <div className="p-2 rounded bg-[var(--color-surface-2)] border border-[var(--color-border)] text-[var(--color-fg-muted)]">
            Listening for socket commands on /tmp/alp.sock...
          </div>
        </div>
      );

    case 'inspector':
      return (
        <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
          <Info className="w-5 h-5 text-[var(--color-fg-subtle)]" aria-hidden="true" />
          <p className="text-xs font-medium text-[var(--color-fg)]">No clip selected</p>
          <p className="text-[11px] text-[var(--color-fg-muted)] max-w-[220px]">
            Select a clip on the timeline to inspect its gain, fades and source here.
          </p>
        </div>
      );

    case 'export_queue':
      return (
        <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
          <DownloadCloud className="w-5 h-5 text-[var(--color-fg-subtle)]" aria-hidden="true" />
          <p className="text-xs font-medium text-[var(--color-fg)]">No renders queued</p>
          <p className="text-[11px] text-[var(--color-fg-muted)] max-w-[220px]">
            Exported masters and stems will appear here with their progress.
          </p>
        </div>
      );

    default:
      return (
        <div className="p-4 text-center text-[var(--color-fg-muted)]">
          Panel content active.
        </div>
      );
  }
}
