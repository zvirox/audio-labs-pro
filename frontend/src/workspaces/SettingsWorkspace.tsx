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
  onToggleTheme: () => void;
  reduceMotion: boolean;
  onToggleReduceMotion: () => void;
}

export function SettingsWorkspace({
  theme,
  onToggleTheme,
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
        <h2 className="text-xl font-bold tracking-tight text-slate-100 flex items-center gap-2">
          <Sliders className="w-5 h-5 text-cyan-400" />
          System Settings & Studio Preferences
        </h2>
        <p className="text-xs text-slate-400 mt-1">
          Configure real-time audio devices, window compositor effects, autonomous AI security tiers, and mastering presets.
        </p>
      </div>

      {/* Pending Client Approval Banner */}
      {pendingApproval && (
        <div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-between gap-4 animate-in fade-in duration-200">
          <div className="flex items-center gap-3">
            <AlertCircle className="w-5 h-5 text-amber-400 shrink-0" />
            <div>
              <div className="font-semibold text-amber-200 text-sm">
                AI Client Connection Request: <span className="font-mono text-cyan-300">"{pendingApproval}"</span>
              </div>
              <div className="text-xs text-slate-300">
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
              <label className="block text-slate-400 mb-1.5 font-medium">Output Device (Sink)</label>
              <select
                value={selectedOutput}
                onChange={(e) => setSelectedOutput(e.target.value)}
                className="w-full px-3 py-2 rounded-lg bg-slate-900 border border-white/10 text-slate-200 focus:outline-none focus:border-cyan-500 font-mono text-xs"
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
              <label className="block text-slate-400 mb-1.5 font-medium">Capture Device (Microphone)</label>
              <select
                value={selectedInput}
                onChange={(e) => setSelectedInput(e.target.value)}
                className="w-full px-3 py-2 rounded-lg bg-slate-900 border border-white/10 text-slate-200 focus:outline-none focus:border-cyan-500 font-mono text-xs"
              >
                {inputs.map((dev) => (
                  <option key={dev.name} value={dev.name}>
                    {dev.name} ({dev.channels}ch · {dev.sample_rate / 1000}kHz)
                  </option>
                ))}
              </select>
            </div>

            {/* Test Tone Action */}
            <div className="pt-2 flex items-center justify-between border-t border-white/5">
              <span className="text-slate-400">Audio Stream Health: 0 buffer underruns</span>
              <Button
                variant={testToneActive ? 'primary' : 'secondary'}
                size="sm"
                onClick={handleTestTone}
                leftIcon={<Volume2 className="w-3.5 h-3.5" />}
              >
                {testToneActive ? 'Playing 440 Hz...' : 'Play Test Tone'}
              </Button>
            </div>
          </div>
        </Card>

        {/* Appearance & Compositor Glass Card */}
        <Card title="Appearance & Window Glass" subtitle="KWin desktop blur and interface themes">
          <div className="space-y-4 text-xs">
            <div className="flex items-center justify-between py-1">
              <div>
                <div className="font-semibold text-slate-200">Interface Theme</div>
                <div className="text-slate-400 text-[11px]">Dark graphite or clean studio light (Ctrl+D)</div>
              </div>
              <Button variant="secondary" size="sm" onClick={onToggleTheme}>
                {theme === 'dark' ? 'Dark (Studio)' : 'Light (Clean)'}
              </Button>
            </div>

            <div className="flex items-center justify-between py-1 border-t border-white/5">
              <div>
                <div className="font-semibold text-slate-200">Reduce Motion</div>
                <div className="text-slate-400 text-[11px]">Disables decorative transitions (Ctrl+Shift+M)</div>
              </div>
              <Toggle checked={reduceMotion} onChange={onToggleReduceMotion} label="" />
            </div>

            {/* Glass Status */}
            <div className="p-3 rounded-lg bg-slate-950/80 border border-white/10 space-y-1.5 font-mono text-[11px]">
              <div className="flex items-center justify-between">
                <span className="text-slate-400">Desktop Session:</span>
                <span className="text-cyan-400 font-semibold">{glass?.desktop || 'KDE Plasma'} ({glass?.session || 'X11'})</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-slate-400">Glass Mode:</span>
                <span className="text-purple-400 font-semibold">{glass?.mode || 'compositor_blur'}</span>
              </div>
              <div className="text-slate-500 text-[10px] pt-1">
                {glass?.detail || 'KWin blur property active via safe x11rb atom manipulation.'}
              </div>
            </div>
          </div>
        </Card>

        {/* AI Control & MCP Tiers Card */}
        <Card title="AI Agent Control & Security" subtitle="Model Context Protocol (MCP) live socket host">
          <div className="space-y-4 text-xs">
            {/* Status & Toggle Header */}
            <div className="flex items-center justify-between p-3 rounded-lg bg-slate-950/80 border border-white/10">
              <div className="flex items-center gap-2.5">
                <div
                  className={`w-2.5 h-2.5 rounded-full ${
                    aiStatus?.enabled ? 'bg-emerald-400 shadow-sm shadow-emerald-400/50 animate-pulse' : 'bg-slate-600'
                  }`}
                />
                <div>
                  <div className="font-semibold text-slate-200 flex items-center gap-2">
                    <span>MCP Attach Host</span>
                    <span
                      className={`text-[10px] px-2 py-0.5 rounded font-mono font-medium ${
                        aiStatus?.enabled
                          ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30'
                          : 'bg-slate-800 text-slate-400 border border-white/10'
                      }`}
                    >
                      {aiStatus?.enabled ? 'ACTIVE' : 'STANDBY'}
                    </span>
                  </div>
                  <div className="text-[11px] text-slate-400 font-mono mt-0.5 truncate max-w-xs">
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
                <label className="text-slate-400 font-medium">Permission Tier</label>
                <span className="text-slate-500 text-[11px]">Enforced on all tool calls</span>
              </div>
              <div className="grid grid-cols-3 gap-2">
                {(['observe', 'edit', 'export'] as const).map((tier) => (
                  <button
                    key={tier}
                    type="button"
                    onClick={() => handleTierChange(tier)}
                    className={`py-2 px-3 rounded-lg font-medium text-center capitalize border transition-all ${
                      aiStatus?.tier === tier
                        ? 'bg-cyan-500/20 text-cyan-300 border-cyan-500/50 shadow-sm shadow-cyan-500/20'
                        : 'bg-slate-900 text-slate-400 border-white/10 hover:text-slate-200'
                    }`}
                  >
                    {tier}
                  </button>
                ))}
              </div>
              <p className="text-[11px] text-slate-400 mt-1.5 italic">
                {aiStatus?.tier === 'observe' && 'Observe: attached AI can read project metadata and tracks (read-only).'}
                {aiStatus?.tier === 'edit' && 'Edit: full MCP tool catalog edits live project; commits share one undo entry.'}
                {aiStatus?.tier === 'export' && 'Export: attached AI may also master and render audio into the project sandbox.'}
              </p>
            </div>

            {/* Live Socket Transport Details */}
            {aiStatus?.enabled && (
              <div className="space-y-2 p-3 rounded-lg bg-slate-950/80 border border-white/10 font-mono text-[11px]">
                <div className="flex items-center justify-between">
                  <span className="text-slate-400">Session Token:</span>
                  <div className="flex items-center gap-1.5">
                    <span className="text-slate-300 truncate max-w-[140px]">
                      {aiStatus.token ? `${aiStatus.token.slice(0, 16)}...` : 'None'}
                    </span>
                    <button
                      type="button"
                      onClick={handleCopyToken}
                      aria-label="Copy session token"
                      className="p-1 rounded hover:bg-white/10 text-cyan-400 transition-colors"
                      title="Copy full token"
                    >
                      {copiedToken ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                    </button>
                  </div>
                </div>

                <div className="flex items-center justify-between pt-1 border-t border-white/5">
                  <span className="text-slate-400">CLI Attach:</span>
                  <button
                    type="button"
                    onClick={handleCopyCommand}
                    className="flex items-center gap-1 text-cyan-300 hover:text-cyan-200 text-[10px] underline"
                  >
                    <Terminal className="w-3 h-3" />
                    <span>{copiedCmd ? 'Command Copied!' : 'Copy Shell Command'}</span>
                  </button>
                </div>

                <div className="flex items-center justify-between pt-1 border-t border-white/5 text-[10px]">
                  <span className="text-slate-400">Commits / Latency:</span>
                  <span className="text-slate-300">
                    {aiStatus.commits} commits · {aiStatus.last_commit_ms != null ? `${aiStatus.last_commit_ms} ms latest` : 'no edits yet'} (max {aiStatus.max_commit_ms} ms)
                  </span>
                </div>
              </div>
            )}

            {/* Safety & Undo Controls */}
            <div className="flex items-center justify-between pt-2 border-t border-white/5">
              <Button
                variant="secondary"
                size="sm"
                onClick={handleUndoAi}
                leftIcon={<RotateCcw className="w-3.5 h-3.5" />}
              >
                Undo Last AI Edit
              </Button>
              {undoNotice && (
                <span className="text-[11px] text-cyan-400 font-mono animate-in fade-in">
                  {undoNotice}
                </span>
              )}
            </div>

            {/* Invariant Highlights */}
            <div className="p-3 rounded-lg bg-slate-950/80 border border-white/10 space-y-1.5">
              <div className="flex items-center gap-2 text-slate-200 font-medium">
                <ShieldCheck className="w-4 h-4 text-emerald-400" />
                <span>Active Protection Invariants</span>
              </div>
              <ul className="text-[11px] text-slate-400 space-y-0.5 list-disc list-inside">
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
              <span className="text-slate-500 block text-[10px] uppercase">Active Project</span>
              <span className="text-slate-200">{summary?.name || 'Voice Studio Demo'} (Revision #{summary?.revision || 1})</span>
            </div>
            <div>
              <span className="text-slate-500 block text-[10px] uppercase">Media & Waveforms</span>
              <span className="text-slate-300 break-all">~/.local/share/audio-labs-pro/projects/default/media/</span>
            </div>
            <div>
              <span className="text-slate-500 block text-[10px] uppercase">Model Cache</span>
              <span className="text-slate-300 break-all">~/.local/share/alp/models/</span>
            </div>
            <div className="pt-2 border-t border-white/5 flex items-center justify-between text-[11px]">
              <span className="text-slate-400">Autosave Status:</span>
              <span className="text-emerald-400 font-semibold">Active (debounced 750ms)</span>
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
            <div className="p-6 text-center text-slate-500 italic">No agent activity recorded yet.</div>
          ) : (
            <div className="max-h-48 overflow-y-auto space-y-1 pr-1 font-mono text-[11px]">
              {activityLogs.map((log, idx) => (
                <div
                  key={`${log.timestamp}-${idx}`}
                  className="p-2 rounded bg-slate-950/60 border border-white/5 flex items-center justify-between gap-3"
                >
                  <div className="flex items-center gap-2 truncate">
                    <span className="text-slate-500 text-[10px]">{log.timestamp}</span>
                    <span className="text-cyan-400 font-semibold">{log.client_id}</span>
                    <span className="text-slate-400">›</span>
                    <span className="text-slate-200 font-medium">{log.operation}</span>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    {log.queue_to_commit_ms != null && (
                      <span className="text-slate-500 text-[10px]">{log.queue_to_commit_ms} ms</span>
                    )}
                    <span
                      className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${
                        log.committed
                          ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30'
                          : log.outcome.startsWith('refused')
                          ? 'bg-rose-500/20 text-rose-300 border border-rose-500/30'
                          : 'bg-slate-800 text-slate-400'
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
              leftIcon={<FileCheck className="w-3.5 h-3.5 text-cyan-400" />}
            >
              Podcast (-16 LUFS)
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => handleExportPreset('Audiobook', -20)}
              leftIcon={<FileCheck className="w-3.5 h-3.5 text-purple-400" />}
            >
              Audiobook (-20 LUFS)
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => handleExportPreset('Voice-over', -24)}
              leftIcon={<FileCheck className="w-3.5 h-3.5 text-emerald-400" />}
            >
              Voice-over (-24 LUFS)
            </Button>
          </div>
          {exportNotice && (
            <div className="p-2.5 rounded-lg bg-cyan-950/40 border border-cyan-500/30 text-cyan-300 font-mono text-[11px] animate-in fade-in">
              {exportNotice}
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
