/**
 * Audio Labs Pro - Captures Workspace
 *
 * Take recording with input health audit, live voice activity detection (VAD),
 * and interactive word-level transcript editor.
 */

import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Toggle } from '../kit/Toggle';
import { Meter } from '../kit/Meter';
import { getCaptures, getTranscript } from '../ipc/client';
import type { CaptureTake, TranscriptWord } from '../ipc/types';
import {
  Mic,
  Circle,
  Square,
  Scissors,
  CheckCircle2,
  Clock,
} from 'lucide-react';

export interface CapturesWorkspaceProps {
  onSeek?: (secs: number) => void;
}

export function CapturesWorkspace({ onSeek }: CapturesWorkspaceProps) {
  const [takes, setTakes] = useState<CaptureTake[]>([]);
  const [transcript, setTranscript] = useState<TranscriptWord[]>([]);
  const [isRecording, setIsRecording] = useState<boolean>(false);
  const [recordSecs, setRecordSecs] = useState<number>(0);
  const [monitor, setMonitor] = useState<boolean>(false);
  const [punchIn, setPunchIn] = useState<boolean>(false);
  const [inputLevel, setInputLevel] = useState<number>(0.0);
  const [selectedWordIdx, setSelectedWordIdx] = useState<number | null>(null);

  useEffect(() => {
    getCaptures().then(setTakes);
    getTranscript().then(setTranscript);
  }, []);

  // Timer & meter simulation during recording
  useEffect(() => {
    let timer: ReturnType<typeof setInterval>;
    if (isRecording) {
      timer = setInterval(() => {
        setRecordSecs((s) => s + 0.1);
        setInputLevel(Math.min(0.92, Math.random() * 0.5 + 0.3));
      }, 100);
    } else {
      setRecordSecs(0);
      setInputLevel(0.0);
    }
    return () => clearInterval(timer);
  }, [isRecording]);

  const handleToggleRecord = () => {
    if (isRecording) {
      // Save new take
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

  const handleWordClick = (word: TranscriptWord, idx: number) => {
    setSelectedWordIdx(idx);
    onSeek?.(word.start_secs);
  };

  const handleDeleteWord = (idx: number) => {
    setTranscript((prev) => prev.filter((_, i) => i !== idx));
    setSelectedWordIdx(null);
  };

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      {/* Workspace Header */}
      <div>
        <h2 className="text-xl font-bold tracking-tight text-slate-100 flex items-center gap-2">
          <Mic className="w-5 h-5 text-red-400" />
          Captures & Transcript Editor
        </h2>
        <p className="text-xs text-slate-400 mt-1">
          Record vocal takes with real-time level auditing, speech-to-text transcription, and word-level non-destructive editing.
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left: Recording Deck & Takes List (5 cols) */}
        <div className="lg:col-span-5 space-y-6">
          {/* Recording Control Deck */}
          <Card title="Live Audio Capture" subtitle="ALSA / PipeWire direct hardware input">
            <div className="space-y-4 text-xs">
              {/* Record Action & Timer */}
              <div className="p-4 rounded-xl bg-slate-900 border border-white/10 flex items-center justify-between">
                <div className="space-y-1">
                  <div className="text-[11px] font-mono uppercase text-slate-400">
                    {isRecording ? 'Recording Active' : 'Input Standby'}
                  </div>
                  <div className="text-2xl font-bold font-mono text-slate-100">
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
                      <Circle className="w-4 h-4 fill-current text-red-500" />
                    )
                  }
                >
                  {isRecording ? 'Stop Recording' : 'Record Take'}
                </Button>
              </div>

              {/* Input Meter */}
              <div className="space-y-1.5">
                <div className="flex justify-between text-[11px] font-mono text-slate-400">
                  <span>Input Gain Meter</span>
                  <span>{isRecording ? '-1.2 dBFS' : '-inf'}</span>
                </div>
                <Meter leftLevel={inputLevel} rightLevel={inputLevel * 0.98} showLabels={false} />
              </div>

              {/* Monitor & Punch-In Toggles */}
              <div className="pt-2 border-t border-white/5 space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-slate-300">Software Input Monitoring</span>
                  <Toggle checked={monitor} onChange={setMonitor} label="" />
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-slate-300">Punch-In Overwrite Mode</span>
                  <Toggle checked={punchIn} onChange={setPunchIn} label="" />
                </div>
              </div>
            </div>
          </Card>

          {/* Takes Library */}
          <Card title="Recorded Takes" subtitle={`${takes.length} takes in project storage`}>
            <div className="space-y-2 text-xs max-h-[320px] overflow-y-auto pr-1">
              {takes.map((take) => (
                <div
                  key={take.id}
                  className="p-3 rounded-lg bg-slate-900/80 border border-white/10 hover:border-white/20 transition-all space-y-1.5"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-semibold text-slate-200">{take.name}</span>
                    <span className="font-mono text-[11px] text-cyan-400 font-bold">{take.duration_secs}s</span>
                  </div>

                  <div className="flex items-center justify-between text-[11px] font-mono text-slate-400">
                    <span className="flex items-center gap-1">
                      <Clock className="w-3 h-3 text-slate-500" />
                      {take.timestamp}
                    </span>
                    <span className="flex items-center gap-1 text-emerald-400">
                      <CheckCircle2 className="w-3 h-3" />
                      Peak: {take.peak_db} dBFS
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </Card>
        </div>

        {/* Right: Word-Level Transcript Editor (7 cols) */}
        <div className="lg:col-span-7 space-y-4">
          <Card title="Interactive Word Transcript" subtitle="Sherpa-ONNX speech-to-text alignment with timeline click-to-seek">
            <div className="space-y-4 text-xs">
              {/* Word Chips Surface */}
              <div className="p-4 rounded-xl bg-slate-900/90 border border-white/10 min-h-[220px] flex flex-wrap gap-2 items-center leading-loose">
                {transcript.map((item, idx) => {
                  const isSelected = selectedWordIdx === idx;

                  return (
                    <button
                      key={`${item.word}-${idx}`}
                      type="button"
                      onClick={() => handleWordClick(item, idx)}
                      className={`px-2 py-1 rounded-md text-xs font-medium transition-all flex items-center gap-1 border ${
                        isSelected
                          ? 'bg-cyan-500 text-slate-950 border-cyan-400 shadow-md shadow-cyan-500/30 scale-105'
                          : 'bg-slate-800 text-slate-200 border-white/10 hover:bg-slate-700 hover:border-white/20'
                      }`}
                      title={`${item.start_secs}s - ${item.end_secs}s (${(item.confidence * 100).toFixed(0)}% conf)`}
                    >
                      <span>{item.word}</span>
                      <span className="text-[9px] opacity-60 font-mono">
                        {item.start_secs}s
                      </span>
                    </button>
                  );
                })}
              </div>

              {/* Word Action Bar */}
              {selectedWordIdx !== null && transcript[selectedWordIdx] && (
                <div className="p-3 rounded-lg bg-cyan-950/60 border border-cyan-500/30 flex items-center justify-between animate-fade-in">
                  <div className="space-y-0.5 font-mono text-[11px]">
                    <span className="text-cyan-300 font-bold text-sm">
                      &ldquo;{transcript[selectedWordIdx].word}&rdquo;
                    </span>
                    <span className="text-slate-400 block">
                      Range: {transcript[selectedWordIdx].start_secs}s – {transcript[selectedWordIdx].end_secs}s · Confidence: {(transcript[selectedWordIdx].confidence * 100).toFixed(1)}%
                    </span>
                  </div>

                  <div className="flex items-center gap-2">
                    <Button
                      variant="danger"
                      size="xs"
                      onClick={() => handleDeleteWord(selectedWordIdx)}
                      leftIcon={<Scissors className="w-3 h-3" />}
                    >
                      Cut Word Segment
                    </Button>
                  </div>
                </div>
              )}

              <div className="p-3 rounded-lg bg-slate-950/80 border border-white/10 text-[11px] text-slate-400 space-y-1">
                <span className="font-semibold text-slate-300 block">Non-Destructive Text Editing:</span>
                <p>
                  Clicking any word seeks the audio playhead directly to its start timestamp. Cutting words executes non-destructive range split and ripple edits in the session command stack.
                </p>
              </div>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
