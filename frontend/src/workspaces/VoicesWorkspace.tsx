/**
 * Audio Labs Pro - Voices Workspace
 *
 * Voice profile library, neural model slot assignment, acoustic voice design,
 * and access-controlled voice cloning with mandatory legal consent.
 */

import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Slider } from '../kit/Slider';
import { getVoiceProfiles } from '../ipc/client';
import type { VoiceProfile } from '../ipc/types';
import { CloningConsentModal } from './CloningConsentModal';
import {
  Users,
  Plus,
  Play,
  Volume2,
  ShieldCheck,
  Sliders,
} from 'lucide-react';

export function VoicesWorkspace() {
  const [profiles, setProfiles] = useState<VoiceProfile[]>([]);
  const [selectedId, setSelectedId] = useState<string>('');
  const [isModalOpen, setIsModalOpen] = useState<boolean>(false);
  const [playingVoiceId, setPlayingVoiceId] = useState<string | null>(null);

  useEffect(() => {
    getVoiceProfiles().then((profs) => {
      setProfiles(profs);
      if (profs.length > 0) setSelectedId(profs[0].id);
    });
  }, []);

  const activeProfile = profiles.find((p) => p.id === selectedId) || profiles[0];

  const handlePreviewVoice = (id: string) => {
    setPlayingVoiceId(id);
    setTimeout(() => setPlayingVoiceId(null), 1500);
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

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      {/* Workspace Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
            <Users className="w-4 h-4 text-[var(--color-accent)]" />
            Voice Profiles & Acoustic Lab
          </h2>
          <p className="text-xs text-[var(--color-fg-muted)] mt-1">
            Browse installed neural voice personas, design pitch and cadence, or clone new voices with verified consent.
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
                        {isPlaying ? (
                          <Volume2 className="w-4 h-4" />
                        ) : (
                          <Play className="w-4 h-4" />
                        )}
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
                {/* Consent Badge if Cloned */}
                {activeProfile.consent && (
                  <div className="p-3 rounded-lg bg-chip-sky border border-border-strong flex items-start gap-3">
                    <ShieldCheck className="w-4 h-4 text-[var(--color-fg-muted)] flex-shrink-0 mt-0.5" />
                    <div className="space-y-0.5 text-[var(--color-fg-muted)]">
                      <div className="font-medium text-[var(--color-fg-muted)]">Verified Legal Consent Record</div>
                      <div className="text-[11px] font-mono text-[var(--color-fg-muted)]">
                        Speaker: {activeProfile.consent.speaker_name} · Attested by: {activeProfile.consent.attested_by}
                      </div>
                    </div>
                  </div>
                )}

                {/* Acoustic Parameter Faders */}
                <div className="space-y-4">
                  <h3 className="font-medium text-[var(--color-fg)] flex items-center gap-1.5">
                    <Sliders className="w-4 h-4 text-[var(--color-accent)]" />
                    Acoustic Voice Tuning
                  </h3>

                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <div>
                      <Slider
                        label="Speech Cadence / Speed"
                        min={0.5}
                        max={1.75}
                        step={0.05}
                        value={activeProfile.speed}
                        onChange={(v) => updateActiveParam('speed', v)}
                        formatValue={(v) => `${v.toFixed(2)}x`}
                      />
                    </div>
                    <div>
                      <Slider
                        label="Fundamental Pitch Shift"
                        min={-12}
                        max={12}
                        step={0.5}
                        value={activeProfile.pitch}
                        onChange={(v) => updateActiveParam('pitch', v)}
                        unit="st"
                      />
                    </div>
                    <div>
                      <Slider
                        label="Vocal Energy / Effort"
                        min={0.5}
                        max={1.5}
                        step={0.05}
                        value={activeProfile.energy}
                        onChange={(v) => updateActiveParam('energy', v)}
                        formatValue={(v) => `${(v * 100).toFixed(0)}%`}
                      />
                    </div>
                    <div>
                      <label className="block text-[var(--color-fg-muted)] mb-1 font-medium">Expression Tone</label>
                      <select
                        value={activeProfile.emotion}
                        onChange={(e) => updateActiveParam('emotion', e.target.value)}
                        className="w-full px-3 py-2 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)] font-mono text-xs"
                      >
                        <option value="neutral">Neutral (Balanced)</option>
                        <option value="warm">Warm & Conversational</option>
                        <option value="excited">Energetic & Punchy</option>
                        <option value="whisper">Soft / Whisper</option>
                      </select>
                    </div>
                  </div>
                </div>

                {/* Synthesis Test Phrase */}
                <div className="pt-4 border-t border-[var(--color-border)] flex items-center justify-between">
                  <span className="text-[var(--color-fg-muted)] font-mono text-[11px]">
                    Model engine: Kokoro-82M / StyleTTS2 Studio
                  </span>
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => handlePreviewVoice(activeProfile.id)}
                    leftIcon={<Play className="w-4 h-4" />}
                  >
                    Test Synthesis Phrase
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

      {/* Consent Modal Dialog */}
      <CloningConsentModal
        open={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onCreated={handleCreatedProfile}
      />
    </div>
  );
}
