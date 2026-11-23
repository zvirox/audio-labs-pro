/**
 * Audio Labs Pro - Transport & Audio Monitoring Hook
 *
 * Connects UI views to alp-desktop transport and meter streams.
 * Implements client-side playhead interpolation between IPC meter anchors
 * for smooth 60-120 fps visual playhead motion.
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import {
  getTransportState,
  playTransport,
  pauseTransport,
  stopTransport,
  seekTransport,
  setTransportVolume,
  setTransportLoop,
  subscribeMeters,
} from '../ipc/client';
import type { MeterFrame, TransportState } from '../ipc/types';

export interface UseTransportResult {
  playing: boolean;
  positionSecs: number;
  durationSecs: number;
  volume: number;
  loopEnabled: boolean;
  loopStartSecs: number;
  loopEndSecs: number;
  peakL: number;
  peakR: number;
  rmsL: number;
  rmsR: number;
  isClipping: boolean;
  togglePlay: () => Promise<void>;
  stop: () => Promise<void>;
  seek: (positionSecs: number) => Promise<void>;
  setVolume: (volume: number) => Promise<void>;
  toggleLoop: () => Promise<void>;
}

export function useTransport(): UseTransportResult {
  const [playing, setPlaying] = useState<boolean>(false);
  const [positionSecs, setPositionSecs] = useState<number>(0.0);
  const [durationSecs, setDurationSecs] = useState<number>(20.0);
  const [volume, setVolumeState] = useState<number>(1.0);
  const [loopEnabled, setLoopEnabled] = useState<boolean>(false);
  const [loopStartSecs, setLoopStartSecs] = useState<number>(0.0);
  const [loopEndSecs, setLoopEndSecs] = useState<number>(20.0);
  const [peakL, setPeakL] = useState<number>(0.0);
  const [peakR, setPeakR] = useState<number>(0.0);
  const [rmsL, setRmsL] = useState<number>(0.0);
  const [rmsR, setRmsR] = useState<number>(0.0);
  const [isClipping, setIsClipping] = useState<boolean>(false);

  // Playhead anchor ref for requestAnimationFrame interpolation
  const anchorRef = useRef<{
    position: number;
    timestamp: number;
    playing: boolean;
    duration: number;
    loopEnabled: boolean;
    loopStart: number;
    loopEnd: number;
  }>({
    position: 0.0,
    timestamp: performance.now(),
    playing: false,
    duration: 20.0,
    loopEnabled: false,
    loopStart: 0.0,
    loopEnd: 20.0,
  });

  // Initial transport state fetch
  useEffect(() => {
    let mounted = true;
    getTransportState().then((state: TransportState) => {
      if (!mounted) return;
      setPlaying(state.playing);
      setPositionSecs(state.position_secs);
      setDurationSecs(state.duration_secs);
      setVolumeState(state.volume);
      setLoopEnabled(state.loop_enabled);
      setLoopStartSecs(state.loop_start_secs);
      setLoopEndSecs(state.loop_end_secs);

      anchorRef.current = {
        position: state.position_secs,
        timestamp: performance.now(),
        playing: state.playing,
        duration: state.duration_secs,
        loopEnabled: state.loop_enabled,
        loopStart: state.loop_start_secs,
        loopEnd: state.loop_end_secs,
      };
    });

    return () => {
      mounted = false;
    };
  }, []);

  // Subscribe to Tauri Channel meters
  useEffect(() => {
    let unlisten: (() => void) | null = null;

    subscribeMeters((frame: MeterFrame) => {
      setPeakL(frame.peak_l);
      setPeakR(frame.peak_r);
      setRmsL(frame.rms_l);
      setRmsR(frame.rms_r);
      setIsClipping(frame.peak_l >= 0.99 || frame.peak_r >= 0.99);

      if (frame.playing !== anchorRef.current.playing) {
        setPlaying(frame.playing);
      }
      setDurationSecs(frame.duration_secs);

      anchorRef.current.position = frame.position_secs;
      anchorRef.current.timestamp = performance.now();
      anchorRef.current.playing = frame.playing;
      anchorRef.current.duration = frame.duration_secs;
    }).then((cleanup) => {
      unlisten = cleanup;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Smooth playhead interpolation via requestAnimationFrame
  useEffect(() => {
    let animId: number;

    const renderLoop = () => {
      const anchor = anchorRef.current;
      if (anchor.playing) {
        const elapsedSecs = (performance.now() - anchor.timestamp) / 1000.0;
        let interpolated = anchor.position + elapsedSecs;

        const maxBoundary = anchor.loopEnabled && anchor.loopEnd > anchor.loopStart
          ? anchor.loopEnd
          : anchor.duration;

        if (interpolated >= maxBoundary) {
          interpolated = anchor.loopEnabled ? anchor.loopStart : 0.0;
        }

        setPositionSecs(interpolated);
      } else {
        setPositionSecs(anchor.position);
      }

      animId = requestAnimationFrame(renderLoop);
    };

    animId = requestAnimationFrame(renderLoop);
    return () => {
      cancelAnimationFrame(animId);
    };
  }, []);

  const togglePlay = useCallback(async () => {
    if (playing) {
      const next = await pauseTransport();
      setPlaying(false);
      anchorRef.current.playing = false;
      anchorRef.current.position = next.position_secs;
      setPositionSecs(next.position_secs);
    } else {
      const next = await playTransport();
      setPlaying(true);
      anchorRef.current.playing = true;
      anchorRef.current.position = next.position_secs;
      anchorRef.current.timestamp = performance.now();
      setPositionSecs(next.position_secs);
    }
  }, [playing]);

  const stop = useCallback(async () => {
    const next = await stopTransport();
    setPlaying(false);
    anchorRef.current.playing = false;
    anchorRef.current.position = next.position_secs;
    setPositionSecs(next.position_secs);
  }, []);

  const seek = useCallback(
    async (targetSecs: number) => {
      const next = await seekTransport(targetSecs);
      anchorRef.current.position = next.position_secs;
      anchorRef.current.timestamp = performance.now();
      setPositionSecs(next.position_secs);
    },
    []
  );

  const setVolume = useCallback(async (newVol: number) => {
    const next = await setTransportVolume(newVol);
    setVolumeState(next.volume);
  }, []);

  const toggleLoop = useCallback(async () => {
    const nextLoop = !loopEnabled;
    const next = await setTransportLoop(nextLoop, loopStartSecs, loopEndSecs);
    setLoopEnabled(next.loop_enabled);
    anchorRef.current.loopEnabled = next.loop_enabled;
    anchorRef.current.loopStart = next.loop_start_secs;
    anchorRef.current.loopEnd = next.loop_end_secs;
  }, [loopEnabled, loopStartSecs, loopEndSecs]);

  return {
    playing,
    positionSecs,
    durationSecs,
    volume,
    loopEnabled,
    loopStartSecs,
    loopEndSecs,
    peakL,
    peakR,
    rmsL,
    rmsR,
    isClipping,
    togglePlay,
    stop,
    seek,
    setVolume,
    toggleLoop,
  };
}
