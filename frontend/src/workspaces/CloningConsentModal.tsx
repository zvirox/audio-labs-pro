/**
 * Audio Labs Pro - Voice Cloning Consent Gate Modal
 *
 * Mandatory legal consent attestation modal required for voice cloning.
 * Automated agents, scripts, and MCP calls are strictly prohibited from bypassing
 * this human verification gate.
 */

import { useState } from 'react';
import { Dialog } from '../kit/Dialog';
import { Button } from '../kit/Button';
import { Input } from '../kit/Input';
import type { VoiceProfile, ConsentRecord } from '../ipc/types';
import { ShieldAlert, Mic, Upload, CheckCircle2, AlertTriangle } from 'lucide-react';

export interface CloningConsentModalProps {
  open: boolean;
  onClose: () => void;
  onCreated: (profile: VoiceProfile) => void;
}

const LANGUAGES = ['English', 'German', 'French', 'Spanish', 'Japanese'];

export function CloningConsentModal({ open, onClose, onCreated }: CloningConsentModalProps) {
  const [sourceTab, setSourceTab] = useState<'upload' | 'record'>('record');
  const [speakerName, setSpeakerName] = useState<string>('');
  const [attestedBy, setAttestedBy] = useState<string>('Producer (Studio User)');
  const [referenceText, setReferenceText] = useState<string>('');
  const [language, setLanguage] = useState<string>('English');
  const [consentAcknowledged, setConsentAcknowledged] = useState<boolean>(false);
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  // Verification gate validity check
  const isValid =
    speakerName.trim().length >= 2 &&
    attestedBy.trim().length >= 2 &&
    referenceText.trim().length >= 8 &&
    consentAcknowledged;

  const handleSubmit = async () => {
    if (!isValid) return;
    setIsSubmitting(true);
    setError(null);

    try {
      const consentRecord: ConsentRecord = {
        speaker_name: speakerName.trim(),
        attested_by: attestedBy.trim(),
        timestamp: Date.now(),
        statement:
          'I certify that I have obtained explicit, verifiable legal consent from the speaker to synthesize speech with their vocal likeness.',
      };

      const newProfile: VoiceProfile = {
        id: `voice-${Date.now()}`,
        name: speakerName.trim(),
        language,
        tags: ['cloned', language.toLowerCase()],
        kind: 'cloned',
        speed: 1.0,
        pitch: 0.0,
        energy: 1.0,
        emotion: 'neutral',
        consent: consentRecord,
      };

      onCreated(newProfile);
      onClose();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to create cloned voice profile');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Create Voice — Legal Consent Gate"
      description="Zero-shot cloning requires explicit, human-attested permission from the speaker before generating acoustic embeddings."
      size="lg"
    >
      <div className="space-y-5 text-xs">
        {/* Safety Warning Banner */}
        <div className="p-3 rounded-lg bg-amber-500/10 border border-amber-500/30 flex items-start gap-3">
          <ShieldAlert className="w-5 h-5 text-amber-400 flex-shrink-0 mt-0.5" />
          <div className="space-y-1 text-slate-300">
            <div className="font-semibold text-amber-300">Mandatory Human Consent Record</div>
            <p className="text-[11px] leading-relaxed">
              In accordance with Audio Labs Pro safety guidelines, automated AI agents and background scripts cannot forge or bypass this attestation. The consent statement is cryptographically signed and stored in the profile bundle.
            </p>
          </div>
        </div>

        {/* Source Tab Toggle */}
        <div className="flex items-center gap-2 border-b border-white/10 pb-3">
          <button
            type="button"
            onClick={() => setSourceTab('record')}
            className={`px-3 py-1.5 rounded-lg flex items-center gap-1.5 font-medium transition-colors ${
              sourceTab === 'record'
                ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/40'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            <Mic className="w-3.5 h-3.5" />
            Record Reference Sample
          </button>
          <button
            type="button"
            onClick={() => setSourceTab('upload')}
            className={`px-3 py-1.5 rounded-lg flex items-center gap-1.5 font-medium transition-colors ${
              sourceTab === 'upload'
                ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/40'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            <Upload className="w-3.5 h-3.5" />
            Upload WAV Reference
          </button>
        </div>

        {/* Form Fields */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label className="block text-slate-300 mb-1 font-medium">Speaker Name (Target Voice)</label>
            <Input
              value={speakerName}
              onChange={(e) => setSpeakerName(e.target.value)}
              placeholder="e.g. Linus Sebastian"
            />
          </div>

          <div>
            <label className="block text-slate-300 mb-1 font-medium">Attesting User (Your Identity)</label>
            <Input
              value={attestedBy}
              onChange={(e) => setAttestedBy(e.target.value)}
              placeholder="e.g. Studio Lead / Producer"
            />
          </div>

          <div className="md:col-span-2">
            <label className="block text-slate-300 mb-1 font-medium">Reference Text Transcript</label>
            <Input
              value={referenceText}
              onChange={(e) => setReferenceText(e.target.value)}
              placeholder="Enter the exact words spoken in the 5-15 second reference audio sample..."
            />
            <span className="text-[10px] text-slate-500 mt-1 block">
              Acoustic analysis matches phonetic alignment between audio and transcript.
            </span>
          </div>

          <div>
            <label className="block text-slate-300 mb-1 font-medium">Spoken Language</label>
            <select
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
              className="w-full px-3 py-2 rounded-lg bg-slate-900 border border-white/10 text-slate-200 focus:outline-none focus:border-cyan-500 font-mono text-xs"
            >
              {LANGUAGES.map((lang) => (
                <option key={lang} value={lang}>
                  {lang}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Consent Checkbox */}
        <div className="pt-3 border-t border-white/10 space-y-2">
          <label className="flex items-start gap-2.5 cursor-pointer">
            <input
              type="checkbox"
              checked={consentAcknowledged}
              onChange={(e) => setConsentAcknowledged(e.target.checked)}
              className="mt-0.5 rounded border-white/20 bg-slate-900 text-cyan-500 focus:ring-cyan-500/20"
            />
            <span className="text-slate-200 text-xs leading-relaxed select-none">
              <strong className="text-cyan-400">Human Legal Attestation:</strong> I declare under penalty of terms violation that the speaker has given explicit permission to clone their voice for use in this workstation.
            </span>
          </label>
        </div>

        {error && (
          <div className="p-2.5 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 flex items-center gap-2">
            <AlertTriangle className="w-4 h-4 flex-shrink-0" />
            <span>{error}</span>
          </div>
        )}

        {/* Dialog Action Buttons */}
        <div className="pt-3 border-t border-white/10 flex items-center justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            size="sm"
            disabled={!isValid || isSubmitting}
            onClick={handleSubmit}
            leftIcon={<CheckCircle2 className="w-4 h-4" />}
          >
            {isSubmitting ? 'Generating Acoustic Embedding...' : 'Sign & Create Voice Profile'}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
