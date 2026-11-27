import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Input } from '../kit/Input';
import { Slider } from '../kit/Slider';
import { Meter } from '../kit/Meter';
import type { TimelineDto, TimelineEnvelopePointDto, TimelinePanPointDto } from '../timeline/types';
import {
  timelineAddTrack,
  timelineSetTrackGain,
  timelineSetTrackPan,
  timelineToggleTrackMute,
  timelineToggleTrackSolo,
  timelineToggleTrackArm,
  timelineSetMasterGain,
  timelineAddMarker,
  timelineRemoveMarker,
  timelineSetLoopRange,
  timelineSetClipFades,
  timelineSetClipEnvelope,
  timelineSetClipPanEnvelope,
  timelineSlipClip,
} from '../ipc/client';
import { Plus, Trash2, MapPin, Sliders, Activity, Repeat } from 'lucide-react';

interface SubtabProps {
  timeline: TimelineDto;
  currentTimeSecs: number;
  onUpdateTimeline: (next: TimelineDto) => void;
  onSeek: (secs: number) => void;
  onStatus: (msg: string) => void;
}

export function ConsoleMixerView({ timeline, onUpdateTimeline, onStatus }: SubtabProps) {
  const [newTrackName, setNewTrackName] = useState('');

  const handleAddTrack = async () => {
    const label = newTrackName.trim() || `Audio Track ${timeline.tracks.length + 1}`;
    try {
      const next = await timelineAddTrack(label);
      onUpdateTimeline(next);
      setNewTrackName('');
      onStatus(`Added track "${label}"`);
    } catch (err) {
      onStatus(`Add track failed: ${err}`);
    }
  };

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-medium text-fg flex items-center gap-2">
            <Sliders className="w-4 h-4 text-primary-text" />
            Console Mixer
          </h2>
          <p className="text-xs text-muted mt-1">
            Per-track faders, stereo pan, mute/solo/record arm, and Master Bus trim backed by undoable session commands.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <input
            type="text"
            value={newTrackName}
            onChange={(e) => setNewTrackName(e.target.value)}
            placeholder="New track name..."
            className="px-3 py-1.5 rounded-lg bg-surface border border-border text-xs text-fg focus:outline-none focus:border-border-strong"
          />
          <Button variant="primary" size="sm" onClick={handleAddTrack} leftIcon={<Plus className="w-4 h-4" />}>
            Add Track
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {timeline.tracks.map((track, idx) => (
          <Card key={track.id} title={track.name} subtitle={`${track.clips.length} clip(s) · ${track.view_mode}`}>
            <div className="space-y-4 text-xs">
              <div className="flex items-center justify-center h-20 py-1">
                <Meter
                  leftLevel={track.muted ? 0 : Math.min(0.95, track.gain * 0.65)}
                  rightLevel={track.muted ? 0 : Math.min(0.95, track.gain * 0.6)}
                  orientation="vertical"
                  showLabels={false}
                />
              </div>

              <Slider
                label="Track Gain"
                min={0}
                max={2}
                step={0.05}
                value={track.gain}
                onChange={async (val) => {
                  const next = await timelineSetTrackGain(idx, val);
                  onUpdateTimeline(next);
                }}
                formatValue={(v) => `${(v * 100).toFixed(0)}%`}
              />

              <Slider
                label="Stereo Pan"
                min={-1}
                max={1}
                step={0.05}
                value={track.pan}
                onChange={async (val) => {
                  const next = await timelineSetTrackPan(idx, val);
                  onUpdateTimeline(next);
                }}
                formatValue={(v) => (v === 0 ? 'Center' : v < 0 ? `L ${Math.round(-v * 100)}` : `R ${Math.round(v * 100)}`)}
              />

              <div className="grid grid-cols-3 gap-1.5 pt-2 border-t border-border">
                <Button
                  variant={track.muted ? 'danger' : 'secondary'}
                  size="xs"
                  onClick={async () => onUpdateTimeline(await timelineToggleTrackMute(idx))}
                >
                  {track.muted ? 'Muted' : 'Mute'}
                </Button>
                <Button
                  variant={track.solo ? 'primary' : 'secondary'}
                  size="xs"
                  onClick={async () => onUpdateTimeline(await timelineToggleTrackSolo(idx))}
                >
                  Solo
                </Button>
                <Button
                  variant={track.armed ? 'danger' : 'secondary'}
                  size="xs"
                  onClick={async () => onUpdateTimeline(await timelineToggleTrackArm(idx))}
                >
                  Arm
                </Button>
              </div>
            </div>
          </Card>
        ))}

        {/* Master Output Bus Strip */}
        <Card title="Master Output" subtitle={`${timeline.sample_rate} Hz stereo mix bus`}>
          <div className="space-y-4 text-xs">
            <div className="flex items-center justify-center h-20 py-1">
              <Meter leftLevel={0.78} rightLevel={0.75} orientation="vertical" showLabels={false} />
            </div>
            <Slider
              label="Master Trim"
              min={0}
              max={2}
              step={0.05}
              value={timeline.master_gain ?? 1.0}
              onChange={async (val) => {
                const next = await timelineSetMasterGain(val);
                onUpdateTimeline(next);
              }}
              formatValue={(v) => `${(v * 100).toFixed(0)}%`}
            />
            <div className="p-2.5 rounded-lg bg-surface-2 border border-border font-mono text-[10px] text-muted space-y-1">
              <div>Tracks: {timeline.tracks.length}</div>
              <div>Undo history: {timeline.undo_steps} steps</div>
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
}

export function CueMarkersView({ timeline, currentTimeSecs, onUpdateTimeline, onSeek, onStatus }: SubtabProps) {
  const [label, setLabel] = useState('');
  const [timeInput, setTimeInput] = useState(currentTimeSecs.toFixed(2));
  const [loopStart, setLoopStart] = useState((timeline.loop_range?.start_secs ?? 0).toFixed(2));
  const [loopEnd, setLoopEnd] = useState((timeline.loop_range?.end_secs ?? Math.max(4, timeline.duration_secs)).toFixed(2));

  useEffect(() => {
    setLoopStart((timeline.loop_range?.start_secs ?? 0).toFixed(2));
    setLoopEnd((timeline.loop_range?.end_secs ?? Math.max(4, timeline.duration_secs)).toFixed(2));
  }, [timeline.loop_range, timeline.duration_secs]);

  const handleAddMarker = async () => {
    const t = Number(timeInput);
    if (!Number.isFinite(t) || t < 0) return;
    const text = label.trim() || `Cue @ ${t.toFixed(2)}s`;
    const next = await timelineAddMarker(t, text);
    onUpdateTimeline(next);
    setLabel('');
    onStatus(`Added marker "${text}" at ${t.toFixed(2)}s`);
  };

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      <div>
        <h2 className="text-xl font-medium text-fg flex items-center gap-2">
          <MapPin className="w-4 h-4 text-primary-text" />
          Cue Markers &amp; Loop Regions
        </h2>
        <p className="text-xs text-muted mt-1">
          Drop named chapter/edit cue markers, seek directly to timestamps, and configure loop playback boundaries.
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        <div className="lg:col-span-5 space-y-6">
          <Card title="Insert Cue Marker" subtitle={`Current playhead: ${currentTimeSecs.toFixed(2)}s`}>
            <div className="space-y-3 text-xs">
              <Input
                label="Marker Label"
                placeholder="Chapter 2 Intro"
                value={label}
                onChange={(e) => setLabel(e.target.value)}
              />
              <div className="flex items-end gap-2">
                <div className="flex-1">
                  <Input
                    label="Timestamp (seconds)"
                    value={timeInput}
                    onChange={(e) => setTimeInput(e.target.value)}
                  />
                </div>
                <Button variant="secondary" size="sm" onClick={() => setTimeInput(currentTimeSecs.toFixed(2))}>
                  Use Playhead
                </Button>
              </div>
              <Button variant="primary" size="sm" onClick={handleAddMarker} leftIcon={<Plus className="w-4 h-4" />}>
                Add Cue Marker
              </Button>
            </div>
          </Card>

          <Card title="Loop Region" subtitle={timeline.loop_range ? 'Loop active on timeline' : 'Loop disabled'}>
            <div className="space-y-3 text-xs">
              <div className="grid grid-cols-2 gap-3">
                <Input label="Start (s)" value={loopStart} onChange={(e) => setLoopStart(e.target.value)} />
                <Input label="End (s)" value={loopEnd} onChange={(e) => setLoopEnd(e.target.value)} />
              </div>
              <div className="flex items-center gap-2">
                <Button
                  variant="primary"
                  size="xs"
                  leftIcon={<Repeat className="w-3.5 h-3.5" />}
                  onClick={async () => {
                    const s = Math.max(0, Number(loopStart) || 0);
                    const e = Math.max(s + 0.1, Number(loopEnd) || s + 1);
                    onUpdateTimeline(await timelineSetLoopRange(s, e, true));
                    onStatus(`Loop range set ${s.toFixed(2)}s - ${e.toFixed(2)}s`);
                  }}
                >
                  Apply Loop Range
                </Button>
                <Button
                  variant="secondary"
                  size="xs"
                  onClick={async () => {
                    onUpdateTimeline(await timelineSetLoopRange(0, 0, false));
                    onStatus('Loop range cleared');
                  }}
                >
                  Clear Loop
                </Button>
              </div>
            </div>
          </Card>
        </div>

        <div className="lg:col-span-7">
          <Card title="Session Markers" subtitle={`${timeline.markers.length} marker(s) in project`}>
            <div className="space-y-2 text-xs">
              {timeline.markers.length === 0 ? (
                <div className="p-6 text-center text-muted italic">No cue markers placed yet.</div>
              ) : (
                timeline.markers.map((marker) => (
                  <div
                    key={marker.id}
                    className="p-3 rounded-lg bg-surface-2 border border-border flex items-center justify-between gap-3"
                  >
                    <div>
                      <div className="font-medium text-fg">{marker.label}</div>
                      <div className="font-mono text-[11px] text-primary-text tabular-nums">
                        {marker.time_secs.toFixed(3)}s
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button variant="secondary" size="xs" onClick={() => onSeek(marker.time_secs)}>
                        Seek
                      </Button>
                      <Button
                        variant="ghost"
                        size="xs"
                        onClick={async () => {
                          onUpdateTimeline(await timelineRemoveMarker(marker.id));
                          onStatus(`Removed marker "${marker.label}"`);
                        }}
                      >
                        <Trash2 className="w-3.5 h-3.5 text-red-text" />
                      </Button>
                    </div>
                  </div>
                ))
              )}
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}

export function EnvelopesView({ timeline, onUpdateTimeline, onStatus }: SubtabProps) {
  const allClips = timeline.tracks.flatMap((t) =>
    t.clips.map((c) => ({ trackName: t.name, clip: c }))
  );
  const [selectedClipId, setSelectedClipId] = useState<string>(
    timeline.selected_clip_id || allClips[0]?.clip.id || ''
  );

  useEffect(() => {
    if (!selectedClipId && allClips.length > 0) {
      setSelectedClipId(allClips[0].clip.id);
    }
  }, [selectedClipId, allClips]);

  const activeEntry = allClips.find((e) => e.clip.id === selectedClipId) || allClips[0];
  const clip = activeEntry?.clip;

  const [fadeIn, setFadeIn] = useState(clip?.fade_in_secs ?? 0);
  const [fadeOut, setFadeOut] = useState(clip?.fade_out_secs ?? 0);
  const [gainPoints, setGainPoints] = useState<TimelineEnvelopePointDto[]>(clip?.envelope ?? []);
  const [panPoints, setPanPoints] = useState<TimelinePanPointDto[]>(clip?.pan_envelope ?? []);

  useEffect(() => {
    if (clip) {
      setFadeIn(clip.fade_in_secs);
      setFadeOut(clip.fade_out_secs);
      setGainPoints(clip.envelope ?? []);
      setPanPoints(clip.pan_envelope ?? []);
    }
  }, [clip?.id, clip?.fade_in_secs, clip?.fade_out_secs, clip?.envelope, clip?.pan_envelope]);

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      <div>
        <h2 className="text-xl font-medium text-fg flex items-center gap-2">
          <Activity className="w-4 h-4 text-primary-text" />
          Clip Envelopes, Fades &amp; Stereo Pan Automation
        </h2>
        <p className="text-xs text-muted mt-1">
          Shape per-clip gain fades, non-destructive source slip, volume keyframes, and stereo pan breakpoints.
        </p>
      </div>

      {!clip ? (
        <Card title="No Clips Available" subtitle="Add a clip on the Timeline Arranger first">
          <p className="text-xs text-muted">No audio clips exist in the active timeline.</p>
        </Card>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-5 space-y-6">
            <Card title="Target Clip" subtitle={`${activeEntry.trackName} · ${clip.duration_secs.toFixed(2)}s`}>
              <div className="space-y-4 text-xs">
                <select
                  value={clip.id}
                  onChange={(e) => setSelectedClipId(e.target.value)}
                  className="w-full px-3 py-2 rounded-lg bg-surface border border-border text-fg font-mono text-xs"
                >
                  {allClips.map((entry) => (
                    <option key={entry.clip.id} value={entry.clip.id}>
                      {entry.trackName} — {entry.clip.name} ({entry.clip.timeline_start_secs.toFixed(1)}s)
                    </option>
                  ))}
                </select>

                <Slider
                  label="Fade In"
                  min={0}
                  max={Math.max(0.5, clip.duration_secs * 0.5)}
                  step={0.05}
                  value={fadeIn}
                  onChange={setFadeIn}
                  unit="s"
                />
                <Slider
                  label="Fade Out"
                  min={0}
                  max={Math.max(0.5, clip.duration_secs * 0.5)}
                  step={0.05}
                  value={fadeOut}
                  onChange={setFadeOut}
                  unit="s"
                />

                <div className="flex items-center gap-2">
                  <Button
                    variant="primary"
                    size="xs"
                    onClick={async () => {
                      onUpdateTimeline(await timelineSetClipFades(clip.id, fadeIn, fadeOut));
                      onStatus(`Updated clip fades (${fadeIn.toFixed(2)}s / ${fadeOut.toFixed(2)}s)`);
                    }}
                  >
                    Save Fades
                  </Button>
                  <Button
                    variant="secondary"
                    size="xs"
                    onClick={async () => {
                      onUpdateTimeline(await timelineSlipClip(clip.id, -0.05));
                      onStatus('Slipped source audio -50ms');
                    }}
                  >
                    Slip -50ms
                  </Button>
                  <Button
                    variant="secondary"
                    size="xs"
                    onClick={async () => {
                      onUpdateTimeline(await timelineSlipClip(clip.id, 0.05));
                      onStatus('Slipped source audio +50ms');
                    }}
                  >
                    Slip +50ms
                  </Button>
                </div>
              </div>
            </Card>
          </div>

          <div className="lg:col-span-7 space-y-6">
            <Card title="Volume Envelope Keyframes" subtitle={`${gainPoints.length} automation point(s)`}>
              <div className="space-y-3 text-xs">
                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    variant="secondary"
                    size="xs"
                    leftIcon={<Plus className="w-3.5 h-3.5" />}
                    onClick={async () => {
                      const nextPts = [
                        ...gainPoints,
                        {
                          offset_secs: +(clip.duration_secs * 0.5).toFixed(2),
                          gain: gainPoints.length % 2 === 0 ? 0.65 : 1.0,
                        },
                      ].sort((a, b) => a.offset_secs - b.offset_secs);
                      setGainPoints(nextPts);
                      onUpdateTimeline(await timelineSetClipEnvelope(clip.id, nextPts));
                      onStatus('Added gain automation point');
                    }}
                  >
                    Add Midpoint Keyframe
                  </Button>
                  <Button
                    variant="ghost"
                    size="xs"
                    onClick={async () => {
                      setGainPoints([]);
                      onUpdateTimeline(await timelineSetClipEnvelope(clip.id, []));
                      onStatus('Cleared gain envelope');
                    }}
                  >
                    Reset Flat
                  </Button>
                </div>
                {gainPoints.map((pt, i) => (
                  <div key={`${pt.offset_secs}-${i}`} className="p-2 rounded bg-surface-2 border border-border flex items-center justify-between font-mono text-[11px]">
                    <span>@{pt.offset_secs.toFixed(2)}s</span>
                    <span className="text-primary-text">gain {(pt.gain * 100).toFixed(0)}%</span>
                  </div>
                ))}
              </div>
            </Card>

            <Card title="Stereo Pan Automation" subtitle={`${panPoints.length} pan point(s)`}>
              <div className="space-y-3 text-xs">
                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    variant="secondary"
                    size="xs"
                    onClick={async () => {
                      const sweep: TimelinePanPointDto[] = [
                        { offset_secs: 0, pan: -0.6 },
                        { offset_secs: +(clip.duration_secs * 0.5).toFixed(2), pan: 0.0 },
                        { offset_secs: +clip.duration_secs.toFixed(2), pan: 0.6 },
                      ];
                      setPanPoints(sweep);
                      onUpdateTimeline(await timelineSetClipPanEnvelope(clip.id, sweep));
                      onStatus('Applied L -> R stereo pan sweep');
                    }}
                  >
                    Apply L→R Sweep
                  </Button>
                  <Button
                    variant="ghost"
                    size="xs"
                    onClick={async () => {
                      setPanPoints([]);
                      onUpdateTimeline(await timelineSetClipPanEnvelope(clip.id, []));
                      onStatus('Cleared pan envelope');
                    }}
                  >
                    Center Pan
                  </Button>
                </div>
                {panPoints.map((pt, i) => (
                  <div key={`${pt.offset_secs}-${i}`} className="p-2 rounded bg-surface-2 border border-border flex items-center justify-between font-mono text-[11px]">
                    <span>@{pt.offset_secs.toFixed(2)}s</span>
                    <span className="text-sky-text">pan {pt.pan.toFixed(2)}</span>
                  </div>
                ))}
              </div>
            </Card>
          </div>
        </div>
      )}
    </div>
  );
}
