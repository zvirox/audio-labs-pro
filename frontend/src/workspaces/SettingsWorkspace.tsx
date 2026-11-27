import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Toggle } from '../kit/Toggle';
import { SHORTCUT_GROUPS } from '../components/ShortcutsModal';
import {
  getAudioDevices,
  getGlassReport,
  getProjectSummary,
  getAiStatus,
  toggleAiControl,
  setAiPermissionTier,
  undoAiChange,
  getAiActivityLog,
  respondAiApproval,
  onAiActivityEvent,
  onAiClientApproval,
  playTransport,
  stopTransport,
} from '../ipc/client';
import type {
  AudioDevice,
  GlassReport,
  ProjectSummary,
  AiControlStatus,
  AiActivityEntry,
  PermissionTier,
} from '../ipc/types';
import {
  Sliders,
  Volume2,
  FileCheck,
  ShieldCheck,
  Copy,
  Check,
  RotateCcw,
  Terminal,
  AlertCircle,
  Monitor,
  Keyboard,
  LayoutGrid,
} from 'lucide-react';

export interface SettingsWorkspaceProps {
  subtab?: string;
  theme: 'dark' | 'light';
  onToggleTheme?: () => void;
  reduceMotion: boolean;
  onToggleReduceMotion: () => void;
  onResetLayout?: () => void;
  onOpenShortcuts?: () => void;
}

export function SettingsWorkspace({
  subtab = 'audio_dev',
  theme,
  onToggleTheme,
  reduceMotion,
  onToggleReduceMotion,
  onResetLayout,
  onOpenShortcuts,
}: SettingsWorkspaceProps) {
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [glass, setGlass] = useState<GlassReport | null>(null);
  const [summary, setSummary] = useState<ProjectSummary | null>(null);
  const [selectedOutput, setSelectedOutput] = useState<string>('');
  const [selectedInput, setSelectedInput] = useState<string>('');
  const [testToneActive, setTestToneActive] = useState<boolean>(false);
  const [exportNotice, setExportNotice] = useState<string | null>(null);
  const [shortcutFilter, setShortcutFilter] = useState<string>('');

  // AI & MCP Attach State
  const [aiStatus, setAiStatus] = useState<AiControlStatus | null>(null);
  const [activityLogs, setActivityLogs] = useState<AiActivityEntry[]>([]);
  const [copiedToken, setCopiedToken] = useState<boolean>(false);
  const [copiedCmd, setCopiedCmd] = useState<boolean>(false);
  const [undoNotice, setUndoNotice] = useState<string | null>(null);
  const [pendingApproval, setPendingApproval] = useState<string | null>(null);

  useEffect(() => {
    getAudioDevices().then((devs) => {
      setDevices(devs);
      const defaultOut = devs.find((d) => d.direction === 'output' && d.is_default);
      if (defaultOut) setSelectedOutput(defaultOut.name);
      const defaultIn = devs.find((d) => d.direction === 'input' && d.is_default);
      if (defaultIn) setSelectedInput(defaultIn.name);
    });

    getGlassReport().then(setGlass);
    getProjectSummary().then(setSummary);
    getAiStatus().then(setAiStatus);
    getAiActivityLog().then(setActivityLogs);

    const unlistenActivity = onAiActivityEvent((entry) => {
      setActivityLogs((prev) => [entry, ...prev.slice(0, 19)]);
      getAiStatus().then(setAiStatus);
      getProjectSummary().then(setSummary);
    });

    const unlistenApproval = onAiClientApproval((payload) => {
      setPendingApproval(payload.clientId);
    });

    return () => {
      unlistenActivity.then((fn) => fn());
      unlistenApproval.then((fn) => fn());
    };
  }, []);

  const handleTestPlayback = async () => {
    if (testToneActive) {
      await stopTransport().catch(() => {});
      setTestToneActive(false);
    } else {
      setTestToneActive(true);
      await playTransport().catch(() => {});
      setTimeout(async () => {
        await stopTransport().catch(() => {});
        setTestToneActive(false);
      }, 900);
    }
  };

  const handleExportPreset = (preset: string, lufs: number) => {
    setExportNotice(`Active delivery profile switched to ${preset} (${lufs} LUFS target)`);
  };

  const handleToggleAi = async () => {
    try {
      const updated = await toggleAiControl();
      setAiStatus(updated);
      setActivityLogs(await getAiActivityLog());
    } catch (err) {
      console.error('Failed to toggle AI control:', err);
    }
  };

  const handleTierChange = async (tier: PermissionTier) => {
    try {
      const updated = await setAiPermissionTier(tier);
      setAiStatus(updated);
      setActivityLogs(await getAiActivityLog());
    } catch (err) {
      console.error('Failed to set AI tier:', err);
    }
  };

  const handleCopyToken = () => {
    if (!aiStatus?.token) return;
    navigator.clipboard.writeText(aiStatus.token);
    setCopiedToken(true);
    setTimeout(() => setCopiedToken(false), 2000);
  };

  const handleCopyCommand = () => {
    if (!aiStatus?.token) return;
    const cmd = `ALP_CONTROL_TOKEN='${aiStatus.token}' alp-mcp --attach`;
    navigator.clipboard.writeText(cmd);
    setCopiedCmd(true);
    setTimeout(() => setCopiedCmd(false), 2000);
  };

  const handleUndoAi = async () => {
    try {
      await undoAiChange();
      setSummary(await getProjectSummary());
      setUndoNotice('Rolled back last AI edit');
    } catch (err) {
      setUndoNotice(`Undo failed: ${err}`);
    }
  };

  const handleApproval = async (decision: 'session' | 'always' | 'deny') => {
    if (!pendingApproval) return;
    await respondAiApproval(pendingApproval, decision, decision === 'always');
    setPendingApproval(null);
    getAiStatus().then(setAiStatus);
    getAiActivityLog().then(setActivityLogs);
  };

  const outputs = devices.filter((d) => d.direction === 'output');
  const inputs = devices.filter((d) => d.direction === 'input');

  const headerMeta: Record<string, { title: string; desc: string }> = {
    audio_dev: {
      title: 'Audio Hardware & ALSA Routing',
      desc: 'Select active PipeWire / ALSA output sinks, capture microphones, and loudness delivery targets.',
    },
    mcp_agents: {
      title: 'MCP Agents, Permission Tiers & Audit',
      desc: 'Control the live UNIX socket attach server, token permissions, client approvals, and AI undo log.',
    },
    appearance: {
      title: 'Window & Surface Architecture',
      desc: 'Inspect opaque Linear surface telemetry, motion preferences, storage directories, and layout recovery.',
    },
    shortcuts: {
      title: 'Keyboard Shortcuts & Workflow Parity',
      desc: 'Complete global navigation, playback transport, and multitrack/spectral editing keybindings.',
    },
  };

  const activeHeader = headerMeta[subtab] || headerMeta.audio_dev;

  const filteredGroups = SHORTCUT_GROUPS.map((group) => ({
    ...group,
    items: group.items.filter(
      (item) =>
        !shortcutFilter.trim() ||
        item.desc.toLowerCase().includes(shortcutFilter.toLowerCase()) ||
        item.keys.toLowerCase().includes(shortcutFilter.toLowerCase())
    ),
  })).filter((g) => g.items.length > 0);

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      <div>
        <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
          {subtab === 'appearance' ? (
            <Monitor className="w-4 h-4 text-[var(--color-accent)]" />
          ) : subtab === 'shortcuts' ? (
            <Keyboard className="w-4 h-4 text-[var(--color-accent)]" />
          ) : subtab === 'mcp_agents' ? (
            <ShieldCheck className="w-4 h-4 text-[var(--color-accent)]" />
          ) : (
            <Sliders className="w-4 h-4 text-[var(--color-accent)]" />
          )}
          {activeHeader.title}
        </h2>
        <p className="text-xs text-[var(--color-fg-muted)] mt-1">{activeHeader.desc}</p>
      </div>

      {/* Subtab 1: Audio Hardware & ALSA */}
      {subtab === 'audio_dev' && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-7">
            <Card title="Audio Hardware (PipeWire / ALSA)" subtitle={`${devices.length} detected host endpoint(s)`}>
              <div className="space-y-4 text-xs">
                <div>
                  <label className="block text-[var(--color-fg-muted)] mb-1.5 font-medium">Output Device (Sink)</label>
                  <select
                    value={selectedOutput}
                    onChange={(e) => setSelectedOutput(e.target.value)}
                    className="w-full px-3 py-2 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)] font-mono text-xs"
                  >
                    {outputs.length === 0 && <option value="">No ALSA/PipeWire output endpoints found</option>}
                    {outputs.map((dev) => (
                      <option key={`out-${dev.name}`} value={dev.name}>
                        {dev.name} ({dev.channels}ch · {dev.sample_rate / 1000}kHz · {dev.sample_format})
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label className="block text-[var(--color-fg-muted)] mb-1.5 font-medium">Capture Device (Microphone)</label>
                  <select
                    value={selectedInput}
                    onChange={(e) => setSelectedInput(e.target.value)}
                    className="w-full px-3 py-2 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)] font-mono text-xs"
                  >
                    {inputs.length === 0 && <option value="">No ALSA/PipeWire input endpoints found</option>}
                    {inputs.map((dev) => (
                      <option key={`in-${dev.name}`} value={dev.name}>
                        {dev.name} ({dev.channels}ch · {dev.sample_rate / 1000}kHz · {dev.sample_format})
                      </option>
                    ))}
                  </select>
                </div>

                <div className="pt-2 flex items-center justify-between border-t border-[var(--color-border)]">
                  <span className="text-[var(--color-fg-muted)] font-mono">
                    Session Rate: {summary ? `${summary.sample_rate} Hz` : '48000 Hz'} · Lock-free SPSC bus
                  </span>
                  <Button
                    variant={testToneActive ? 'primary' : 'secondary'}
                    size="sm"
                    onClick={handleTestPlayback}
                    leftIcon={<Volume2 className="w-4 h-4" />}
                  >
                    {testToneActive ? 'Testing Audio Output...' : 'Test Transport Output'}
                  </Button>
                </div>
              </div>
            </Card>
          </div>

          <div className="lg:col-span-5">
            <Card title="Delivery & Export Presets" subtitle="Loudness-calibrated export workflows">
              <div className="space-y-3 text-xs">
                <div className="flex flex-col gap-2">
                  {[
                    { name: 'Podcast Stereo', lufs: -16 },
                    { name: 'Audiobook ACX', lufs: -20 },
                    { name: 'Broadcast Voice-over', lufs: -24 },
                  ].map((preset) => (
                    <Button
                      key={preset.name}
                      variant="secondary"
                      size="sm"
                      onClick={() => handleExportPreset(preset.name, preset.lufs)}
                      leftIcon={<FileCheck className="w-4 h-4 text-[var(--color-accent)]" />}
                    >
                      {preset.name} ({preset.lufs} LUFS)
                    </Button>
                  ))}
                </div>
                {exportNotice && (
                  <div className="p-2.5 rounded-lg bg-primary-tint border border-border-strong text-primary-text font-mono text-[11px]">
                    {exportNotice}
                  </div>
                )}
              </div>
            </Card>
          </div>
        </div>
      )}

      {/* Subtab 2: MCP Agents & Tiers */}
      {subtab === 'mcp_agents' && (
        <div className="space-y-6">
          {pendingApproval && (
            <div className="p-4 rounded-lg bg-surface-2 border border-amber flex items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <AlertCircle className="w-4 h-4 text-amber shrink-0" />
                <div>
                  <div className="font-medium text-fg text-[13px]">
                    AI Client Connection Request: <span className="font-mono text-primary-text">"{pendingApproval}"</span>
                  </div>
                  <div className="text-xs text-[var(--color-fg-muted)]">
                    A local agent authenticated with your session token and requests attachment to this workspace.
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <Button variant="secondary" size="sm" onClick={() => handleApproval('deny')}>Deny</Button>
                <Button variant="secondary" size="sm" onClick={() => handleApproval('session')}>Approve Session</Button>
                <Button variant="primary" size="sm" onClick={() => handleApproval('always')}>Always Approve</Button>
              </div>
            </div>
          )}

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <Card title="AI Agent Control & Security" subtitle="87 typed MCP tools over UNIX domain socket">
              <div className="space-y-4 text-xs">
                <div className="flex items-center justify-between p-3 rounded-lg bg-surface-2 border border-border">
                  <div className="flex items-center gap-2.5">
                    <div className={`w-2.5 h-2.5 rounded-full ${aiStatus?.enabled ? 'bg-sky' : 'bg-faint'}`} />
                    <div>
                      <div className="font-medium text-fg flex items-center gap-2">
                        <span>MCP Attach Host</span>
                        <span
                          className={`text-[10px] px-2 py-0.5 rounded font-mono font-medium border ${
                            aiStatus?.enabled
                              ? 'bg-chip-sky text-sky-text border-border-strong'
                              : 'bg-surface text-muted border-border'
                          }`}
                        >
                          {aiStatus?.enabled ? 'ACTIVE' : 'STANDBY'}
                        </span>
                      </div>
                      <div className="text-[11px] text-muted font-mono mt-0.5 truncate max-w-xs">
                        {aiStatus?.enabled
                          ? aiStatus.socket_path || '$XDG_RUNTIME_DIR/alp/control.sock'
                          : 'Unix socket standby'}
                      </div>
                    </div>
                  </div>
                  <Button
                    variant={aiStatus?.enabled ? 'danger' : 'secondary'}
                    size="sm"
                    onClick={handleToggleAi}
                  >
                    {aiStatus?.enabled ? 'Stop MCP' : 'Allow MCP'}
                  </Button>
                </div>

                <div>
                  <div className="flex items-center justify-between mb-1.5">
                    <label className="text-muted font-medium">Permission Tier</label>
                    <span className="text-muted text-[11px]">Enforced on all 87 tool calls</span>
                  </div>
                  <div className="grid grid-cols-3 gap-2">
                    {(['observe', 'edit', 'export'] as const).map((tier) => (
                      <button
                        key={tier}
                        type="button"
                        onClick={() => handleTierChange(tier)}
                        className={`py-2 px-3 rounded-md font-medium text-center capitalize border transition-colors cursor-pointer ${
                          aiStatus?.tier === tier
                            ? 'bg-primary-tint text-primary-text border-border-strong'
                            : 'bg-surface text-muted border-border hover:text-fg'
                        }`}
                      >
                        {tier}
                      </button>
                    ))}
                  </div>
                </div>

                {aiStatus?.enabled && (
                  <div className="space-y-2 p-3 rounded-lg bg-[var(--color-bg)] border border-[var(--color-border)] font-mono text-[11px]">
                    <div className="flex items-center justify-between">
                      <span className="text-[var(--color-fg-muted)]">Session Token:</span>
                      <div className="flex items-center gap-1.5">
                        <span className="text-[var(--color-fg-muted)] truncate max-w-[140px]">
                          {aiStatus.token ? `${aiStatus.token.slice(0, 16)}...` : 'None'}
                        </span>
                        <button
                          type="button"
                          onClick={handleCopyToken}
                          aria-label="Copy session token"
                          className="p-1 rounded hover:bg-[var(--color-surface-2)] text-[var(--color-accent)]"
                        >
                          {copiedToken ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
                        </button>
                      </div>
                    </div>
                    <div className="flex items-center justify-between pt-1 border-t border-[var(--color-border)]">
                      <span className="text-[var(--color-fg-muted)]">CLI Attach:</span>
                      <button
                        type="button"
                        onClick={handleCopyCommand}
                        className="flex items-center gap-1 text-primary-text hover:text-fg text-[10px] underline cursor-pointer"
                      >
                        <Terminal className="w-4 h-4" />
                        <span>{copiedCmd ? 'Command Copied!' : 'Copy Shell Command'}</span>
                      </button>
                    </div>
                  </div>
                )}

                <div className="flex items-center justify-between pt-2 border-t border-[var(--color-border)]">
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={handleUndoAi}
                    leftIcon={<RotateCcw className="w-4 h-4" />}
                  >
                    Undo Last AI Edit
                  </Button>
                  {undoNotice && <span className="text-[11px] text-primary-text font-mono">{undoNotice}</span>}
                </div>
              </div>
            </Card>

            <Card
              title="AI Agent Activity Audit Log"
              subtitle="Real-time log of autonomous tool dispatches and undo commits"
            >
              <div className="space-y-2 text-xs">
                {activityLogs.length === 0 ? (
                  <div className="p-6 text-center text-[var(--color-fg-muted)] italic">
                    No agent activity recorded yet.
                  </div>
                ) : (
                  <div className="max-h-80 overflow-y-auto space-y-1 pr-1 font-mono text-[11px]">
                    {activityLogs.map((log, idx) => (
                      <div
                        key={`${log.timestamp}-${idx}`}
                        className="p-2 rounded bg-[var(--color-bg)] border border-[var(--color-border)] flex items-center justify-between gap-3"
                      >
                        <div className="flex items-center gap-2 truncate">
                          <span className="text-[var(--color-fg-muted)] text-[10px]">{log.timestamp}</span>
                          <span className="text-[var(--color-accent)] font-medium">{log.client_id}</span>
                          <span className="text-[var(--color-fg-muted)]">›</span>
                          <span className="text-[var(--color-fg)] font-medium">{log.operation}</span>
                        </div>
                        <span
                          className={`px-1.5 py-0.5 rounded text-[10px] font-medium border ${
                            log.committed
                              ? 'bg-chip-sky text-sky-text border-border-strong'
                              : log.outcome.startsWith('refused')
                              ? 'bg-chip-red text-red-text border-red'
                              : 'bg-surface-2 text-muted border-border'
                          }`}
                        >
                          {log.outcome}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </Card>
          </div>
        </div>
      )}

      {/* Subtab 3: Window & Surface */}
      {subtab === 'appearance' && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <Card title="Window & Surface Architecture" subtitle="Locked opaque Linear geometry and motion preferences">
            <div className="space-y-4 text-xs">
              <div className="flex items-center justify-between py-1">
                <div>
                  <div className="font-medium text-[var(--text)]">Active Interface Theme</div>
                  <div className="text-[var(--text-muted)] text-[11px]">Persisted to localStorage (ui-theme) · Shortcut Ctrl+D</div>
                </div>
                <div className="flex items-center gap-2">
                  <span className="rounded border border-[var(--border)] bg-[var(--bg-surface-2)] px-2.5 py-1 text-[12px] font-medium text-[var(--text)]">
                    {theme === 'dark' ? 'Dark' : 'Light'}
                  </span>
                  {onToggleTheme && (
                    <Button variant="secondary" size="xs" onClick={onToggleTheme}>
                      Toggle
                    </Button>
                  )}
                </div>
              </div>

              <div className="flex items-center justify-between py-1 border-t border-[var(--border)]">
                <div>
                  <div className="font-medium text-[var(--text)]">Reduce Motion</div>
                  <div className="text-[var(--text-muted)] text-[11px]">Disables UI transitions (Ctrl+Shift+M)</div>
                </div>
                <Toggle checked={reduceMotion} onChange={onToggleReduceMotion} label="" />
              </div>

              <div className="p-3 rounded-lg bg-surface-2 border border-border space-y-1.5 font-mono text-[11px]">
                <div className="flex items-center justify-between">
                  <span className="text-muted">Desktop Session:</span>
                  <span className="text-primary-text font-medium">
                    {glass?.desktop || 'Linux'} ({glass?.session || 'X11/Wayland'})
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted">Window Bounds:</span>
                  <span className="text-fg font-medium">1280×800 default · 1024×640 min</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted">Surface Policy:</span>
                  <span className="text-fg font-medium">Opaque Solid Matte (Zero Blur)</span>
                </div>
                <div className="text-muted text-[10px] pt-1">
                  {glass?.detail || 'Opaque solid Linear window surface (compositor blur disabled).'}
                </div>
              </div>
            </div>
          </Card>

          <Card title="Storage, Autosave & Panel Recovery" subtitle="Local XDG directories and floating layout reset">
            <div className="space-y-3 text-xs font-mono">
              <div>
                <span className="text-[var(--color-fg-muted)] block text-[10px]">Active Project</span>
                <span className="text-[var(--color-fg)]">
                  {summary?.name || 'Voice Studio Demo'} (Revision #{summary?.revision || 1})
                </span>
              </div>
              <div>
                <span className="text-[var(--color-fg-muted)] block text-[10px]">Projects & SampleBlocks</span>
                <span className="text-[var(--color-fg-muted)] break-all">
                  ~/.local/share/audio-labs-pro/projects/default/
                </span>
              </div>
              <div>
                <span className="text-[var(--color-fg-muted)] block text-[10px]">Model Cache</span>
                <span className="text-[var(--color-fg-muted)] break-all">~/.local/share/alp/models/</span>
              </div>
              <div className="pt-3 border-t border-[var(--color-border)] flex items-center justify-between">
                <span className="text-[var(--color-fg-muted)]">Autosave: Active (debounced 750ms)</span>
                {onResetLayout && (
                  <Button
                    variant="secondary"
                    size="xs"
                    onClick={onResetLayout}
                    leftIcon={<LayoutGrid className="w-3.5 h-3.5" />}
                  >
                    Reset Floating Panels
                  </Button>
                )}
              </div>
            </div>
          </Card>
        </div>
      )}

      {/* Subtab 4: Shortcuts Parity */}
      {subtab === 'shortcuts' && (
        <div className="space-y-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <input
              type="text"
              value={shortcutFilter}
              onChange={(e) => setShortcutFilter(e.target.value)}
              placeholder="Filter keybindings by action or key (e.g. Ctrl+Z, Split, Mute)..."
              className="w-full max-w-md px-3 py-2 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-xs text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)]"
            />
            {onOpenShortcuts && (
              <Button variant="secondary" size="sm" onClick={onOpenShortcuts} leftIcon={<Keyboard className="w-4 h-4" />}>
                Open Quick Cheatsheet (?)
              </Button>
            )}
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {filteredGroups.map((group) => (
              <Card key={group.title} title={group.title} subtitle={`${group.items.length} keybinding(s)`}>
                <div className="space-y-2 text-xs">
                  {group.items.map((item) => (
                    <div
                      key={item.keys}
                      className="p-2.5 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] flex items-center justify-between gap-2"
                    >
                      <span className="text-[var(--text)]">{item.desc}</span>
                      <kbd className="shrink-0 rounded border border-[var(--border-strong)] bg-[var(--bg-surface-2)] px-1.5 py-0.5 font-mono text-[11px] text-primary-text">
                        {item.keys}
                      </kbd>
                    </div>
                  ))}
                </div>
              </Card>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
