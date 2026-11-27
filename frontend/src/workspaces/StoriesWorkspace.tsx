/**
 * Audio Labs Pro - Audiobook Lab Workspace
 *
 * Dedicated subtabs:
 * - `script`: Chapter script editor & real speech synthesis per line
 * - `characters`: Cast & voice persona assignments across lines
 * - `acx_qa`: Multi-chapter ACX Loudness / True-Peak / Noise Floor QA runner
 * - `m4b_export`: Native AAC-LC M4B packaging with chapter table & metadata
 */

import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Input } from '../kit/Input';
import {
  analyzeAcxChapters,
  desktopTtsSynthesize,
  exportAudiobookM4b,
  getStories,
  getVoiceProfiles,
} from '../ipc/client';
import type {
  StoryScript,
  StoryLine,
  VoiceProfile,
  AcxChapterFileDto,
  AcxQaReport,
  M4bExportReport,
} from '../ipc/types';
import {
  BookOpen,
  Plus,
  Sparkles,
  CheckCircle2,
  ShieldCheck,
  FileAudio,
  Users,
  Trash2,
} from 'lucide-react';

export interface StoriesWorkspaceProps {
  subtab?: string;
}

export function StoriesWorkspace({ subtab = 'script' }: StoriesWorkspaceProps) {
  const [stories, setStories] = useState<StoryScript[]>([]);
  const [voices, setVoices] = useState<VoiceProfile[]>([]);
  const [activeStoryId, setActiveStoryId] = useState<string>('story-01');
  const [newLineText, setNewLineText] = useState<string>('');
  const [newLineSpeaker, setNewLineSpeaker] = useState<string>('Studio Narrator (Acoustic)');
  const [isRenderingAll, setIsRenderingAll] = useState<boolean>(false);
  const [stitchNotice, setStitchNotice] = useState<string | null>(null);

  // ACX QA Batch state
  const [acxChapters, setAcxChapters] = useState<AcxChapterFileDto[]>([
    { title: 'Chapter 1', audio_path: '' },
  ]);
  const [acxReport, setAcxReport] = useState<AcxQaReport | null>(null);
  const [isChecking, setIsChecking] = useState<boolean>(false);

  // M4B Packaging state
  const [masterPath, setMasterPath] = useState<string>('');
  const [outputM4bPath, setOutputM4bPath] = useState<string>('');
  const [coverPath, setCoverPath] = useState<string>('');
  const [bookNarrator, setBookNarrator] = useState<string>('');
  const [bookSeries, setBookSeries] = useState<string>('');
  const [m4bReport, setM4bReport] = useState<M4bExportReport | null>(null);
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [masterNotice, setMasterNotice] = useState<string | null>(null);

  useEffect(() => {
    getStories().then((s) => {
      setStories(s);
      if (s.length > 0) setActiveStoryId(s[0].id);
    });
    getVoiceProfiles().then((v) => {
      setVoices(v);
      if (v.length > 0) setNewLineSpeaker(v[0].name);
    });
  }, []);

  const activeStory = stories.find((s) => s.id === activeStoryId) || stories[0];

  const handleAddLine = () => {
    if (!newLineText.trim() || !activeStory) return;
    const newLine: StoryLine = {
      id: `l-${Date.now()}`,
      speaker: newLineSpeaker,
      text: newLineText.trim(),
      status: 'pending',
    };
    setStories((prev) =>
      prev.map((s) =>
        s.id === activeStory.id ? { ...s, lines: [...s.lines, newLine] } : s
      )
    );
    setNewLineText('');
  };

  const handleRenderAll = async () => {
    if (!activeStory) return;
    setIsRenderingAll(true);
    setStitchNotice(null);
    let rendered = 0;
    for (const line of activeStory.lines) {
      const matchedVoice =
        voices.find((v) => v.name === line.speaker)?.id || voices[0]?.id || 'builtin-narrator';
      try {
        const res = await desktopTtsSynthesize(matchedVoice, line.text, 1.0);
        rendered += 1;
        setStories((prev) =>
          prev.map((s) =>
            s.id === activeStory.id
              ? {
                  ...s,
                  lines: s.lines.map((l) =>
                    l.id === line.id
                      ? { ...l, status: 'ready', take_duration: +res.duration_secs.toFixed(1) }
                      : l
                  ),
                }
              : s
          )
        );
      } catch {
        setStories((prev) =>
          prev.map((s) =>
            s.id === activeStory.id
              ? {
                  ...s,
                  lines: s.lines.map((l) => (l.id === line.id ? { ...l, status: 'failed' } : l)),
                }
              : s
          )
        );
      }
    }
    setIsRenderingAll(false);
    setStitchNotice(`Synthesized ${rendered}/${activeStory.lines.length} script take(s) via alp-voice.`);
  };

  const handleAcxCheck = async () => {
    const valid = acxChapters.filter((c) => c.audio_path.trim().length > 0);
    if (valid.length === 0) {
      setMasterNotice('Provide at least one rendered chapter WAV path to check.');
      return;
    }
    setIsChecking(true);
    setMasterNotice(null);
    try {
      setAcxReport(
        await analyzeAcxChapters(
          valid.map((c) => ({ title: c.title.trim() || 'Chapter', audio_path: c.audio_path.trim() }))
        )
      );
    } catch (error) {
      setMasterNotice(`ACX check failed: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsChecking(false);
    }
  };

  const bookChapters = () => {
    if (!activeStory) return [];
    const duration =
      activeStory.lines.reduce((total, line) => total + (line.take_duration || 0), 0) ||
      activeStory.lines.length * 4;
    return [{ title: activeStory.title, start_secs: 0, duration_secs: duration }];
  };

  const handleExportM4b = async () => {
    if (!activeStory || !masterPath.trim()) {
      setMasterNotice('Set a rendered master WAV path first');
      return;
    }
    const targetM4b =
      outputM4bPath.trim() || masterPath.trim().replace(/\.[^.]+$/, '') + '.m4b';
    setIsExporting(true);
    setMasterNotice(null);
    try {
      const report = await exportAudiobookM4b({
        output_path: targetM4b,
        master_audio_path: masterPath.trim(),
        chapters: bookChapters(),
        cover_path: coverPath.trim() || null,
        metadata: {
          title: activeStory.title,
          narrator: bookNarrator.trim(),
          series: bookSeries.trim(),
          series_number: null,
          comment: '',
        },
      });
      setM4bReport(report);
    } catch (error) {
      setMasterNotice(`M4B export failed: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsExporting(false);
    }
  };

  const castSummary = voices.map((v) => ({
    voice: v,
    lineCount: (activeStory?.lines || []).filter((l) => l.speaker === v.name).length,
  }));

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
            {subtab === 'characters' ? (
              <Users className="w-4 h-4 text-[var(--color-accent)]" />
            ) : subtab === 'acx_qa' ? (
              <ShieldCheck className="w-4 h-4 text-[var(--color-accent)]" />
            ) : subtab === 'm4b_export' ? (
              <FileAudio className="w-4 h-4 text-[var(--color-accent)]" />
            ) : (
              <BookOpen className="w-4 h-4 text-[var(--color-accent)]" />
            )}
            {subtab === 'script' && 'Chapter Script & Narration Queue'}
            {subtab === 'characters' && 'Cast & Voice Assignments'}
            {subtab === 'acx_qa' && 'ACX Broadcast & Audiobook Compliance QA'}
            {subtab === 'm4b_export' && 'Native M4B Audiobook Packager'}
          </h2>
          <p className="text-xs text-[var(--color-fg-muted)] mt-1">
            {subtab === 'script' && 'Edit multi-speaker chapters and synthesize spoken takes line by line.'}
            {subtab === 'characters' && 'Assign local voice personas to narrative roles and inspect speaker coverage.'}
            {subtab === 'acx_qa' && 'Validate chapter RMS (-23..-18 dBFS), true peak (<= -3.0 dBFS), and noise floor (<= -60 dBFS).'}
            {subtab === 'm4b_export' && 'Encode chaptered AAC-LC M4B audiobook packages with cover artwork and ID3/MP4 tags.'}
          </p>
        </div>

        {subtab === 'script' && (
          <div className="flex items-center gap-2">
            <Button
              variant="primary"
              size="sm"
              onClick={handleRenderAll}
              disabled={isRenderingAll}
              leftIcon={<Sparkles className="w-4 h-4" />}
            >
              {isRenderingAll ? 'Synthesizing Takes...' : 'Render Script Takes'}
            </Button>
          </div>
        )}
      </div>

      {stitchNotice && (
        <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-strong)] text-sky-text text-xs font-mono flex items-center gap-2">
          <CheckCircle2 className="w-4 h-4 flex-shrink-0" />
          <span>{stitchNotice}</span>
        </div>
      )}

      {masterNotice && (
        <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-strong)] text-amber text-xs font-mono">
          {masterNotice}
        </div>
      )}

      {/* Subtab 1: Chapter Script */}
      {subtab === 'script' && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-4 space-y-4">
            <Card title="Chapters & Scripts" subtitle={`${stories.length} story project(s)`}>
              <div className="space-y-2 text-xs">
                {stories.map((story) => (
                  <div
                    key={story.id}
                    onClick={() => setActiveStoryId(story.id)}
                    className={`p-3 rounded-lg border transition-colors cursor-pointer space-y-1 ${
                      story.id === activeStoryId
                        ? 'bg-[var(--bg-surface)] border-[var(--border-strong)]'
                        : 'bg-[var(--bg-app)] border-[var(--border)] hover:border-[var(--border-strong)]'
                    }`}
                  >
                    <div className="font-medium text-[var(--color-fg)]">{story.title}</div>
                    <div className="text-[11px] font-mono text-[var(--color-fg-muted)]">
                      Chapter {story.chapter} · {story.lines.length} dialogue lines
                    </div>
                  </div>
                ))}
              </div>
            </Card>
          </div>

          <div className="lg:col-span-8">
            <Card
              title={activeStory?.title || 'Story Script'}
              subtitle={`Chapter ${activeStory?.chapter || 1} Dialogue Queue`}
            >
              <div className="space-y-4 text-xs">
                <div className="space-y-2.5 max-h-[420px] overflow-y-auto pr-1">
                  {activeStory?.lines.map((line, idx) => (
                    <div
                      key={line.id}
                      className="p-3 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] space-y-2"
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <span className="font-mono text-muted font-medium w-4">#{idx + 1}</span>
                          <span className="font-medium text-primary-text">{line.speaker}</span>
                        </div>
                        <span
                          className={`px-2 py-0.5 rounded font-mono text-[10px] font-medium border ${
                            line.status === 'ready'
                              ? 'bg-chip-sky text-sky-text border-border-strong'
                              : line.status === 'failed'
                              ? 'bg-chip-red text-red-text border-red'
                              : 'bg-surface-2 text-muted border-border'
                          }`}
                        >
                          {line.status}
                        </span>
                      </div>
                      <p className="text-[var(--color-fg)] text-xs italic pl-6 leading-relaxed">
                        &ldquo;{line.text}&rdquo;
                      </p>
                      {line.take_duration && (
                        <div className="pl-6 text-[10px] font-mono text-[var(--color-fg-muted)]">
                          Duration: {line.take_duration}s · Synthesized WAV ready
                        </div>
                      )}
                    </div>
                  ))}
                </div>

                <div className="pt-3 border-t border-[var(--color-border)] grid grid-cols-1 sm:grid-cols-3 gap-2">
                  <div>
                    <label className="block text-[var(--color-fg-muted)] mb-1 text-[11px] font-medium">
                      Cast Speaker
                    </label>
                    <select
                      value={newLineSpeaker}
                      onChange={(e) => setNewLineSpeaker(e.target.value)}
                      className="w-full px-2.5 py-1.5 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] font-mono text-xs"
                    >
                      {voices.map((v) => (
                        <option key={v.id} value={v.name}>
                          {v.name}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div className="sm:col-span-2">
                    <label className="block text-[var(--color-fg-muted)] mb-1 text-[11px] font-medium">
                      Spoken Line
                    </label>
                    <div className="flex gap-2">
                      <input
                        type="text"
                        value={newLineText}
                        onChange={(e) => setNewLineText(e.target.value)}
                        onKeyDown={(e) => e.key === 'Enter' && handleAddLine()}
                        placeholder="Enter dialog line to append..."
                        className="flex-1 px-3 py-1.5 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] text-xs"
                      />
                      <Button variant="secondary" size="sm" onClick={handleAddLine} leftIcon={<Plus className="w-4 h-4" />}>
                        Add
                      </Button>
                    </div>
                  </div>
                </div>
              </div>
            </Card>
          </div>
        </div>
      )}

      {/* Subtab 2: Cast & Voices */}
      {subtab === 'characters' && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-5">
            <Card title="Cast Persona Roster" subtitle={`${voices.length} registered character voice(s)`}>
              <div className="space-y-2.5 text-xs">
                {castSummary.map(({ voice, lineCount }) => (
                  <div
                    key={voice.id}
                    className="p-3 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] flex items-center justify-between"
                  >
                    <div>
                      <div className="font-medium text-fg">{voice.name}</div>
                      <div className="font-mono text-[10px] text-muted">
                        {voice.kind} · speed {voice.speed.toFixed(2)}x · {voice.emotion}
                      </div>
                    </div>
                    <span className="px-2 py-0.5 rounded bg-primary-tint text-primary-text font-mono text-[11px] border border-border-strong">
                      {lineCount} line(s)
                    </span>
                  </div>
                ))}
              </div>
            </Card>
          </div>

          <div className="lg:col-span-7">
            <Card title="Line-to-Speaker Casting" subtitle={`Reassign voice personas inside ${activeStory?.title || 'chapter'}`}>
              <div className="space-y-2.5 text-xs">
                {activeStory?.lines.map((line, idx) => (
                  <div
                    key={line.id}
                    className="p-3 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] flex flex-col sm:flex-row sm:items-center justify-between gap-3"
                  >
                    <div className="min-w-0">
                      <div className="font-mono text-[10px] text-muted">Line #{idx + 1}</div>
                      <div className="text-fg truncate">&ldquo;{line.text}&rdquo;</div>
                    </div>
                    <select
                      value={line.speaker}
                      onChange={(e) => {
                        const speaker = e.target.value;
                        setStories((prev) =>
                          prev.map((s) =>
                            s.id === activeStory.id
                              ? {
                                  ...s,
                                  lines: s.lines.map((l) => (l.id === line.id ? { ...l, speaker } : l)),
                                }
                              : s
                          )
                        );
                      }}
                      className="px-2.5 py-1.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] text-fg font-mono text-xs shrink-0"
                    >
                      {voices.map((v) => (
                        <option key={v.id} value={v.name}>
                          {v.name}
                        </option>
                      ))}
                    </select>
                  </div>
                ))}
              </div>
            </Card>
          </div>
        </div>
      )}

      {/* Subtab 3: ACX Compliance QA */}
      {subtab === 'acx_qa' && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-5">
            <Card title="Chapter Batch Inputs" subtitle="ACX Targets: RMS -23..-18 dBFS · True Peak <= -3.0 dBFS · Floor <= -60 dBFS">
              <div className="space-y-3 text-xs">
                {acxChapters.map((ch, idx) => (
                  <div key={idx} className="p-3 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] space-y-2">
                    <div className="flex items-center justify-between">
                      <span className="font-mono text-[11px] text-muted">Chapter #{idx + 1}</span>
                      {acxChapters.length > 1 && (
                        <button
                          type="button"
                          onClick={() => setAcxChapters((prev) => prev.filter((_, i) => i !== idx))}
                          className="text-red-text cursor-pointer"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>
                    <Input
                      label="Title"
                      value={ch.title}
                      onChange={(e) =>
                        setAcxChapters((prev) =>
                          prev.map((item, i) => (i === idx ? { ...item, title: e.target.value } : item))
                        )
                      }
                    />
                    <Input
                      label="Audio WAV path"
                      placeholder="/path/to/chapter-01.wav"
                      value={ch.audio_path}
                      onChange={(e) =>
                        setAcxChapters((prev) =>
                          prev.map((item, i) => (i === idx ? { ...item, audio_path: e.target.value } : item))
                        )
                      }
                    />
                  </div>
                ))}

                <div className="flex items-center gap-2 pt-1">
                  <Button
                    variant="secondary"
                    size="xs"
                    leftIcon={<Plus className="w-3.5 h-3.5" />}
                    onClick={() =>
                      setAcxChapters((prev) => [
                        ...prev,
                        { title: `Chapter ${prev.length + 1}`, audio_path: '' },
                      ])
                    }
                  >
                    Add Chapter
                  </Button>
                  <Button
                    variant="primary"
                    size="xs"
                    onClick={handleAcxCheck}
                    disabled={isChecking}
                    leftIcon={<ShieldCheck className="w-4 h-4" />}
                  >
                    {isChecking ? 'Analyzing...' : 'Run ACX Compliance Check'}
                  </Button>
                </div>
              </div>
            </Card>
          </div>

          <div className="lg:col-span-7">
            <Card
              title="ACX QA Inspection Results"
              subtitle={
                acxReport
                  ? `${acxReport.compliant_count}/${acxReport.total_count} chapters passed all ACX gates`
                  : 'Run an inspection to compute RMS, quarter-sample true peak, and noise floor'
              }
            >
              {!acxReport ? (
                <div className="p-8 text-center text-xs text-muted italic">
                  Add rendered chapter WAV files and click &ldquo;Run ACX Compliance Check&rdquo;.
                </div>
              ) : (
                <div className="space-y-2.5 text-xs">
                  {acxReport.chapters.map((chapter) => (
                    <div
                      key={chapter.title}
                      className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] space-y-1.5"
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-medium text-[var(--color-fg)]">{chapter.title}</span>
                        <span
                          className={
                            chapter.compliant
                              ? 'px-2 py-0.5 rounded bg-chip-sky text-sky-text font-mono text-[10px]'
                              : 'px-2 py-0.5 rounded bg-chip-red text-red-text font-mono text-[10px]'
                          }
                        >
                          {chapter.compliant ? 'ACX PASS' : 'NEEDS FIX'}
                        </span>
                      </div>
                      <div className="font-mono text-[11px] text-[var(--color-fg-muted)] tabular-nums">
                        RMS: {chapter.rms_dbfs.toFixed(1)} dBFS · True Peak: {chapter.true_peak_dbfs.toFixed(1)} dBFS · Noise Floor: {chapter.noise_floor_dbfs.toFixed(1)} dBFS
                      </div>
                      {chapter.failures.map((f) => (
                        <div key={f} className="text-[11px] text-amber">
                          • {f}
                        </div>
                      ))}
                    </div>
                  ))}
                </div>
              )}
            </Card>
          </div>
        </div>
      )}

      {/* Subtab 4: Native M4B Package */}
      {subtab === 'm4b_export' && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-6">
            <Card title="M4B Container & Metadata" subtitle="Pure-Rust AAC-LC + chpl chapter atoms">
              <div className="space-y-3 text-xs">
                <Input
                  label="Rendered master WAV"
                  placeholder="/path/to/book-master.wav"
                  value={masterPath}
                  onChange={(e) => setMasterPath(e.target.value)}
                />
                <Input
                  label="Output .m4b path (optional)"
                  placeholder="/path/to/audiobook.m4b"
                  value={outputM4bPath}
                  onChange={(e) => setOutputM4bPath(e.target.value)}
                />
                <Input
                  label="Cover artwork JPEG / PNG (optional)"
                  placeholder="/path/to/cover.jpg"
                  value={coverPath}
                  onChange={(e) => setCoverPath(e.target.value)}
                />
                <div className="grid grid-cols-2 gap-3">
                  <Input
                    label="Narrator"
                    placeholder="Marcus Reed"
                    value={bookNarrator}
                    onChange={(e) => setBookNarrator(e.target.value)}
                  />
                  <Input
                    label="Series"
                    placeholder="Audio Labs Originals"
                    value={bookSeries}
                    onChange={(e) => setBookSeries(e.target.value)}
                  />
                </div>
                <Button
                  variant="primary"
                  size="sm"
                  onClick={handleExportM4b}
                  disabled={isExporting}
                  leftIcon={<FileAudio className="w-4 h-4" />}
                >
                  {isExporting ? 'Muxing .m4b...' : 'Build Chaptered M4B Package'}
                </Button>
              </div>
            </Card>
          </div>

          <div className="lg:col-span-6">
            <Card title="Chapter Map & Output Report" subtitle={`${bookChapters().length} chapter marker(s)`}>
              <div className="space-y-3 text-xs">
                {bookChapters().map((ch) => (
                  <div
                    key={ch.title}
                    className="p-2.5 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] flex items-center justify-between font-mono text-[11px]"
                  >
                    <span className="text-fg">{ch.title}</span>
                    <span className="text-muted">
                      start {ch.start_secs.toFixed(1)}s · len {ch.duration_secs.toFixed(1)}s
                    </span>
                  </div>
                ))}

                {m4bReport && (
                  <div className="p-3 rounded-lg bg-chip-sky border border-border-strong font-mono text-[11px] text-sky-text space-y-1">
                    <div>
                      Exported {m4bReport.chapter_count} chapter(s) · {m4bReport.frames} AAC frames ·{' '}
                      {m4bReport.duration_secs.toFixed(1)}s
                    </div>
                    <div className="truncate" title={m4bReport.path}>
                      {m4bReport.path}
                    </div>
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
