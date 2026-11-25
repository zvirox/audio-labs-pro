/**
 * Audio Labs Pro - Generate Workspace
 *
 * Neural speech synthesis prompt box, voice persona selection, real-time take rendering,
 * and generated audio history with timeline insertion.
 */

import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Slider } from '../kit/Slider';
import { getVoiceProfiles } from '../ipc/client';
import type { VoiceProfile } from '../ipc/types';
import {
  Sparkles,
  Play,
  RotateCcw,
  Volume2,
  Clock,
  ArrowRightCircle,
} from 'lucide-react';

export interface TakeItem {
  id: string;
  voiceName: string;
  text: string;
  duration: number;
  timestamp: string;
  speed: number;
}

export function GenerateWorkspace() {
  const [profiles, setProfiles] = useState<VoiceProfile[]>([]);
  const [selectedVoiceId, setSelectedVoiceId] = useState<string>('');
  const [promptText, setPromptText] = useState<string>(
    'Welcome to Audio Labs Pro, the local speech studio engineered for Linux workstations.'
  );
  const [speed, setSpeed] = useState<number>(1.0);
  const [pitch, setPitch] = useState<number>(0.0);
  const [isRendering, setIsRendering] = useState<boolean>(false);
  const [takes, setTakes] = useState<TakeItem[]>([
    {
      id: 'take-01',
      voiceName: 'Linus Sebastian',
      text: 'Welcome to Audio Labs Pro, the local speech studio engineered for Linux workstations.',
      duration: 5.4,
      timestamp: 'Just now',
      speed: 1.0,
    },
    {
      id: 'take-02',
      voiceName: 'Studio Narrator (Acoustic)',
      text: 'In this chapter, we explore how neural audio synthesis runs entirely offline on your machine.',
      duration: 7.2,
      timestamp: '5m ago',
      speed: 0.95,
    },
  ]);
  const [playingTakeId, setPlayingTakeId] = useState<string | null>(null);

  useEffect(() => {
    getVoiceProfiles().then((profs) => {
      setProfiles(profs);
      if (profs.length > 0) setSelectedVoiceId(profs[0].id);
    });
  }, []);

  const handleRender = () => {
    if (!promptText.trim()) return;
    setIsRendering(true);

    const voice = profiles.find((p) => p.id === selectedVoiceId) || profiles[0];

    setTimeout(() => {
      const newTake: TakeItem = {
        id: `take-${Date.now()}`,
        voiceName: voice?.name || 'Acoustic Voice',
        text: promptText,
        duration: Math.max(2.5, +(promptText.split(' ').length * 0.35).toFixed(1)),
        timestamp: 'Just now',
        speed,
      };

      setTakes((prev) => [newTake, ...prev]);
      setIsRendering(false);
    }, 900);
  };

  const handlePlayTake = (id: string) => {
    setPlayingTakeId(id);
    setTimeout(() => setPlayingTakeId(null), 2000);
  };

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      {/* Workspace Header */}
      <div>
        <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
          <Sparkles className="w-4 h-4 text-[var(--color-accent)]" />
          Speech Generation & Take History
        </h2>
        <p className="text-xs text-[var(--color-fg-muted)] mt-1">
          Compose spoken text, choose voice personas, synthesize high-fidelity takes, and route audio directly onto the multitrack timeline.
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left: Speech Prompt Box (7 cols) */}
        <div className="lg:col-span-7 space-y-4">
          <Card title="Speech Prompt" subtitle="Neural text-to-speech synthesis">
            <div className="space-y-4 text-xs">
              {/* Voice Selector */}
              <div>
                <label className="block text-[var(--color-fg-muted)] mb-1.5 font-medium">Active Voice Persona</label>
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

              {/* Text Input Area */}
              <div>
                <label className="block text-[var(--color-fg-muted)] mb-1.5 font-medium">Text to Synthesize</label>
                <textarea
                  rows={5}
                  value={promptText}
                  onChange={(e) => setPromptText(e.target.value)}
                  placeholder="Enter the narration script or speech dialogue..."
                  className="w-full px-3 py-2.5 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)] font-sans text-xs resize-none leading-relaxed"
                />
                <div className="flex justify-between items-center text-[11px] text-[var(--color-fg-muted)] mt-1 font-mono">
                  <span>{promptText.length} characters · {promptText.split(/\s+/).filter(Boolean).length} words</span>
                  <span>Est. duration: ~{(promptText.split(/\s+/).filter(Boolean).length * 0.35).toFixed(1)}s</span>
                </div>
              </div>

              {/* Quick Controls */}
              <div className="grid grid-cols-2 gap-4 pt-2">
                <div>
                  <Slider
                    label="Speed / Cadence"
                    min={0.5}
                    max={1.5}
                    step={0.05}
                    value={speed}
                    onChange={setSpeed}
                    formatValue={(v) => `${v.toFixed(2)}x`}
                  />
                </div>
                <div>
                  <Slider
                    label="Pitch Shift"
                    min={-6}
                    max={6}
                    step={0.5}
                    value={pitch}
                    onChange={setPitch}
                    unit="st"
                  />
                </div>
              </div>

              {/* Action Button */}
              <div className="pt-3 border-t border-[var(--color-border)] flex items-center justify-between">
                <span className="text-[11px] font-mono text-[var(--color-fg-muted)]">
                  Engine: Kokoro-82M (Sub-50ms)
                </span>
                <Button
                  variant="primary"
                  size="sm"
                  onClick={handleRender}
                  disabled={isRendering || !promptText.trim()}
                  leftIcon={
                    isRendering ? (
                      <RotateCcw className="w-4 h-4 animate-spin" />
                    ) : (
                      <Sparkles className="w-4 h-4" />
                    )
                  }
                >
                  {isRendering ? 'Synthesizing...' : 'Render Speech Take'}
                </Button>
              </div>
            </div>
          </Card>
        </div>

        {/* Right: Take History Feed (5 cols) */}
        <div className="lg:col-span-5 space-y-4">
          <Card title="Take History" subtitle={`${takes.length} generated takes in this session`}>
            <div className="space-y-3 max-h-[500px] overflow-y-auto pr-1 text-xs">
              {takes.map((take) => {
                const isPlaying = playingTakeId === take.id;

                return (
                  <div
                    key={take.id}
                    className="p-3 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] hover:border-[var(--border-strong)] transition-colors space-y-2"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-[var(--color-fg)]">{take.voiceName}</span>
                        <span className="text-[10px] font-mono text-[var(--color-fg-muted)] flex items-center gap-1">
                          <Clock className="w-4 h-4 text-[var(--color-fg-muted)]" />
                          {take.timestamp}
                        </span>
                      </div>

                      <Button
                        variant={isPlaying ? 'primary' : 'ghost'}
                        size="xs"
                        onClick={() => handlePlayTake(take.id)}
                        aria-label="Play take"
                      >
                        {isPlaying ? (
                          <Volume2 className="w-4 h-4 text-[var(--color-fg-muted)]" />
                        ) : (
                          <Play className="w-4 h-4" />
                        )}
                      </Button>
                    </div>

                    <p className="text-[var(--color-fg-muted)] text-xs italic line-clamp-2">
                      &ldquo;{take.text}&rdquo;
                    </p>

                    <div className="pt-2 border-t border-[var(--color-border)] flex items-center justify-between text-[11px] font-mono text-[var(--color-fg-muted)]">
                      <span>Length: {take.duration}s · {take.speed}x</span>
                      <button
                        type="button"
                        className="text-[var(--color-accent)] hover:text-[var(--color-accent)] flex items-center gap-1 transition-colors"
                        onClick={() => alert(`Take "${take.id}" queued for Timeline Arranger insertion.`)}
                      >
                        <ArrowRightCircle className="w-4 h-4" />
                        Insert into Track
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
