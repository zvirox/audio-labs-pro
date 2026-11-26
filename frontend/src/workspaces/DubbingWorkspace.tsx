/**
 * Audio Labs Pro - Dubbing Studio
 *
 * Video sync, four-stem isolation, ADR rate fitting and stream-copy remux.
 * Uses the locked Linear UI kit and tokens only.
 */

import { useState } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Input } from '../kit/Input';
import { separateStems, fitRate, remuxVideo } from '../ipc/client';
import type {
  StemSeparationReport,
  RateFitReport,
  RemuxReport,
} from '../ipc/types';
import { Film, Layers, SlidersHorizontal, Captions } from 'lucide-react';

export interface DubbingWorkspaceProps {
  subtab?: string;
}

export function DubbingWorkspace({ subtab }: DubbingWorkspaceProps) {
  const [mixedPath, setMixedPath] = useState('');
  const [stemDir, setStemDir] = useState('');
  const [stems, setStems] = useState<StemSeparationReport | null>(null);

  const [takePath, setTakePath] = useState('');
  const [targetDuration, setTargetDuration] = useState('');
  const [fitOutput, setFitOutput] = useState('');
  const [fitReport, setFitReport] = useState<RateFitReport | null>(null);

  const [videoPath, setVideoPath] = useState('');
  const [dubAudioPath, setDubAudioPath] = useState('');
  const [remuxOutput, setRemuxOutput] = useState('');
  const [subtitlePath, setSubtitlePath] = useState('');
  const [remuxReport, setRemuxReport] = useState<RemuxReport | null>(null);

  const [busy, setBusy] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const message = (error: unknown) =>
    error instanceof Error ? error.message : String(error);

  const handleSeparate = async () => {
    if (!mixedPath.trim() || !stemDir.trim()) {
      setNotice('Set a mixed audio path and stem output directory first');
      return;
    }
    setBusy('stems');
    setNotice(null);
    try {
      setStems(
        await separateStems({
          audio_path: mixedPath.trim(),
          output_dir: stemDir.trim(),
        })
      );
    } catch (error) {
      setNotice(`Stem separation failed: ${message(error)}`);
    } finally {
      setBusy(null);
    }
  };

  const handleFit = async () => {
    const target = Number(targetDuration);
    if (!takePath.trim() || !fitOutput.trim() || !Number.isFinite(target) || target <= 0) {
      setNotice('Set a take path, output path and positive target duration first');
      return;
    }
    setBusy('fit');
    setNotice(null);
    try {
      setFitReport(
        await fitRate({
          audio_path: takePath.trim(),
          target_duration_secs: target,
          output_path: fitOutput.trim(),
        })
      );
    } catch (error) {
      setNotice(`Rate fitting failed: ${message(error)}`);
    } finally {
      setBusy(null);
    }
  };

  const handleRemux = async () => {
    if (!videoPath.trim() || !remuxOutput.trim()) {
      setNotice('Set a source video and output path first');
      return;
    }
    setBusy('remux');
    setNotice(null);
    try {
      setRemuxReport(
        await remuxVideo({
          video_path: videoPath.trim(),
          audio_path: dubAudioPath.trim() || null,
          output_path: remuxOutput.trim(),
          subtitle_path: subtitlePath.trim() || null,
        })
      );
    } catch (error) {
      setNotice(`Remux failed: ${message(error)}`);
    } finally {
      setBusy(null);
    }
  };

  const visible = (key: string) => !subtab || subtab === key || subtab === 'video_sync';

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      <div>
        <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
          <Film className="w-4 h-4 text-[var(--color-fg-muted)]" />
          Dubbing Studio
        </h2>
        <p className="text-xs text-[var(--color-fg-muted)] mt-1">
          Isolate stems, fit translated takes to dialogue windows, and remux video without re-encoding
        </p>
      </div>

      {notice && (
        <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-strong)] text-amber text-xs font-mono">
          {notice}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {visible('stem_sep') && (
          <Card title="Stem Isolation" subtitle="Four-stem ONNX separation">
            <div className="space-y-3 text-xs">
              <Input
                label="Mixed audio"
                placeholder="/path/to/mix.wav"
                value={mixedPath}
                onChange={(event) => setMixedPath(event.target.value)}
              />
              <Input
                label="Stem output directory"
                placeholder="/path/to/stems"
                value={stemDir}
                onChange={(event) => setStemDir(event.target.value)}
              />
              <Button
                variant="secondary"
                size="xs"
                onClick={handleSeparate}
                disabled={busy === 'stems'}
                leftIcon={<Layers className="w-4 h-4" />}
              >
                {busy === 'stems' ? 'Separating...' : 'Separate 4 Stems'}
              </Button>
              {stems && (
                <div className="space-y-1 pt-2 border-t border-[var(--color-border)]">
                  <div className="font-mono text-[10px] text-[var(--color-fg-muted)]">
                    {stems.sample_rate} Hz · 4 stems
                  </div>
                  {stems.stems.map((stem) => (
                    <div
                      key={stem.name}
                      className="p-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] flex items-center justify-between gap-2"
                    >
                      <span className="font-medium text-[var(--color-fg)]">{stem.name}</span>
                      <span className="font-mono text-[10px] text-[var(--color-fg-muted)] truncate" title={stem.path}>
                        {stem.path}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </Card>
        )}

        {visible('adr') && (
          <Card title="ADR Dialog Fitting" subtitle="TTS speed then WSOLA stretch">
            <div className="space-y-3 text-xs">
              <Input
                label="Rendered take"
                placeholder="/path/to/take.wav"
                value={takePath}
                onChange={(event) => setTakePath(event.target.value)}
              />
              <Input
                label="Target window (seconds)"
                placeholder="3.2"
                value={targetDuration}
                onChange={(event) => setTargetDuration(event.target.value)}
              />
              <Input
                label="Fitted output"
                placeholder="/path/to/fitted.wav"
                value={fitOutput}
                onChange={(event) => setFitOutput(event.target.value)}
              />
              <Button
                variant="secondary"
                size="xs"
                onClick={handleFit}
                disabled={busy === 'fit'}
                leftIcon={<SlidersHorizontal className="w-4 h-4" />}
              >
                {busy === 'fit' ? 'Fitting...' : 'Fit Take'}
              </Button>
              {fitReport && (
                <div className="pt-2 border-t border-[var(--color-border)] font-mono text-[10px] text-[var(--color-fg-muted)] space-y-1">
                  <div>
                    target {fitReport.plan.target_duration_secs.toFixed(2)}s · result {fitReport.duration_secs.toFixed(2)}s
                  </div>
                  <div>
                    speed {fitReport.plan.recommended_speed.toFixed(2)}x · stretch {fitReport.plan.stretch_ratio.toFixed(2)}x
                  </div>
                  <div>
                    {fitReport.plan.within_tts_range ? 'TTS range' : 'WSOLA required'}
                  </div>
                  <div className="truncate" title={fitReport.path}>{fitReport.path}</div>
                </div>
              )}
            </div>
          </Card>
        )}

        {visible('burn_in') && (
          <Card title="Subtitles & Remux" subtitle="Stream-copy video with dubbed audio">
            <div className="space-y-3 text-xs">
              <Input
                label="Source video"
                placeholder="/path/to/source.mp4"
                value={videoPath}
                onChange={(event) => setVideoPath(event.target.value)}
              />
              <Input
                label="Dubbed audio (optional)"
                placeholder="/path/to/dub.wav"
                value={dubAudioPath}
                onChange={(event) => setDubAudioPath(event.target.value)}
              />
              <Input
                label="Subtitle side file (optional)"
                placeholder="/path/to/subtitles.srt"
                value={subtitlePath}
                onChange={(event) => setSubtitlePath(event.target.value)}
              />
              <Input
                label="Output video"
                placeholder="/path/to/dubbed.mp4"
                value={remuxOutput}
                onChange={(event) => setRemuxOutput(event.target.value)}
              />
              <Button
                variant="secondary"
                size="xs"
                onClick={handleRemux}
                disabled={busy === 'remux'}
                leftIcon={<Captions className="w-4 h-4" />}
              >
                {busy === 'remux' ? 'Remuxing...' : 'Remux (stream copy)'}
              </Button>
              {remuxReport && (
                <div className="pt-2 border-t border-[var(--color-border)] font-mono text-[10px] text-[var(--color-fg-muted)] space-y-1">
                  <div>
                    {remuxReport.video_stream_copied ? 'video copied' : 'video re-encoded'} · {remuxReport.audio_codec} audio
                  </div>
                  <div className="truncate" title={remuxReport.output_path}>{remuxReport.output_path}</div>
                </div>
              )}
            </div>
          </Card>
        )}
      </div>
    </div>
  );
}
