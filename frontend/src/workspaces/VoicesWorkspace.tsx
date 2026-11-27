/**
 * Audio Labs Pro - Voices & Lexicon Workspace
 *
 * Dedicated views for:
 * - `cloning`: Voice profile library, tuning, consent-gated cloning, and synthesis preview
 * - `lexicon`: Pronunciation dictionary & [pause=...]/[emphasis] markup tester
 */

import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Input } from '../kit/Input';
import { Slider } from '../kit/Slider';
import {
  applyPronunciationLexicon,
  desktopTtsSynthesize,
  getVoiceProfiles,
} from '../ipc/client';
import type { LexiconPreview, LexiconRule, VoiceProfile } from '../ipc/types';
import { CloningConsentModal } from './CloningConsentModal';
import {
  Users,
  Plus,
  Play,
  Volume2,
  ShieldCheck,
  Sliders,
  BookA,
  Trash2,
} from 'lucide-react';

export interface VoicesWorkspaceProps {
  subtab?: string;
}

export function VoicesWorkspace({ subtab = 'cloning' }: VoicesWorkspaceProps) {
  const [profiles, setProfiles] = useState<VoiceProfile[]>([]);
  const [selectedId, setSelectedId] = useState<string>('');
  const [isModalOpen, setIsModalOpen] = useState<boolean>(false);
  const [playingVoiceId, setPlayingVoiceId] = useState<string | null>(null);
  const [synthNotice, setSynthNotice] = useState<string | null>(null);

  // Pronunciation Lexicon state
  const [rules, setRules] = useState<LexiconRule[]>([
    { word: 'ALSA', replacement: 'ahl-sah' },
    { word: 'LUFS', replacement: 'lufs' },
    { word: 'VST3', replacement: 'V-S-T three' },
  ]);
  const [newWord, setNewWord] = useState('');
  const [newReplacement, setNewReplacement] = useState('');
  const [lexiconInput, setLexiconInput] = useState(
    'Testing ALSA [pause=350ms] with [emphasis]VST3[/emphasis] loudness at -16 LUFS.'
  );
  const [lexiconResult, setLexiconResult] = useState<LexiconPreview | null>(null);

  useEffect(() => {
    getVoiceProfiles().then((profs) => {
      setProfiles(profs);
      if (profs.length > 0) setSelectedId(profs[0].id);
    });
  }, []);

  useEffect(() => {
    if (subtab === 'lexicon') {
      applyPronunciationLexicon(rules, lexiconInput)
        .then(setLexiconResult)
        .catch(() => {});
    }
  }, [subtab, rules, lexiconInput]);

  const activeProfile = profiles.find((p) => p.id === selectedId) || profiles[0];

  const handlePreviewVoice = async (id: string) => {
    setPlayingVoiceId(id);
    setSynthNotice(null);
    const prof = profiles.find((p) => p.id === id) || activeProfile;
    try {
      const report = await desktopTtsSynthesize(
        id,
        `Voice audition for ${prof?.name || id}.`,
        prof?.speed || 1.0
      );
      setSynthNotice(`Rendered ${report.duration_secs.toFixed(1)}s via ${report.engine} -> ${report.path}`);
    } catch (error) {
      setSynthNotice(`Preview failed: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setPlayingVoiceId(null);
    }
  };

  const handleCreatedProfile = (newProfile: VoiceProfile) => {
    setProfiles((prev) => [newProfile, ...prev]);
    setSelectedId(newProfile.id);
  };

  const updateActiveParam = (field: keyof VoiceProfile, val: number | string) => {
    if (!activeProfile) return;
    setProfiles((prev) =>
      prev.map((p) => (p.id === activeProfile.id ? { ...p, [field]: val } : p))
    );
  };

  if (subtab === 'lexicon') {
    return (
      <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
        <div>
          <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
            <BookA className="w-4 h-4 text-[var(--color-accent)]" />
            Pronunciation Lexicon &amp; Speech Markup
          </h2>
          <p className="text-xs text-[var(--color-fg-muted)] mt-1">
            Define custom phonetic respellings and test inline <code>[pause=...]</code> and{' '}
            <code>[emphasis]...[/emphasis]</code> tags through <code>alp-voice::lexicon</code>.
          </p>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-5 space-y-4">
            <Card title="Dictionary Overrides" subtitle={`${rules.length} active word rule(s)`}>
              <div className="space-y-3 text-xs">
                <div className="space-y-2 max-h-64 overflow-y-auto pr-1">
                  {rules.map((r, idx) => (
                    <div
                      key={`${r.word}-${idx}`}
                      className="p-2.5 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] flex items-center justify-between gap-2 font-mono"
                    >
                      <span>
                        <strong className="text-fg">{r.word}</strong> →{' '}
                        <span className="text-primary-text">{r.replacement}</span>
                      </span>
                      <button
                        type="button"
                        onClick={() => setRules((prev) => prev.filter((_, i) => i !== idx))}
                        className="text-red-text hover:text-fg p-1 cursor-pointer"
                        title="Remove rule"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  ))}
                </div>

                <div className="pt-3 border-t border-[var(--color-border)] space-y-2">
                  <div className="grid grid-cols-2 gap-2">
                    <Input
                      label="Word"
                      placeholder="PipeWire"
                      value={newWord}
                      onChange={(e) => setNewWord(e.target.value)}
                    />
                    <Input
                      label="Spoken Form"
                      placeholder="pipe-wire"
                      value={newReplacement}
                      onChange={(e) => setNewReplacement(e.target.value)}
                    />
                  </div>
                  <Button
                    variant="secondary"
                    size="xs"
                    leftIcon={<Plus className="w-3.5 h-3.5" />}
                    onClick={() => {
                      if (!newWord.trim() || !newReplacement.trim()) return;
                      setRules((prev) => [
                        ...prev,
                        { word: newWord.trim(), replacement: newReplacement.trim() },
                      ]);
                      setNewWord('');
                      setNewReplacement('');
                    }}
                  >
                    Add Pronunciation Rule
                  </Button>
                </div>
              </div>
            </Card>
          </div>

          <div className="lg:col-span-7 space-y-4">
            <Card title="Markup Parser Preview" subtitle="Live Phonetic & Timing Tokenizer">
              <div className="space-y-4 text-xs">
                <div>
                  <label className="block text-[var(--color-fg-muted)] mb-1.5 font-medium">
                    Input Script with Markup
                  </label>
                  <textarea
                    rows={4}
                    value={lexiconInput}
                    onChange={(e) => setLexiconInput(e.target.value)}
                    className="w-full px-3 py-2 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] font-mono text-xs resize-none"
                  />
                </div>

                {lexiconResult && (
                  <>
                    <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] space-y-1">
                      <div className="text-[11px] font-medium text-[var(--color-fg-muted)]">
                        Substituted Text
                      </div>
                      <div className="font-mono text-fg">{lexiconResult.output_text}</div>
                    </div>

                    <div className="space-y-1.5">
                      <div className="text-[11px] font-medium text-[var(--color-fg-muted)]">
                        Parsed Speech Tokens ({lexiconResult.tokens.length})
                      </div>
                      <div className="flex flex-wrap gap-1.5">
                        {lexiconResult.tokens.map((tok, i) => (
                          <span
                            key={`${tok}-${i}`}
                            className="px-2 py-1 rounded bg-surface-2 border border-border font-mono text-[11px] text-primary-text"
                          >
                            {tok}
                          </span>
                        ))}
                      </div>
                    </div>
                  </>
                )}
              </div>
            </Card>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      {/* Workspace Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
            <Users className="w-4 h-4 text-[var(--color-accent)]" />
            Voice Profiles &amp; Consent-Gated Cloning
          </h2>
          <p className="text-xs text-[var(--color-fg-muted)] mt-1">
            Browse installed voice personas, tune acoustic parameters, and create cloned voices with verified speaker consent.
          </p>
        </div>

        <Button
          variant="primary"
          size="sm"
          onClick={() => setIsModalOpen(true)}
          leftIcon={<Plus className="w-4 h-4" />}
        >
          Create / Clone Voice
        </Button>
      </div>

      {synthNotice && (
        <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-strong)] text-xs font-mono text-sky-text">
          {synthNotice}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left: Profile Cards Library (1 col) */}
        <div className="space-y-4">
          <Card title="Voice Library" subtitle={`${profiles.length} available personas`}>
            <div className="space-y-2.5 max-h-[520px] overflow-y-auto pr-1">
              {profiles.map((profile) => {
                const isSelected = profile.id === selectedId;
                const isPlaying = playingVoiceId === profile.id;

                return (
                  <div
                    key={profile.id}
                    onClick={() => setSelectedId(profile.id)}
                    className={`p-3 rounded-lg border transition-colors cursor-pointer space-y-2 text-xs ${
                      isSelected
                        ? 'bg-[var(--bg-surface)] border-[var(--border-strong)]'
                        : 'bg-[var(--bg-app)] border-[var(--border)] hover:border-[var(--border-strong)]'
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <div className="w-7 h-7 rounded-full bg-primary-tint text-primary-text flex items-center justify-center font-medium text-xs">
                          {profile.name.charAt(0)}
                        </div>
                        <div>
                          <div className="font-medium text-[var(--color-fg)]">{profile.name}</div>
                          <div className="text-[10px] text-[var(--color-fg-muted)] font-mono capitalize">
                            {profile.kind} · {profile.language}
                          </div>
                        </div>
                      </div>

                      <Button
                        variant={isPlaying ? 'primary' : 'ghost'}
                        size="xs"
                        onClick={(e) => {
                          e.stopPropagation();
                          handlePreviewVoice(profile.id);
                        }}
                        aria-label="Preview voice"
                      >
                        {isPlaying ? <Volume2 className="w-4 h-4" /> : <Play className="w-4 h-4" />}
                      </Button>
                    </div>

                    <div className="flex flex-wrap gap-1">
                      {profile.tags.map((tag) => (
                        <span
                          key={tag}
                          className="px-1.5 py-0.5 rounded bg-[var(--color-surface-2)] text-[10px] font-mono text-[var(--color-fg-muted)]"
                        >
                          #{tag}
                        </span>
                      ))}
                    </div>
                  </div>
                );
              })}
            </div>
          </Card>
        </div>

        {/* Right: Active Profile Details & Acoustic Design (2 cols) */}
        <div className="lg:col-span-2 space-y-6">
          {activeProfile ? (
            <Card
              title={activeProfile.name}
              subtitle={`${activeProfile.kind.toUpperCase()} Persona · ${activeProfile.language}`}
            >
              <div className="space-y-6 text-xs">
                {activeProfile.consent && (
                  <div className="p-3 rounded-lg bg-chip-sky border border-border-strong flex items-start gap-3">
                    <ShieldCheck className="w-4 h-4 text-[var(--color-fg-muted)] flex-shrink-0 mt-0.5" />
                    <div className="space-y-0.5 text-[var(--color-fg-muted)]">
                      <div className="font-medium text-[var(--color-fg-muted)]">
                        Verified Legal Consent Record
                      </div>
                      <div className="text-[11px] font-mono text-[var(--color-fg-muted)]">
                        Speaker: {activeProfile.consent.speaker_name} · Attested by:{' '}
                        {activeProfile.consent.attested_by}
                      </div>
                    </div>
                  </div>
                )}

                <div className="space-y-4">
                  <h3 className="font-medium text-[var(--color-fg)] flex items-center gap-1.5">
                    <Sliders className="w-4 h-4 text-[var(--color-accent)]" />
                    Acoustic Voice Tuning
                  </h3>

                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <Slider
                      label="Speech Cadence / Speed"
                      min={0.5}
                      max={1.75}
                      step={0.05}
                      value={activeProfile.speed}
                      onChange={(v) => updateActiveParam('speed', v)}
                      formatValue={(v) => `${v.toFixed(2)}x`}
                    />
                    <Slider
                      label="Fundamental Pitch Shift"
                      min={-12}
                      max={12}
                      step={0.5}
                      value={activeProfile.pitch}
                      onChange={(v) => updateActiveParam('pitch', v)}
                      unit="st"
                    />
                    <Slider
                      label="Vocal Energy / Effort"
                      min={0.5}
                      max={1.5}
                      step={0.05}
                      value={activeProfile.energy}
                      onChange={(v) => updateActiveParam('energy', v)}
                      formatValue={(v) => `${(v * 100).toFixed(0)}%`}
                    />
                    <div>
                      <label className="block text-[var(--color-fg-muted)] mb-1 font-medium">
                        Expression Tone
                      </label>
                      <select
                        value={activeProfile.emotion}
                        onChange={(e) => updateActiveParam('emotion', e.target.value)}
                        className="w-full px-3 py-2 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)] font-mono text-xs"
                      >
                        <option value="neutral">Neutral (Balanced)</option>
                        <option value="warm">Warm &amp; Conversational</option>
                        <option value="excited">Energetic &amp; Punchy</option>
                        <option value="whisper">Soft / Whisper</option>
                      </select>
                    </div>
                  </div>
                </div>

                <div className="pt-4 border-t border-[var(--color-border)] flex items-center justify-between">
                  <span className="text-[var(--color-fg-muted)] font-mono text-[11px]">
                    Profile ID: {activeProfile.id}
                  </span>
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => handlePreviewVoice(activeProfile.id)}
                    leftIcon={<Play className="w-4 h-4" />}
                  >
                    Synthesize Test Take
                  </Button>
                </div>
              </div>
            </Card>
          ) : (
            <div className="p-8 text-center text-[var(--text-muted)] bg-[var(--bg-surface)] rounded-lg border border-[var(--border)]">
              Select a voice profile to view acoustic parameters.
            </div>
          )}
        </div>
      </div>

      <CloningConsentModal
        open={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onCreated={handleCreatedProfile}
      />
    </div>
  );
}
