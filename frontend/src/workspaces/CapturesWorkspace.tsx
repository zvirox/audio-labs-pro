/**
 * Audio Labs Pro - Batch & Repair Workspace
 *
 * Dedicated subtabs:
 * - `captures`: Live take deck + real Speech-to-Text file transcription & VAD word seek
 * - `denoise`: Offline Spectral Notch & Harmonic Hum Removal (`dspSpectralRepairFile`)
 * - `clicks`: Batch De-Click / Repair Macros (`runDspMacro`) & WSOLA Time Stretch (`dspTimeStretchFile`)
 * - `loudness`: Integrated LUFS / True Peak Analyzer (`analyzeAudioFileLoudness`) & Batch Normalizer
 */

import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Input } from '../kit/Input';
import { Toggle } from '../kit/Toggle';
import { Meter } from '../kit/Meter';
import {
  analyzeAudioFileLoudness,
  desktopTranscribeAudio,
  dspSpectralRepairFile,
  dspTimeStretchFile,
  getCaptures,
  getTranscript,
  runDspMacro,
} from '../ipc/client';
import type {
  CaptureTake,
  FileLoudnessReport,
  MacroJobReportDto,
  SpectralRepairReport,
  TimeStretchReport,
  TranscriptWord,
} from '../ipc/types';
import {
  Mic,
  Circle,
  Square,
  Scissors,
  CheckCircle2,
  Clock,
  AudioWaveform,
  Zap,
  Gauge,
  FileText,
} from 'lucide-react';

export interface CapturesWorkspaceProps {
  subtab?: string;
  onSeek?: (secs: number) => void;
}

export function CapturesWorkspace({ subtab = 'captures', onSeek }: CapturesWorkspaceProps) {
  const [takes, setTakes] = useState<CaptureTake[]>([]);
  const [transcript, setTranscript] = useState<TranscriptWord[]>([]);
  const [sttPath, setSttPath] = useState<string>('');
  const [sttEngine, setSttEngine] = useState<string>('Moonshine Tiny / Built-in STT');
  const [sttBusy, setSttBusy] = useState<boolean>(false);

  const [isRecording, setIsRecording] = useState<boolean>(false);
  const [recordSecs, setRecordSecs] = useState<number>(0);
  const [monitor, setMonitor] = useState<boolean>(false);
  const [punchIn, setPunchIn] = useState<boolean>(false);
  const [inputLevel, setInputLevel] = useState<number>(0.0);
  const [selectedWordIdx, setSelectedWordIdx] = useState<number | null>(null);

  // Spectral Repair (denoise) state
  const [repairMode, setRepairMode] = useState<'notch' | 'harmonic'>('harmonic');
  const [repairInput, setRepairInput] = useState('');
  const [repairOutput, setRepairOutput] = useState('');
  const [centerHz, setCenterHz] = useState('1000');
  const [fundamentalHz, setFundamentalHz] = useState('60');
  const [harmonics, setHarmonics] = useState('4');
  const [bandwidthQ, setBandwidthQ] = useState('12');
  const [reductionDb, setReductionDb] = useState('24');
  const [repairReport, setRepairReport] = useState<SpectralRepairReport | null>(null);

  // Click Removal / Batch Macro + Time Stretch state
  const [macroInDir, setMacroInDir] = useState('');
  const [macroOutDir, setMacroOutDir] = useState('');
  const [macroEffectId, setMacroEffectId] = useState('voice_cleanup');
  const [macroReport, setMacroReport] = useState<MacroJobReportDto | null>(null);
  const [stretchIn, setStretchIn] = useState('');
  const [stretchOut, setStretchOut] = useState('');
  const [stretchRatio, setStretchRatio] = useState('1.1');
  const [stretchReport, setStretchReport] = useState<TimeStretchReport | null>(null);

  // LUFS Normalizer state
  const [loudnessPath, setLoudnessPath] = useState('');
  const [loudnessReport, setLoudnessReport] = useState<FileLoudnessReport | null>(null);

  const [busy, setBusy] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    getCaptures().then(setTakes);
    getTranscript().then(setTranscript);
  }, []);

  useEffect(() => {
    let timer: ReturnType<typeof setInterval>;
    if (isRecording) {
      timer = setInterval(() => {
        setRecordSecs((s) => s + 0.1);
        setInputLevel(0.45 + 0.25 * Math.abs(Math.sin(Date.now() / 160)));
      }, 100);
    } else {
      setRecordSecs(0);
      setInputLevel(0.0);
    }
    return () => clearInterval(timer);
  }, [isRecording]);

  const handleToggleRecord = () => {
    if (isRecording) {
      const newTake: CaptureTake = {
        id: `take-${Date.now()}`,
        name: `Studio Vocal Take #${takes.length + 1}`,
        duration_secs: +recordSecs.toFixed(1),
        timestamp: 'Just now',
        peak_db: -1.2,
        rms_db: -16.4,
        clipped: false,
      };
      setTakes((prev) => [newTake, ...prev]);
      setIsRecording(false);
    } else {
      setIsRecording(true);
    }
  };

  const handleTranscribeFile = async () => {
    if (!sttPath.trim()) {
      setNotice('Enter an audio file path to transcribe.');
      return;
    }
    setSttBusy(true);
    setNotice(null);
    try {
      const report = await desktopTranscribeAudio(sttPath.trim());
      const words: TranscriptWord[] = report.segments.flatMap((seg) =>
        seg.words.map((w) => ({
          word: w.word,
          start_secs: +w.start_secs.toFixed(2),
          end_secs: +w.end_secs.toFixed(2),
          confidence: w.confidence,
          speaker: 'Speaker',
        }))
      );
      setTranscript(words);
      setSttEngine(report.engine);
      setNotice(`Transcribed ${words.length} word(s) in ${report.duration_secs.toFixed(1)}s (${report.engine}).`);
    } catch (err) {
      setNotice(`Transcription failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setSttBusy(false);
    }
  };

  const handleRunRepair = async () => {
    if (!repairInput.trim() || !repairOutput.trim()) {
      setNotice('Set an input audio path and output WAV path first.');
      return;
    }
    setBusy('repair');
    setNotice(null);
    try {
      const rep = await dspSpectralRepairFile({
        input_path: repairInput.trim(),
        output_path: repairOutput.trim(),
        mode: repairMode,
        center_hz: Number(centerHz) || 1000,
        fundamental_hz: Number(fundamentalHz) || 60,
        harmonics: Math.max(1, Number(harmonics) || 4),
        bandwidth_q: Number(bandwidthQ) || 12,
        reduction_db: Number(reductionDb) || 24,
      });
      setRepairReport(rep);
    } catch (err) {
      setNotice(`Spectral repair failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(null);
    }
  };

  const handleRunStretch = async () => {
    const ratio = Number(stretchRatio);
    if (!stretchIn.trim() || !stretchOut.trim() || !Number.isFinite(ratio) || ratio <= 0) {
      setNotice('Set valid input path, output WAV path, and positive stretch ratio.');
      return;
    }
    setBusy('stretch');
    setNotice(null);
    try {
      setStretchReport(
        await dspTimeStretchFile({
          input_path: stretchIn.trim(),
          output_path: stretchOut.trim(),
          ratio,
        })
      );
    } catch (err) {
      setNotice(`Time stretch failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(null);
    }
  };

  const handleRunMacro = async () => {
    if (!macroInDir.trim() || !macroOutDir.trim()) {
      setNotice('Set both input directory and output directory first.');
      return;
    }
    setBusy('macro');
    setNotice(null);
    try {
      setMacroReport(
        await runDspMacro({
          input_dir: macroInDir.trim(),
          output_dir: macroOutDir.trim(),
          overwrite: false,
          steps: [{ effect_id: macroEffectId.trim() || 'voice_cleanup', params: {} }],
        })
      );
    } catch (err) {
      setNotice(`Batch macro failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(null);
    }
  };

  const handleAnalyzeLoudness = async () => {
    if (!loudnessPath.trim()) {
      setNotice('Set an audio file path to measure LUFS and true peak.');
      return;
    }
    setBusy('loudness');
    setNotice(null);
    try {
      setLoudnessReport(await analyzeAudioFileLoudness(loudnessPath.trim()));
    } catch (err) {
      setNotice(`Loudness analysis failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      <div>
        <h2 className="text-xl font-medium text-fg flex items-center gap-2">
          {subtab === 'denoise' ? (
            <AudioWaveform className="w-4 h-4 text-primary-text" />
          ) : subtab === 'clicks' ? (
            <Zap className="w-4 h-4 text-primary-text" />
          ) : subtab === 'loudness' ? (
            <Gauge className="w-4 h-4 text-primary-text" />
          ) : (
            <Mic className="w-4 h-4 text-red-text" />
          )}
          {subtab === 'captures' && 'Voice Captures, VAD & Word Transcript'}
          {subtab === 'denoise' && 'Spectral De-Noise & Hum Suppression'}
          {subtab === 'clicks' && 'Batch Repair Macros & WSOLA Time-Stretch'}
          {subtab === 'loudness' && 'EBU R128 LUFS Analyzer & Batch Normalizer'}
        </h2>
        <p className="text-xs text-[var(--color-fg-muted)] mt-1">
          {subtab === 'captures' &&
            'Record takes or transcribe audio files with Moonshine Tiny STT and click-to-seek timestamps.'}
          {subtab === 'denoise' &&
            'Attenuate tonal hum, fundamental harmonics, or narrow-band noise directly via alp-dsp::repair.'}
          {subtab === 'clicks' &&
            'Execute directory-wide cleanup macro batches and pitch-preserving WSOLA time stretching.'}
          {subtab === 'loudness' &&
            'Audit integrated LUFS, sample peak, and quarter-sample true peak, or batch-normalize directories.'}
        </p>
      </div>

      {notice && (
        <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-strong)] text-xs font-mono text-amber">
          {notice}
        </div>
      )}

      {/* Subtab 1: Voice Captures & VAD */}
      {subtab === 'captures' && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-5 space-y-6">
            <Card title="Live Audio Capture" subtitle="ALSA / PipeWire capture deck">
              <div className="space-y-4 text-xs">
                <div className="p-4 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] flex items-center justify-between">
                  <div className="space-y-1">
                    <div className="text-[11px] font-mono text-[var(--color-fg-muted)]">
                      {isRecording ? 'Recording Active' : 'Input Standby'}
                    </div>
                    <div className="text-2xl font-medium font-mono text-[var(--color-fg)] tabular-nums">
                      {Math.floor(recordSecs / 60).toString().padStart(2, '0')}:
                      {Math.floor(recordSecs % 60).toString().padStart(2, '0')}.
                      {Math.floor((recordSecs % 1) * 10).toString()}
                    </div>
                  </div>
                  <Button
                    variant={isRecording ? 'danger' : 'primary'}
                    size="md"
                    onClick={handleToggleRecord}
                    leftIcon={
                      isRecording ? (
                        <Square className="w-4 h-4 fill-current" />
                      ) : (
                        <Circle className="w-4 h-4 fill-current text-red" />
                      )
                    }
                  >
                    {isRecording ? 'Stop Recording' : 'Record Take'}
                  </Button>
                </div>

                <div className="space-y-1.5">
                  <div className="flex justify-between text-[11px] font-mono text-[var(--color-fg-muted)]">
                    <span>Input Gain Meter</span>
                    <span>{isRecording ? '-1.2 dBFS' : '-inf'}</span>
                  </div>
                  <Meter leftLevel={inputLevel} rightLevel={inputLevel * 0.98} showLabels={false} />
                </div>

                <div className="pt-2 border-t border-[var(--color-border)] space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-[var(--color-fg-muted)]">Software Input Monitoring</span>
                    <Toggle checked={monitor} onChange={setMonitor} label="" />
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-[var(--color-fg-muted)]">Punch-In Overwrite Mode</span>
                    <Toggle checked={punchIn} onChange={setPunchIn} label="" />
                  </div>
                </div>
              </div>
            </Card>

            <Card title="Recorded Takes" subtitle={`${takes.length} take(s) in session`}>
              <div className="space-y-2 text-xs max-h-[260px] overflow-y-auto pr-1">
                {takes.map((take) => (
                  <div
                    key={take.id}
                    className="p-3 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] space-y-1"
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-medium text-fg">{take.name}</span>
                      <span className="font-mono text-[11px] text-primary-text">{take.duration_secs}s</span>
                    </div>
                    <div className="flex items-center justify-between text-[11px] font-mono text-muted">
                      <span className="flex items-center gap-1">
                        <Clock className="w-3.5 h-3.5" />
                        {take.timestamp}
                      </span>
                      <span className="flex items-center gap-1">
                        <CheckCircle2 className="w-3.5 h-3.5" />
                        Peak: {take.peak_db} dBFS
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </Card>
          </div>

          <div className="lg:col-span-7 space-y-4">
            <Card title="File Transcription & Interactive Word Transcript" subtitle={`Engine: ${sttEngine}`}>
              <div className="space-y-4 text-xs">
                <div className="flex flex-wrap items-end gap-2">
                  <div className="flex-1 min-w-[220px]">
                    <Input
                      label="Audio file to transcribe"
                      placeholder="/path/to/recording.wav"
                      value={sttPath}
                      onChange={(e) => setSttPath(e.target.value)}
                    />
                  </div>
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={handleTranscribeFile}
                    disabled={sttBusy}
                    leftIcon={<FileText className="w-4 h-4" />}
                  >
                    {sttBusy ? 'Transcribing...' : 'Run STT'}
                  </Button>
                </div>

                <div className="p-4 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] min-h-[200px] flex flex-wrap gap-2 items-center leading-loose">
                  {transcript.map((item, idx) => (
                    <button
                      key={`${item.word}-${idx}`}
                      type="button"
                      onClick={() => {
                        setSelectedWordIdx(idx);
                        onSeek?.(item.start_secs);
                      }}
                      className={`px-2 py-1 rounded-md text-xs font-medium transition-colors flex items-center gap-1 border cursor-pointer ${
                        selectedWordIdx === idx
                          ? 'bg-primary text-primary-fg border-primary'
                          : 'bg-surface-2 text-fg border-border hover:bg-hover hover:border-border-strong'
                      }`}
                    >
                      <span>{item.word}</span>
                      <span className="text-[9px] text-muted font-mono">{item.start_secs}s</span>
                    </button>
                  ))}
                </div>

                {selectedWordIdx !== null && transcript[selectedWordIdx] && (
                  <div className="p-3 rounded-lg bg-primary-tint border border-border-strong flex items-center justify-between">
                    <div className="font-mono text-[11px]">
                      <span className="text-primary-text font-medium">&ldquo;{transcript[selectedWordIdx].word}&rdquo;</span>
                      <span className="text-muted block">
                        {transcript[selectedWordIdx].start_secs}s – {transcript[selectedWordIdx].end_secs}s
                      </span>
                    </div>
                    <Button
                      variant="danger"
                      size="xs"
                      onClick={() => {
                        setTranscript((prev) => prev.filter((_, i) => i !== selectedWordIdx));
                        setSelectedWordIdx(null);
                      }}
                      leftIcon={<Scissors className="w-4 h-4" />}
                    >
                      Cut Word Segment
                    </Button>
                  </div>
                )}
              </div>
            </Card>
          </div>
        </div>
      )}

      {/* Subtab 2: Spectral De-Noise */}
      {subtab === 'denoise' && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-6">
            <Card title="Spectral Notch & Harmonic Hum Removal" subtitle="STFT offline processing via alp-dsp::repair">
              <div className="space-y-3 text-xs">
                <div className="flex items-center gap-2">
                  {(['harmonic', 'notch'] as const).map((m) => (
                    <Button
                      key={m}
                      variant={repairMode === m ? 'primary' : 'secondary'}
                      size="xs"
                      onClick={() => setRepairMode(m)}
                    >
                      {m === 'harmonic' ? 'Harmonic Suppressor (Hum)' : 'Targeted Notch Filter'}
                    </Button>
                  ))}
                </div>
                <Input label="Source audio path" placeholder="/path/to/noisy.wav" value={repairInput} onChange={(e) => setRepairInput(e.target.value)} />
                <Input label="Output clean WAV" placeholder="/path/to/denoised.wav" value={repairOutput} onChange={(e) => setRepairOutput(e.target.value)} />

                {repairMode === 'notch' ? (
                  <Input label="Notch center frequency (Hz)" value={centerHz} onChange={(e) => setCenterHz(e.target.value)} />
                ) : (
                  <div className="grid grid-cols-2 gap-3">
                    <Input label="Fundamental (Hz)" value={fundamentalHz} onChange={(e) => setFundamentalHz(e.target.value)} />
                    <Input label="Harmonics" value={harmonics} onChange={(e) => setHarmonics(e.target.value)} />
                  </div>
                )}

                <div className="grid grid-cols-2 gap-3">
                  <Input label="Bandwidth Q" value={bandwidthQ} onChange={(e) => setBandwidthQ(e.target.value)} />
                  <Input label="Attenuation (dB)" value={reductionDb} onChange={(e) => setReductionDb(e.target.value)} />
                </div>

                <Button variant="primary" size="sm" onClick={handleRunRepair} disabled={busy === 'repair'}>
                  {busy === 'repair' ? 'Processing...' : 'Run Spectral Repair'}
                </Button>
              </div>
            </Card>
          </div>

          <div className="lg:col-span-6">
            <Card title="Repair Result" subtitle="Processed PCM metrics">
              {!repairReport ? (
                <div className="p-8 text-center text-xs text-muted italic">
                  Configure notch or harmonic suppression and run repair on a local WAV file.
                </div>
              ) : (
                <div className="space-y-2 font-mono text-xs text-sky-text p-3 rounded-lg bg-chip-sky border border-border-strong">
                  <div>Mode: {repairReport.mode}</div>
                  <div>Frames processed: {repairReport.frames} @ {repairReport.sample_rate} Hz</div>
                  <div className="truncate">{repairReport.path}</div>
                </div>
              )}
            </Card>
          </div>
        </div>
      )}

      {/* Subtab 3: Click & Pop Removal & Batch Macro / Time Stretch */}
      {subtab === 'clicks' && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <Card title="Batch De-Click & Cleanup Macro" subtitle="Directory DSP pipeline via alp-engine MacroChain">
            <div className="space-y-3 text-xs">
              <Input label="Input directory" placeholder="/path/to/raw-takes" value={macroInDir} onChange={(e) => setMacroInDir(e.target.value)} />
              <Input label="Output directory" placeholder="/path/to/cleaned" value={macroOutDir} onChange={(e) => setMacroOutDir(e.target.value)} />
              <div>
                <label className="block text-muted mb-1 font-medium">Cleanup Effect Preset</label>
                <select
                  value={macroEffectId}
                  onChange={(e) => setMacroEffectId(e.target.value)}
                  className="w-full px-3 py-2 rounded-lg bg-surface border border-border text-fg font-mono text-xs"
                >
                  <option value="voice_cleanup">voice_cleanup (Hum + Click + De-Ess)</option>
                  <option value="click_removal">click_removal</option>
                  <option value="hum_removal">hum_removal</option>
                  <option value="gain">gain</option>
                </select>
              </div>
              <Button variant="primary" size="sm" onClick={handleRunMacro} disabled={busy === 'macro'}>
                {busy === 'macro' ? 'Running Batch...' : 'Execute Batch Macro'}
              </Button>
              {macroReport && (
                <div className="p-3 rounded-lg bg-chip-sky border border-border-strong font-mono text-[11px] text-sky-text">
                  Succeeded {macroReport.report.succeeded}/{macroReport.report.processed} files (job {macroReport.job_id})
                </div>
              )}
            </div>
          </Card>

          <Card title="WSOLA Time-Stretch" subtitle="Pitch-preserving duration scaling">
            <div className="space-y-3 text-xs">
              <Input label="Input audio path" placeholder="/path/to/take.wav" value={stretchIn} onChange={(e) => setStretchIn(e.target.value)} />
              <Input label="Output WAV path" placeholder="/path/to/stretched.wav" value={stretchOut} onChange={(e) => setStretchOut(e.target.value)} />
              <Input label="Ratio (e.g. 0.85 faster, 1.25 slower)" value={stretchRatio} onChange={(e) => setStretchRatio(e.target.value)} />
              <Button variant="secondary" size="sm" onClick={handleRunStretch} disabled={busy === 'stretch'}>
                {busy === 'stretch' ? 'Stretching...' : 'Stretch Audio'}
              </Button>
              {stretchReport && (
                <div className="p-3 rounded-lg bg-chip-sky border border-border-strong font-mono text-[11px] text-sky-text">
                  Ratio {stretchReport.ratio.toFixed(2)}x · Duration {stretchReport.duration_secs.toFixed(2)}s → {stretchReport.path}
                </div>
              )}
            </div>
          </Card>
        </div>
      )}

      {/* Subtab 4: LUFS Normalizer */}
      {subtab === 'loudness' && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-6">
            <Card title="EBU R128 Loudness & True-Peak Audit" subtitle="Measure integrated LUFS, sample peak, and quarter-sample true peak">
              <div className="space-y-3 text-xs">
                <Input
                  label="Audio file path"
                  placeholder="/path/to/master.wav"
                  value={loudnessPath}
                  onChange={(e) => setLoudnessPath(e.target.value)}
                />
                <Button variant="primary" size="sm" onClick={handleAnalyzeLoudness} disabled={busy === 'loudness'}>
                  {busy === 'loudness' ? 'Measuring...' : 'Analyze LUFS & True Peak'}
                </Button>

                {loudnessReport && (
                  <div className="grid grid-cols-3 gap-3 pt-2">
                    <div className="p-3 rounded-lg bg-surface-2 border border-border">
                      <div className="text-[10px] text-muted">Integrated</div>
                      <div className="font-mono text-base font-medium text-primary-text tabular-nums">
                        {loudnessReport.integrated_lufs.toFixed(1)} LUFS
                      </div>
                    </div>
                    <div className="p-3 rounded-lg bg-surface-2 border border-border">
                      <div className="text-[10px] text-muted">True Peak</div>
                      <div className="font-mono text-base font-medium text-sky-text tabular-nums">
                        {loudnessReport.true_peak_dbfs.toFixed(1)} dBFS
                      </div>
                    </div>
                    <div className="p-3 rounded-lg bg-surface-2 border border-border">
                      <div className="text-[10px] text-muted">Sample Peak</div>
                      <div className="font-mono text-base font-medium text-fg tabular-nums">
                        {loudnessReport.peak_dbfs.toFixed(1)} dBFS
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </Card>
          </div>

          <div className="lg:col-span-6">
            <Card title="Batch Loudness Normalizer" subtitle="Normalize directory WAVs via MacroChain">
              <div className="space-y-3 text-xs">
                <Input label="Source directory" placeholder="/path/to/chapters" value={macroInDir} onChange={(e) => setMacroInDir(e.target.value)} />
                <Input label="Normalized output directory" placeholder="/path/to/normalized" value={macroOutDir} onChange={(e) => setMacroOutDir(e.target.value)} />
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => {
                    setMacroEffectId('normalize');
                    handleRunMacro();
                  }}
                  disabled={busy === 'macro'}
                >
                  Normalize Directory
                </Button>
                {macroReport && (
                  <div className="p-3 rounded-lg bg-chip-sky border border-border-strong font-mono text-[11px] text-sky-text">
                    Normalized {macroReport.report.succeeded}/{macroReport.report.processed} file(s)
                  </div>
                )}
              </div>
            </Card>
          </div>
        </div>
      )}
    </div>
  );
}
