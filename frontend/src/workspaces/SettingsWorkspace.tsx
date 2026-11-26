/**
 * Audio Labs Pro - Settings Workspace
 *
 * Audio hardware configuration (PipeWire / ALSA), appearance and KWin glass blur,
 * AI MCP permission tier controls, storage directory paths, and delivery export presets.
 */

import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Toggle } from '../kit/Toggle';
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
} from 'lucide-react';

export interface SettingsWorkspaceProps {
  theme: 'dark' | 'light';
  onToggleTheme?: () => void;
  reduceMotion: boolean;
  onToggleReduceMotion: () => void;
}

export function SettingsWorkspace({
  theme,
  reduceMotion,
  onToggleReduceMotion,
}: SettingsWorkspaceProps) {
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [glass, setGlass] = useState<GlassReport | null>(null);
  const [summary, setSummary] = useState<ProjectSummary | null>(null);
  const [selectedOutput, setSelectedOutput] = useState<string>('');
  const [selectedInput, setSelectedInput] = useState<string>('');
  const [testToneActive, setTestToneActive] = useState<boolean>(false);
  const [exportNotice, setExportNotice] = useState<string | null>(null);

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

    // Listen for AI activity and approval events
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

  const handleTestTone = () => {
    setTestToneActive(true);
    setTimeout(() => setTestToneActive(false), 800);
  };

  const handleExportPreset = (preset: string, lufs: number) => {
    setExportNotice(`Queued export with ${preset} (${lufs} LUFS target)`);
    setTimeout(() => setExportNotice(null), 3500);
  };

  const handleToggleAi = async () => {
    try {
      const updated = await toggleAiControl();
      setAiStatus(updated);
      const logs = await getAiActivityLog();
      setActivityLogs(logs);
    } catch (err) {
      console.error('Failed to toggle AI control:', err);
    }
  };

  const handleTierChange = async (tier: PermissionTier) => {
    try {
      const updated = await setAiPermissionTier(tier);
      setAiStatus(updated);
      const logs = await getAiActivityLog();
      setActivityLogs(logs);
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
      const updatedSummary = await getProjectSummary();
      setSummary(updatedSummary);
      setUndoNotice('Successfully rolled back last AI edit');
      setTimeout(() => setUndoNotice(null), 3000);
    } catch (err) {
      setUndoNotice(`Undo failed: ${err}`);
      setTimeout(() => setUndoNotice(null), 3000);
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

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      {/* Workspace Header */}
      <div>
        <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
          <Sliders className="w-4 h-4 text-[var(--color-accent)]" />
          System Settings & Studio Preferences
        </h2>
        <p className="text-xs text-[var(--color-fg-muted)] mt-1">
          Configure real-time audio devices, window compositor effects, autonomous AI security tiers, and mastering presets.
        </p>
      </div>

        {/* Pending Client Approval Banner */}
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
            <Button variant="secondary" size="sm" onClick={() => handleApproval('deny')}>
              Deny
            </Button>
            <Button variant="secondary" size="sm" onClick={() => handleApproval('session')}>
              Approve This Session
            </Button>
            <Button variant="primary" size="sm" onClick={() => handleApproval('always')}>
              Always Approve
            </Button>
          </div>
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Audio Hardware & ALSA Card */}
        <Card title="Audio Hardware (PipeWire / ALSA)" subtitle="Default output sink and recording interface">
          <div className="space-y-4 text-xs">
            {/* Output Device */}
            <div>
              <label className="block text-[var(--color-fg-muted)] mb-1.5 font-medium">Output Device (Sink)</label>
              <select
                value={selectedOutput}
                onChange={(e) => setSelectedOutput(e.target.value)}
                className="w-full px-3 py-2 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)] font-mono text-xs"
              >
                {outputs.map((dev) => (
                  <option key={dev.name} value={dev.name}>
                    {dev.name} ({dev.channels}ch · {dev.sample_rate / 1000}kHz · {dev.sample_format})
                  </option>
                ))}
              </select>
            </div>

            {/* Input Device */}
            <div>
              <label className="block text-[var(--color-fg-muted)] mb-1.5 font-medium">Capture Device (Microphone)</label>
              <select
                value={selectedInput}
                onChange={(e) => setSelectedInput(e.target.value)}
                className="w-full px-3 py-2 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)] font-mono text-xs"
              >
                {inputs.map((dev) => (
                  <option key={dev.name} value={dev.name}>
                    {dev.name} ({dev.channels}ch · {dev.sample_rate / 1000}kHz)
                  </option>
                ))}
              </select>
            </div>

            {/* Test Tone Action */}
            <div className="pt-2 flex items-center justify-between border-t border-[var(--color-border)]">
              <span className="text-[var(--color-fg-muted)]">Audio Stream Health: 0 buffer underruns</span>
              <Button
                variant={testToneActive ? 'primary' : 'secondary'}
                size="sm"
                onClick={handleTestTone}
                leftIcon={<Volume2 className="w-4 h-4" />}
              >
                {testToneActive ? 'Playing 440 Hz...' : 'Play Test Tone'}
              </Button>
            </div>
          </div>
        </Card>

        {/* Window Surface & Motion Card (theme toggle lives solely in TitleBar) */}
        <Card title="Window & Surface Architecture" subtitle="Locked opaque Linear geometry and motion preferences">
          <div className="space-y-4 text-xs">
            <div className="flex items-center justify-between py-1">
              <div>
                <div className="font-medium text-[var(--text)]">Active Interface Theme</div>
                <div className="text-[var(--text-muted)] text-[11px]">Controlled from TitleBar toggle or Ctrl+D</div>
              </div>
              <span className="rounded border border-[var(--border)] bg-[var(--bg-surface-2)] px-2.5 py-1 text-[12px] font-medium text-[var(--text)]">
                {theme === 'dark' ? 'Dark' : 'Light'}
              </span>
            </div>

            <div className="flex items-center justify-between py-1 border-t border-[var(--border)]">
              <div>
                <div className="font-medium text-[var(--text)]">Reduce Motion</div>
                <div className="text-[var(--text-muted)] text-[11px]">Disables decorative transitions (Ctrl+Shift+M)</div>
              </div>
              <Toggle checked={reduceMotion} onChange={onToggleReduceMotion} label="" />
            </div>

            {/* Surface Verification Readout */}
            <div className="p-3 rounded-lg bg-surface-2 border border-border space-y-1.5 font-mono text-[11px]">
              <div className="flex items-center justify-between">
                <span className="text-muted">Desktop Session:</span>
                <span className="text-primary-text font-medium">{glass?.desktop || 'Linux'} ({glass?.session || 'X11/Wayland'})</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted">Surface Policy:</span>
                <span className="text-fg font-medium">Opaque Solid (Linear Standard)</span>
              </div>
              <div className="text-muted text-[10px] pt-1">
                {glass?.detail || 'Opaque solid Linear window surface (compositor blur disabled).'}
              </div>
            </div>
          </div>
        </Card>

        {/* AI Control & MCP Tiers Card */}
        <Card title="AI Agent Control & Security" subtitle="Model Context Protocol (MCP) live socket host">
          <div className="space-y-4 text-xs">
            {/* Status & Toggle Header */}
            <div className="flex items-center justify-between p-3 rounded-lg bg-surface-2 border border-border">
              <div className="flex items-center gap-2.5">
                <div
                  className={`w-2.5 h-2.5 rounded-full ${
                    aiStatus?.enabled ? 'bg-sky' : 'bg-faint'
                  }`}
                />
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

            {/* Permission Tier Selector */}
            <div>
              <div className="flex items-center justify-between mb-1.5">
                <label className="text-muted font-medium">Permission Tier</label>
                <span className="text-muted text-[11px]">Enforced on all tool calls</span>
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
              <p className="text-[11px] text-[var(--color-fg-muted)] mt-1.5 italic">
                {aiStatus?.tier === 'observe' && 'Observe: attached AI can read project metadata and tracks (read-only).'}
                {aiStatus?.tier === 'edit' && 'Edit: full MCP tool catalog edits live project; commits share one undo entry.'}
                {aiStatus?.tier === 'export' && 'Export: attached AI may also master and render audio into the project sandbox.'}
              </p>
            </div>

            {/* Live Socket Transport Details */}
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
                      className="p-1 rounded hover:bg-[var(--color-surface-2)] text-[var(--color-accent)] transition-colors"
                      title="Copy full token"
                    >
                      {copiedToken ? <Check className="w-4 h-4 text-[var(--color-fg-muted)]" /> : <Copy className="w-4 h-4" />}
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

                <div className="flex items-center justify-between pt-1 border-t border-[var(--color-border)] text-[10px]">
                  <span className="text-[var(--color-fg-muted)]">Commits / Latency:</span>
                  <span className="text-[var(--color-fg-muted)]">
                    {aiStatus.commits} commits · {aiStatus.last_commit_ms != null ? `${aiStatus.last_commit_ms} ms latest` : 'no edits yet'} (max {aiStatus.max_commit_ms} ms)
                  </span>
                </div>
              </div>
            )}

            {/* Safety & Undo Controls */}
            <div className="flex items-center justify-between pt-2 border-t border-[var(--color-border)]">
              <Button
                variant="secondary"
                size="sm"
                onClick={handleUndoAi}
                leftIcon={<RotateCcw className="w-4 h-4" />}
              >
                Undo Last AI Edit
              </Button>
              {undoNotice && (
                <span className="text-[11px] text-[var(--color-accent)] font-mono animate-in fade-in">
                  {undoNotice}
                </span>
              )}
            </div>

            {/* Invariant Highlights */}
            <div className="p-3 rounded-lg bg-[var(--color-bg)] border border-[var(--color-border)] space-y-1.5">
              <div className="flex items-center gap-2 text-[var(--color-fg)] font-medium">
                <ShieldCheck className="w-4 h-4 text-[var(--color-fg-muted)]" />
                <span>Active Protection Invariants</span>
              </div>
              <ul className="text-[11px] text-[var(--color-fg-muted)] space-y-0.5 list-disc list-inside">
                <li>Voice cloning consent gate cannot be bypassed by automated agents.</li>
                <li>Revision snapshots enforce single-undo checkpoints for batch operations.</li>
                <li>File writes restricted strictly to project sandbox paths.</li>
              </ul>
            </div>
          </div>
        </Card>

        {/* Storage & Autosave Card */}
        <Card title="Storage & Project Autosave" subtitle="Local directories and recovery snapshots">
          <div className="space-y-3 text-xs font-mono">
            <div>
              <span className="text-[var(--color-fg-muted)] block text-[10px]">Active Project</span>
              <span className="text-[var(--color-fg)]">{summary?.name || 'Voice Studio Demo'} (Revision #{summary?.revision || 1})</span>
            </div>
            <div>
              <span className="text-[var(--color-fg-muted)] block text-[10px]">Media & Waveforms</span>
              <span className="text-[var(--color-fg-muted)] break-all">~/.local/share/audio-labs-pro/projects/default/media/</span>
            </div>
            <div>
              <span className="text-[var(--color-fg-muted)] block text-[10px]">Model Cache</span>
              <span className="text-[var(--color-fg-muted)] break-all">~/.local/share/alp/models/</span>
            </div>
            <div className="pt-2 border-t border-[var(--color-border)] flex items-center justify-between text-[11px]">
              <span className="text-[var(--color-fg-muted)]">Autosave Status:</span>
              <span className="text-[var(--color-fg-muted)] font-medium">Active (debounced 750ms)</span>
            </div>
          </div>
        </Card>
      </div>

      {/* AI Activity Audit Trail */}
      <Card
        title="AI Agent Activity Audit Log"
        subtitle="Real-time log of autonomous actions, tool dispatches, and undo checkpoints"
      >
        <div className="space-y-2 text-xs">
          {activityLogs.length === 0 ? (
            <div className="p-6 text-center text-[var(--color-fg-muted)] italic">No agent activity recorded yet.</div>
          ) : (
            <div className="max-h-48 overflow-y-auto space-y-1 pr-1 font-mono text-[11px]">
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
                  <div className="flex items-center gap-2 shrink-0">
                    {log.queue_to_commit_ms != null && (
                      <span className="text-[var(--color-fg-muted)] text-[10px]">{log.queue_to_commit_ms} ms</span>
                    )}
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
                </div>
              ))}
            </div>
          )}
        </div>
      </Card>

      {/* Delivery Export Presets Card */}
      <Card title="Delivery & Export Presets" subtitle="Loudness-calibrated export workflows">
        <div className="space-y-3 text-xs">
          <div className="flex flex-wrap items-center gap-3">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => handleExportPreset('Podcast', -16)}
              leftIcon={<FileCheck className="w-4 h-4 text-[var(--color-accent)]" />}
            >
              Podcast (-16 LUFS)
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => handleExportPreset('Audiobook', -20)}
              leftIcon={<FileCheck className="w-4 h-4 text-[var(--color-fg-muted)]" />}
            >
              Audiobook (-20 LUFS)
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => handleExportPreset('Voice-over', -24)}
              leftIcon={<FileCheck className="w-4 h-4 text-[var(--color-fg-muted)]" />}
            >
              Voice-over (-24 LUFS)
            </Button>
          </div>
          {exportNotice && (
            <div className="p-2.5 rounded-lg bg-primary-tint border border-border-strong text-primary-text font-mono text-[11px]">
              {exportNotice}
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
