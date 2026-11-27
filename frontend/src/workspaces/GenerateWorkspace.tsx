/**
 * Audio Labs Pro - Speech Generation & Batch Takes Workspace
 *
 * - `tts`: Real TTS synthesis prompt box + take history backed by `desktopTtsSynthesize`
 * - `batch`: Multi-item batch TTS synthesis queue rendering real WAV files
 */

import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Slider } from '../kit/Slider';
import { desktopTtsSynthesize, getVoiceProfiles } from '../ipc/client';
import type { VoiceProfile } from '../ipc/types';
import {
  Sparkles,
  Play,
  RotateCcw,
  Volume2,
  Clock,
  Plus,
  Layers,
  Trash2,
} from 'lucide-react';

export interface TakeItem {
  id: string;
  voiceName: string;
  text: string;
  duration: number;
  timestamp: string;
  speed: number;
  path?: string;
  engine?: string;
}

interface BatchItem {
  id: string;
  voiceId: string;
  text: string;
  status: 'pending' | 'done' | 'error';
  outputPath?: string;
}

export interface GenerateWorkspaceProps {
  subtab?: string;
}

export function GenerateWorkspace({ subtab = 'tts' }: GenerateWorkspaceProps) {
  const [profiles, setProfiles] = useState<VoiceProfile[]>([]);
  const [selectedVoiceId, setSelectedVoiceId] = useState<string>('');
  const [promptText, setPromptText] = useState<string>(
    'Welcome to Audio Labs Pro, the local speech studio engineered for Linux workstations.'
  );
  const [speed, setSpeed] = useState<number>(1.0);
  const [pitch, setPitch] = useState<number>(0.0);
  const [isRendering, setIsRendering] = useState<boolean>(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [takes, setTakes] = useState<TakeItem[]>([]);
  const [playingTakeId, setPlayingTakeId] = useState<string | null>(null);

  // Batch Takes Queue state
  const [batchQueue, setBatchQueue] = useState<BatchItem[]>([
    {
      id: 'b-1',
      voiceId: 'builtin-narrator',
      text: 'Chapter one opens in a quiet local studio.',
      status: 'pending',
    },
    {
      id: 'b-2',
      voiceId: 'builtin-narrator',
      text: 'All audio synthesis stays strictly offline.',
      status: 'pending',
    },
  ]);
  const [batchText, setBatchText] = useState('');
  const [batchRunning, setBatchRunning] = useState(false);

  useEffect(() => {
    getVoiceProfiles().then((profs) => {
      setProfiles(profs);
      if (profs.length > 0) {
        setSelectedVoiceId(profs[0].id);
        setBatchQueue((prev) =>
          prev.map((item) =>
            profs.some((p) => p.id === item.voiceId) ? item : { ...item, voiceId: profs[0].id }
          )
        );
      }
    });
  }, []);

  const handleRender = async () => {
    if (!promptText.trim()) return;
    setIsRendering(true);
    setNotice(null);

    const voice = profiles.find((p) => p.id === selectedVoiceId) || profiles[0];
    const voiceId = voice?.id || 'builtin-narrator';
    try {
      const report = await desktopTtsSynthesize(voiceId, promptText.trim(), speed);
      const newTake: TakeItem = {
        id: `take-${Date.now()}`,
        voiceName: voice?.name || voiceId,
        text: promptText.trim(),
        duration: +report.duration_secs.toFixed(2),
        timestamp: new Date().toLocaleTimeString(),
        speed,
        path: report.path,
        engine: report.engine,
      };
      setTakes((prev) => [newTake, ...prev]);
      setNotice(`Generated ${report.duration_secs.toFixed(2)}s WAV (${report.engine}) -> ${report.path}`);
    } catch (error) {
      setNotice(`Synthesis failed: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsRendering(false);
    }
  };

  const handleRunBatch = async () => {
    if (batchQueue.length === 0) return;
    setBatchRunning(true);
    setNotice(null);
    for (const item of batchQueue) {
      try {
        const res = await desktopTtsSynthesize(item.voiceId, item.text, speed);
        setBatchQueue((prev) =>
          prev.map((q) =>
            q.id === item.id ? { ...q, status: 'done', outputPath: res.path } : q
          )
        );
      } catch {
        setBatchQueue((prev) =>
          prev.map((q) => (q.id === item.id ? { ...q, status: 'error' } : q))
        );
      }
    }
    setBatchRunning(false);
    setNotice(`Batch completed for ${batchQueue.length} cue(s).`);
  };

  if (subtab === 'batch') {
    return (
      <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
              <Layers className="w-4 h-4 text-[var(--color-accent)]" />
              Batch Speech Takes Queue
            </h2>
            <p className="text-xs text-[var(--color-fg-muted)] mt-1">
              Queue multiple dialogue prompts across personas and synthesize each cue cleanly into project WAV files.
            </p>
          </div>
          <Button
            variant="primary"
            size="sm"
            onClick={handleRunBatch}
            disabled={batchRunning || batchQueue.length === 0}
            leftIcon={<Sparkles className="w-4 h-4" />}
          >
            {batchRunning ? 'Rendering Batch...' : `Render ${batchQueue.length} Cue(s)`}
          </Button>
        </div>

        {notice && (
          <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-strong)] text-xs font-mono text-sky-text">
            {notice}
          </div>
        )}

        <Card title="Batch Cue Queue" subtitle={`${batchQueue.length} scripted line(s)`}>
          <div className="space-y-3 text-xs">
            <div className="space-y-2">
              {batchQueue.map((item, idx) => (
                <div
                  key={item.id}
                  className="p-3 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] flex items-center justify-between gap-3"
                >
                  <div className="space-y-1 min-w-0">
                    <div className="flex items-center gap-2 font-mono text-[11px]">
                      <span className="text-muted">#{idx + 1}</span>
                      <span className="text-primary-text font-medium">{item.voiceId}</span>
                      <span
                        className={`px-1.5 py-0.5 rounded border text-[10px] ${
                          item.status === 'done'
                            ? 'bg-chip-sky text-sky-text border-border-strong'
                            : item.status === 'error'
                            ? 'bg-chip-red text-red-text border-red'
                            : 'bg-surface-2 text-muted border-border'
                        }`}
                      >
                        {item.status}
                      </span>
                    </div>
                    <div className="text-fg truncate">{item.text}</div>
                    {item.outputPath && (
                      <div className="font-mono text-[10px] text-muted truncate">{item.outputPath}</div>
                    )}
                  </div>
                  <button
                    type="button"
                    onClick={() => setBatchQueue((prev) => prev.filter((q) => q.id !== item.id))}
                    className="text-red-text p-1 cursor-pointer"
                    title="Remove line"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              ))}
            </div>

            <div className="pt-3 border-t border-[var(--color-border)] flex flex-wrap gap-2">
              <select
                value={selectedVoiceId}
                onChange={(e) => setSelectedVoiceId(e.target.value)}
                className="px-3 py-1.5 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-xs font-mono text-fg"
              >
                {profiles.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
              <input
                type="text"
                value={batchText}
                onChange={(e) => setBatchText(e.target.value)}
                placeholder="Enter line to enqueue..."
                className="flex-1 min-w-[240px] px-3 py-1.5 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-xs text-fg"
              />
              <Button
                variant="secondary"
                size="sm"
                leftIcon={<Plus className="w-4 h-4" />}
                onClick={() => {
                  if (!batchText.trim()) return;
                  setBatchQueue((prev) => [
                    ...prev,
                    {
                      id: `b-${Date.now()}`,
                      voiceId: selectedVoiceId || profiles[0]?.id || 'builtin-narrator',
                      text: batchText.trim(),
                      status: 'pending',
                    },
                  ]);
                  setBatchText('');
                }}
              >
                Queue Line
              </Button>
            </div>
          </div>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      <div>
        <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
          <Sparkles className="w-4 h-4 text-[var(--color-accent)]" />
          Speech Generation &amp; Take History
        </h2>
        <p className="text-xs text-[var(--color-fg-muted)] mt-1">
          Synthesize spoken narration using <code>alp-infer</code> (Kokoro/Kitten) or the built-in acoustic fallback.
        </p>
      </div>

      {notice && (
        <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-strong)] text-xs font-mono text-sky-text">
          {notice}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        <div className="lg:col-span-7 space-y-4">
          <Card title="Speech Prompt" subtitle="Neural text-to-speech synthesis">
            <div className="space-y-4 text-xs">
              <div>
                <label className="block text-[var(--color-fg-muted)] mb-1.5 font-medium">
                  Active Voice Persona
                </label>
                <select
                  value={selectedVoiceId}
                  onChange={(e) => setSelectedVoiceId(e.target.value)}
                  className="w-full px-3 py-2 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)] font-mono text-xs"
                >
                  {profiles.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name} ({p.kind} · {p.language})
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-[var(--color-fg-muted)] mb-1.5 font-medium">
                  Text to Synthesize
                </label>
                <textarea
                  rows={5}
                  value={promptText}
                  onChange={(e) => setPromptText(e.target.value)}
                  placeholder="Enter the narration script or speech dialogue..."
                  className="w-full px-3 py-2.5 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)] font-sans text-xs resize-none leading-relaxed"
                />
                <div className="flex justify-between items-center text-[11px] text-[var(--color-fg-muted)] mt-1 font-mono">
                  <span>
                    {promptText.length} characters · {promptText.split(/\s+/).filter(Boolean).length} words
                  </span>
                  <span>Est. duration: ~{(promptText.split(/\s+/).filter(Boolean).length * 0.35).toFixed(1)}s</span>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4 pt-2">
                <Slider
                  label="Speed / Cadence"
                  min={0.5}
                  max={1.5}
                  step={0.05}
                  value={speed}
                  onChange={setSpeed}
                  formatValue={(v) => `${v.toFixed(2)}x`}
                />
                <Slider
                  label="Pitch Offset"
                  min={-6}
                  max={6}
                  step={0.5}
                  value={pitch}
                  onChange={setPitch}
                  unit="st"
                />
              </div>

              <div className="pt-3 border-t border-[var(--color-border)] flex items-center justify-between">
                <span className="text-[11px] font-mono text-[var(--color-fg-muted)]">
                  Engine: Daemon-first Kokoro-82M / Built-in fallback
                </span>
                <Button
                  variant="primary"
                  size="sm"
                  onClick={handleRender}
                  disabled={isRendering || !promptText.trim()}
                  leftIcon={
                    isRendering ? <RotateCcw className="w-4 h-4 animate-spin" /> : <Sparkles className="w-4 h-4" />
                  }
                >
                  {isRendering ? 'Synthesizing...' : 'Render Speech Take'}
                </Button>
              </div>
            </div>
          </Card>
        </div>

        <div className="lg:col-span-5 space-y-4">
          <Card title="Take History" subtitle={`${takes.length} rendered WAV take(s) this session`}>
            {takes.length === 0 ? (
              <div className="p-6 text-center text-xs text-muted italic">
                Render a prompt on the left to generate a real WAV take.
              </div>
            ) : (
              <div className="space-y-3 max-h-[500px] overflow-y-auto pr-1 text-xs">
                {takes.map((take) => (
                  <div
                    key={take.id}
                    className="p-3 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] space-y-2"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-[var(--color-fg)]">{take.voiceName}</span>
                        <span className="text-[10px] font-mono text-[var(--color-fg-muted)] flex items-center gap-1">
                          <Clock className="w-3.5 h-3.5" />
                          {take.timestamp}
                        </span>
                      </div>
                      <Button
                        variant={playingTakeId === take.id ? 'primary' : 'ghost'}
                        size="xs"
                        onClick={() => {
                          setPlayingTakeId(take.id);
                          setTimeout(() => setPlayingTakeId(null), 1200);
                        }}
                        aria-label="Inspect take"
                      >
                        {playingTakeId === take.id ? <Volume2 className="w-4 h-4" /> : <Play className="w-4 h-4" />}
                      </Button>
                    </div>
                    <p className="text-[var(--color-fg-muted)] text-xs italic">&ldquo;{take.text}&rdquo;</p>
                    <div className="pt-2 border-t border-[var(--color-border)] text-[10px] font-mono text-muted space-y-0.5">
                      <div>
                        Length: {take.duration}s · {take.speed}x · {take.engine || 'acoustic'}
                      </div>
                      {take.path && <div className="truncate">{take.path}</div>}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </Card>
        </div>
      </div>
    </div>
  );
}
