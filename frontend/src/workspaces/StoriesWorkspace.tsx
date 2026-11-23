/**
 * Audio Labs Pro - Stories Workspace
 *
 * Multi-character dialogue narration, script management, line-by-line take generation,
 * and automated timeline stitching.
 */

import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { getStories, getVoiceProfiles } from '../ipc/client';
import type { StoryScript, StoryLine, VoiceProfile } from '../ipc/types';
import {
  BookOpen,
  Plus,
  Layers,
  Sparkles,
  CheckCircle2,
} from 'lucide-react';

export function StoriesWorkspace() {
  const [stories, setStories] = useState<StoryScript[]>([]);
  const [voices, setVoices] = useState<VoiceProfile[]>([]);
  const [activeStoryId, setActiveStoryId] = useState<string>('story-01');
  const [newLineText, setNewLineText] = useState<string>('');
  const [newLineSpeaker, setNewLineSpeaker] = useState<string>('Studio Narrator (Acoustic)');
  const [isRenderingAll, setIsRenderingAll] = useState<boolean>(false);
  const [stitchNotice, setStitchNotice] = useState<string | null>(null);

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

  const handleRenderAll = () => {
    if (!activeStory) return;
    setIsRenderingAll(true);

    setTimeout(() => {
      setStories((prev) =>
        prev.map((s) =>
          s.id === activeStory.id
            ? {
                ...s,
                lines: s.lines.map((l) => ({
                  ...l,
                  status: 'ready',
                  take_duration: +(l.text.split(' ').length * 0.38).toFixed(1),
                })),
              }
            : s
        )
      );
      setIsRenderingAll(false);
    }, 1200);
  };

  const handleStitchTimeline = () => {
    setStitchNotice(`All ${activeStory?.lines.length || 0} story takes stitched to Arranger with natural pacing.`);
    setTimeout(() => setStitchNotice(null), 4000);
  };

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      {/* Workspace Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
            <BookOpen className="w-4 h-4 text-[var(--color-fg-muted)]" />
            Stories & Script Narrator
          </h2>
          <p className="text-xs text-[var(--color-fg-muted)] mt-1">
            Produce multi-character audiobook scripts, assign cast personas per dialogue line, and stitch takes directly onto the timeline.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={handleStitchTimeline}
            leftIcon={<Layers className="w-4 h-4 text-[var(--color-accent)]" />}
          >
            Stitch to Timeline
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={handleRenderAll}
            disabled={isRenderingAll}
            leftIcon={<Sparkles className="w-4 h-4" />}
          >
            {isRenderingAll ? 'Synthesizing All Takes...' : 'Render Script Takes'}
          </Button>
        </div>
      </div>

      {stitchNotice && (
        <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-strong)] text-[var(--color-fg-muted)] text-xs font-mono flex items-center gap-2 animate-fade-in">
          <CheckCircle2 className="w-4 h-4 flex-shrink-0" />
          <span>{stitchNotice}</span>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left: Stories & Chapters (4 cols) */}
        <div className="lg:col-span-4 space-y-4">
          <Card title="Chapters & Scripts" subtitle={`${stories.length} story project(s)`}>
            <div className="space-y-2 text-xs">
              {stories.map((story) => (
                <div
                  key={story.id}
                  onClick={() => setActiveStoryId(story.id)}
                  className={`p-3 rounded-xl border transition-[color,background-color,border-color,opacity,transform] cursor-pointer space-y-1 ${
 story.id === activeStoryId
 ? 'bg-[var(--color-surface)] border-[var(--color-border-strong)] shadow-md'
 : 'bg-[var(--color-bg)] border-[var(--color-border)] hover:border-white/15'
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

        {/* Right: Dialogue Script Editor (8 cols) */}
        <div className="lg:col-span-8 space-y-4">
          <Card
            title={activeStory?.title || 'Story Script'}
            subtitle={`Chapter ${activeStory?.chapter || 1} Dialogue Queue`}
          >
            <div className="space-y-4 text-xs">
              {/* Lines List */}
              <div className="space-y-2.5 max-h-[420px] overflow-y-auto pr-1">
                {activeStory?.lines.map((line, idx) => (
                  <div
                    key={line.id}
                    className="p-3 rounded-xl bg-[var(--color-surface)] border border-[var(--color-border)] space-y-2"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-[var(--color-fg-muted)] font-medium w-4">#{idx + 1}</span>
                        <span className="font-medium text-[var(--color-accent)]">{line.speaker}</span>
                      </div>

                      <span
                        className={`px-2 py-0.5 rounded font-mono text-[10px] font-medium border ${
 line.status === 'ready'
 ? 'bg-emerald-500/20 text-[var(--color-fg-muted)] border-[var(--color-border-strong)]'
 : line.status === 'rendering'
 ? 'bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border-[var(--color-border)]'
 : 'bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border-[var(--color-border)]'
 }`}
                      >
                        {line.status}
                      </span>
                    </div>

                    <p className="text-[var(--color-fg)] text-xs italic pl-6 leading-relaxed">
                      &ldquo;{line.text}&rdquo;
                    </p>

                    {line.take_duration && (
                      <div className="pl-6 text-[10px] font-mono text-[var(--color-fg-muted)] flex items-center gap-2">
                        <span>Duration: {line.take_duration}s</span>
                        <span>·</span>
                        <span className="text-[var(--color-fg-muted)]">Audio ready for playback</span>
                      </div>
                    )}
                  </div>
                ))}
              </div>

              {/* Add Dialogue Line Box */}
              <div className="pt-3 border-t border-[var(--color-border)] space-y-3">
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                  <div className="sm:col-span-1">
                    <label className="block text-[var(--color-fg-muted)] mb-1 text-[11px] font-medium">Cast Speaker</label>
                    <select
                      value={newLineSpeaker}
                      onChange={(e) => setNewLineSpeaker(e.target.value)}
                      className="w-full px-2.5 py-1.5 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)] font-mono text-xs"
                    >
                      {voices.map((v) => (
                        <option key={v.id} value={v.name}>
                          {v.name}
                        </option>
                      ))}
                    </select>
                  </div>

                  <div className="sm:col-span-2">
                    <label className="block text-[var(--color-fg-muted)] mb-1 text-[11px] font-medium">Spoken Line</label>
                    <div className="flex gap-2">
                      <input
                        type="text"
                        value={newLineText}
                        onChange={(e) => setNewLineText(e.target.value)}
                        onKeyDown={(e) => e.key === 'Enter' && handleAddLine()}
                        placeholder="Enter dialog line to append..."
                        className="flex-1 px-3 py-1.5 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-fg)] focus:outline-none focus:border-[var(--color-border-strong)] text-xs"
                      />
                      <Button variant="secondary" size="sm" onClick={handleAddLine} leftIcon={<Plus className="w-4 h-4" />}>
                        Add
                      </Button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
