/**
 * Audio Labs Pro - Multitrack DAW Workspace
 *
 * Full non-linear multitrack arranger, clip envelopes, magnetic snapping,
 * and zero-latency editing with complete keyboard shortcut parity.
 */

import { useState, useEffect, useCallback } from 'react';
import { Button } from '../kit/Button';
import { Slider } from '../kit/Slider';
import { MultitrackCanvas } from '../timeline/MultitrackCanvas';
import type { TimelineDto, TimelineCommandPayload } from '../timeline/types';
import {
  getTimelineState,
  timelineUndo,
  timelineRedo,
  timelineSplitClip,
  timelineDeleteSelected,
  timelineCut,
  timelineCopy,
  timelinePaste,
  timelineMoveClip,
  timelineTrimClip,
  timelineSetClipFades,
  timelineSetLoopRange,
  timelineSetSelection,
  timelineSelectClip,
  listenSessionRevision,
} from '../ipc/client';
import {
  Scissors,
  Trash2,
  Undo2,
  Redo2,
  Magnet,
  Waves,
  ZoomIn,
  ZoomOut,
  Play,
  Pause,
  RotateCcw,
  Layers,
} from 'lucide-react';

export interface MultitrackWorkspaceProps {
  currentTimeSecs: number;
  isPlaying: boolean;
  onSeek: (secs: number) => void;
  onTogglePlay?: () => void;
}

export function MultitrackWorkspace({
  currentTimeSecs,
  isPlaying,
  onSeek,
  onTogglePlay,
}: MultitrackWorkspaceProps) {
  const [timeline, setTimeline] = useState<TimelineDto | null>(null);
  const [zoom, setZoom] = useState<number>(80); // 80px per second default
  const [snapEnabled, setSnapEnabled] = useState<boolean>(true);
  const [rippleEnabled, setRippleEnabled] = useState<boolean>(false);
  const [statusMessage, setStatusMessage] = useState<string>('Ready');

  // Load initial timeline from backend
  const fetchTimeline = useCallback(async () => {
    try {
      const data = await getTimelineState();
      setTimeline(data);
    } catch (err) {
      console.error('Failed to load timeline state:', err);
    }
  }, []);

  useEffect(() => {
    fetchTimeline();
    const unlisten = listenSessionRevision(() => {
      fetchTimeline();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchTimeline]);

  // Command handlers
  const handleUndo = useCallback(async () => {
    if (!timeline?.can_undo) return;
    try {
      const next = await timelineUndo();
      setTimeline(next);
      setStatusMessage('Undone previous edit');
    } catch (err) {
      console.error('Undo failed:', err);
    }
  }, [timeline]);

  const handleRedo = useCallback(async () => {
    if (!timeline?.can_redo) return;
    try {
      const next = await timelineRedo();
      setTimeline(next);
      setStatusMessage('Redone edit');
    } catch (err) {
      console.error('Redo failed:', err);
    }
  }, [timeline]);

  const handleSplitAtPlayhead = useCallback(async () => {
    if (!timeline) return;
    // Find clip under playhead
    let targetClipId: string | null = timeline.selected_clip_id;
    if (!targetClipId) {
      for (const track of timeline.tracks) {
        const found = track.clips.find(
          (c) =>
            currentTimeSecs >= c.timeline_start_secs &&
            currentTimeSecs <= c.timeline_start_secs + c.duration_secs
        );
        if (found) {
          targetClipId = found.id;
          break;
        }
      }
    }

    if (!targetClipId) {
      setStatusMessage('No clip under playhead to split');
      return;
    }

    try {
      const next = await timelineSplitClip(targetClipId, currentTimeSecs);
      setTimeline(next);
      setStatusMessage(`Split clip at ${currentTimeSecs.toFixed(2)}s`);
    } catch (err) {
      setStatusMessage(`Split rejected: ${err}`);
    }
  }, [timeline, currentTimeSecs]);

  const handleDeleteSelected = useCallback(async () => {
    if (!timeline?.selected_clip_id) {
      setStatusMessage('No clip selected to delete');
      return;
    }
    try {
      const next = await timelineDeleteSelected(rippleEnabled);
      setTimeline(next);
      setStatusMessage('Deleted clip');
    } catch (err) {
      setStatusMessage(`Delete failed: ${err}`);
    }
  }, [timeline, rippleEnabled]);

  const handleCut = useCallback(async () => {
    if (!timeline?.selected_clip_id) return;
    try {
      const next = await timelineCut(rippleEnabled);
      setTimeline(next);
      setStatusMessage('Cut clip to clipboard');
    } catch (err) {
      console.error(err);
    }
  }, [timeline, rippleEnabled]);

  const handleCopy = useCallback(async () => {
    await timelineCopy();
    setStatusMessage('Copied clip to clipboard');
  }, []);

  const handlePaste = useCallback(async () => {
    try {
      const next = await timelinePaste();
      setTimeline(next);
      setStatusMessage('Pasted from clipboard');
    } catch (err) {
      console.error(err);
    }
  }, []);

  const handleSelectAll = useCallback(async () => {
    if (!timeline) return;
    const next = await timelineSetSelection(0, timeline.duration_secs);
    setTimeline(next);
    setStatusMessage('Selected all');
  }, [timeline]);

  // Handle command from canvas dragging
  const handleDispatchCommand = useCallback(
    async (cmd: TimelineCommandPayload) => {
      try {
        let next: TimelineDto | null = null;
        switch (cmd.type) {
          case 'seek':
            onSeek(cmd.positionSecs);
            break;
          case 'move_clip':
            next = await timelineMoveClip(cmd.clipId, cmd.targetTrackId, cmd.newStartSecs);
            break;
          case 'trim_clip':
            next = await timelineTrimClip(cmd.clipId, cmd.newStartSecs, cmd.newDurationSecs);
            break;
          case 'set_clip_fades':
            next = await timelineSetClipFades(cmd.clipId, cmd.fadeInSecs, cmd.fadeOutSecs);
            break;
          case 'set_loop_range':
            next = await timelineSetLoopRange(cmd.startSecs, cmd.endSecs, cmd.enabled);
            break;
          case 'set_selection':
            next = await timelineSetSelection(cmd.startSecs, cmd.endSecs);
            break;
          case 'select_clip':
            next = await timelineSelectClip(cmd.clipId);
            break;
        }
        if (next) setTimeline(next);
      } catch (err) {
        console.error('Command dispatch error:', err);
      }
    },
    [onSeek]
  );

  // Keyboard shortcut listener for all timeline commands
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const activeEl = document.activeElement;
      if (
        activeEl instanceof HTMLInputElement ||
        activeEl instanceof HTMLTextAreaElement ||
        activeEl?.getAttribute('contenteditable') === 'true'
      ) {
        return;
      }

      if (e.ctrlKey || e.metaKey) {
        if (e.key === 'z' || e.key === 'Z') {
          e.preventDefault();
          if (e.shiftKey) handleRedo();
          else handleUndo();
        } else if (e.key === 'y' || e.key === 'Y') {
          e.preventDefault();
          handleRedo();
        } else if (e.key === 'b' || e.key === 'B') {
          e.preventDefault();
          handleSplitAtPlayhead();
        } else if (e.key === 'c' || e.key === 'C') {
          e.preventDefault();
          handleCopy();
        } else if (e.key === 'x' || e.key === 'X') {
          e.preventDefault();
          handleCut();
        } else if (e.key === 'v' || e.key === 'V') {
          e.preventDefault();
          handlePaste();
        } else if (e.key === 'a' || e.key === 'A') {
          e.preventDefault();
          handleSelectAll();
        } else if (e.key === '=' || e.key === '+') {
          e.preventDefault();
          setZoom((z) => Math.min(1000, z * 1.25));
        } else if (e.key === '-') {
          e.preventDefault();
          setZoom((z) => Math.max(10, z * 0.8));
        }
      } else {
        if (e.key === 'Delete' || e.key === 'Backspace') {
          e.preventDefault();
          handleDeleteSelected();
        } else if (e.key === 'r' || e.key === 'R') {
          e.preventDefault();
          setRippleEnabled((prev) => !prev);
        } else if (e.key === 's' || e.key === 'S') {
          e.preventDefault();
          setSnapEnabled((prev) => !prev);
        } else if (e.key === 'Home') {
          e.preventDefault();
          onSeek(0);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [
    handleUndo,
    handleRedo,
    handleSplitAtPlayhead,
    handleDeleteSelected,
    handleCut,
    handleCopy,
    handlePaste,
    handleSelectAll,
    onSeek,
  ]);

  if (!timeline) {
    return (
      <div className="h-full flex items-center justify-center text-slate-400">
        <Layers className="w-6 h-6 animate-spin mr-2 text-cyan-400" />
        Loading multitrack session...
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col overflow-hidden bg-[#090d16] select-none">
      {/* Arranger Top Toolbar */}
      <div className="h-10 border-b border-white/10 px-3 flex items-center justify-between bg-slate-900/90 flex-shrink-0 z-10">
        {/* Left: Edit Operations */}
        <div className="flex items-center gap-1">
          <Button
            variant="secondary"
            size="xs"
            onClick={handleUndo}
            disabled={!timeline.can_undo}
            leftIcon={<Undo2 className="w-3.5 h-3.5" />}
            title="Undo last edit (Ctrl+Z)"
          >
            Undo
          </Button>
          <Button
            variant="secondary"
            size="xs"
            onClick={handleRedo}
            disabled={!timeline.can_redo}
            leftIcon={<Redo2 className="w-3.5 h-3.5" />}
            title="Redo undone edit (Ctrl+Y)"
          >
            Redo
          </Button>

          <div className="h-4 w-[1px] bg-white/10 mx-1" />

          <Button
            variant="secondary"
            size="xs"
            onClick={handleSplitAtPlayhead}
            leftIcon={<Scissors className="w-3.5 h-3.5 text-cyan-400" />}
            title="Split clip at playhead position (Ctrl+B)"
          >
            Split
          </Button>

          <Button
            variant="secondary"
            size="xs"
            onClick={handleDeleteSelected}
            disabled={!timeline.selected_clip_id}
            leftIcon={<Trash2 className="w-3.5 h-3.5 text-rose-400" />}
            title="Delete selected clip (Delete)"
          >
            Delete
          </Button>

          <div className="h-4 w-[1px] bg-white/10 mx-1" />

          {/* Magnetic Snapping Toggle */}
          <Button
            variant={snapEnabled ? 'primary' : 'secondary'}
            size="xs"
            onClick={() => setSnapEnabled(!snapEnabled)}
            leftIcon={<Magnet className="w-3.5 h-3.5 text-amber-400" />}
            title="Toggle magnetic snapping to markers and clip edges (S)"
          >
            Snap
          </Button>

          {/* Ripple Mode Toggle */}
          <Button
            variant={rippleEnabled ? 'primary' : 'secondary'}
            size="xs"
            onClick={() => setRippleEnabled(!rippleEnabled)}
            leftIcon={<Waves className="w-3.5 h-3.5 text-indigo-400" />}
            title="Toggle automatic ripple editing on cuts and deletions (R)"
          >
            Ripple
          </Button>
        </div>

        {/* Center: Playhead Timecode & Status */}
        <div className="flex items-center gap-3">
          <div className="font-mono text-xs font-bold text-cyan-400 bg-slate-950 px-2.5 py-1 rounded border border-cyan-500/30">
            {new Date(currentTimeSecs * 1000).toISOString().substring(14, 22)}
          </div>
          <span className="text-[11px] text-slate-400 font-sans hidden sm:inline">
            {statusMessage}
          </span>
        </div>

        {/* Right: Zoom & Track Adder */}
        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            size="xs"
            onClick={() => setZoom((z) => Math.max(10, z * 0.8))}
            leftIcon={<ZoomOut className="w-3 h-3" />}
            title="Zoom out (Ctrl+-)"
          />
          <div className="w-20 hidden md:block">
            <Slider
              value={zoom}
              min={10}
              max={400}
              onChange={setZoom}
            />
          </div>
          <Button
            variant="secondary"
            size="xs"
            onClick={() => setZoom((z) => Math.min(1000, z * 1.25))}
            leftIcon={<ZoomIn className="w-3 h-3" />}
            title="Zoom in (Ctrl++)"
          />

          <div className="h-4 w-[1px] bg-white/10 mx-1" />

          <Button
            variant="secondary"
            size="xs"
            onClick={() => onSeek(0)}
            leftIcon={<RotateCcw className="w-3 h-3" />}
            title="Rewind playhead to start (Home)"
          >
            0:00
          </Button>

          {onTogglePlay && (
            <Button
              variant={isPlaying ? 'secondary' : 'primary'}
              size="xs"
              onClick={onTogglePlay}
              leftIcon={
                isPlaying ? (
                  <Pause className="w-3 h-3 fill-current text-amber-400" />
                ) : (
                  <Play className="w-3 h-3 fill-current text-cyan-400" />
                )
              }
              title="Play / Pause timeline (Space)"
            >
              {isPlaying ? 'Pause' : 'Play'}
            </Button>
          )}
        </div>
      </div>

      {/* Main Multitrack Canvas Arranger Area */}
      <div className="flex-1 relative overflow-hidden">
        <MultitrackCanvas
          timeline={timeline}
          currentTimeSecs={currentTimeSecs}
          isPlaying={isPlaying}
          onSeek={onSeek}
          onSelectClip={(id) => handleDispatchCommand({ type: 'select_clip', clipId: id })}
          onDispatchCommand={handleDispatchCommand}
          zoom={zoom}
          onZoomChange={setZoom}
          snapEnabled={snapEnabled}
          rippleEnabled={rippleEnabled}
        />
      </div>

      {/* Footer Track & Clip Status Bar */}
      <div className="h-6 border-t border-white/5 bg-slate-950 px-3 flex items-center justify-between text-[11px] text-slate-400 font-mono flex-shrink-0">
        <div className="flex items-center gap-4">
          <span>Tracks: {timeline.tracks.length}</span>
          <span>
            Clips:{' '}
            {timeline.tracks.reduce((sum, t) => sum + t.clips.length, 0)}
          </span>
          {timeline.selected_clip_id && (
            <span className="text-cyan-400 font-bold">
              Selected: {timeline.selected_clip_id}
            </span>
          )}
        </div>
        <div className="flex items-center gap-4">
          <span>Undo Steps: {timeline.undo_steps}</span>
          <span>Redo Steps: {timeline.redo_steps}</span>
          <span>Sample Rate: {timeline.sample_rate} Hz</span>
        </div>
      </div>
    </div>
  );
}
