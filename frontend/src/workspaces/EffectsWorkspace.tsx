/**
 * Audio Labs Pro - Effects Workspace
 *
 * Studio cleanup chain (hum, clicks, parametric EQ, de-esser, compressor, limiter)
 * and sandboxed CLAP / LV2 hosted plugin browser.
 */

import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Slider } from '../kit/Slider';
import { Input } from '../kit/Input';
import { getEffectsChain, getDiscoveredPlugins, getEffectsResponse, runDspMacro } from '../ipc/client';
import type {
  EffectStage,
  DiscoveredPlugin,
  BiquadBandDto,
  BiquadResponseDto,
  MacroJobReportDto,
} from '../ipc/types';
import { FrequencyResponseCanvas } from '../components/FrequencyResponseCanvas';
import {
  Wand2,
  Sliders,
  RefreshCw,
  Power,
  Activity,
  Check,
} from 'lucide-react';

export function EffectsWorkspace() {
  const [stages, setStages] = useState<EffectStage[]>([]);
  const [plugins, setPlugins] = useState<DiscoveredPlugin[]>([]);
  const [isRescanning, setIsRescanning] = useState<boolean>(false);
  const [activeStageId, setActiveStageId] = useState<string>('parametric_eq');
  const [response, setResponse] = useState<BiquadResponseDto | null>(null);
  const [macroInput, setMacroInput] = useState('');
  const [macroOutput, setMacroOutput] = useState('');
  const [macroEffect, setMacroEffect] = useState('voice_cleanup');
  const [macroReport, setMacroReport] = useState<MacroJobReportDto | null>(null);
  const [macroBusy, setMacroBusy] = useState(false);
  const [macroNotice, setMacroNotice] = useState<string | null>(null);

  useEffect(() => {
    getEffectsChain().then((s) => {
      setStages(s);
      if (s.length > 0) setActiveStageId(s[2].id);
    });
    getDiscoveredPlugins().then(setPlugins);
  }, []);

  const toggleStage = (id: string) => {
    setStages((prev) =>
      prev.map((s) => (s.id === id ? { ...s, enabled: !s.enabled } : s))
    );
  };

  const handleRescan = () => {
    setIsRescanning(true);
    setTimeout(() => {
      setIsRescanning(false);
    }, 1000);
  };

  const handleMacro = async () => {
    if (!macroInput.trim() || !macroOutput.trim()) {
      setMacroNotice('Set an input and output directory first');
      return;
    }
    setMacroBusy(true);
    setMacroNotice(null);
    try {
      setMacroReport(
        await runDspMacro({
          input_dir: macroInput.trim(),
          output_dir: macroOutput.trim(),
          overwrite: false,
          steps: [{ effect_id: macroEffect.trim() || 'gain', params: {} }],
        })
      );
    } catch (error) {
      setMacroNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setMacroBusy(false);
    }
  };

  const activeStage = stages.find((s) => s.id === activeStageId) || stages[0];
  const enabledCount = stages.filter((s) => s.enabled).length;

  const bandsForStage = (stage: EffectStage | undefined): BiquadBandDto[] => {
    if (!stage) return [];
    if (stage.id === 'parametric_eq') {
      return [
        { filter_type: 'lowshelf', frequency_hz: 120, q: 0.7, gain_db: stage.params.lowGain ?? 0, enabled: stage.enabled },
        { filter_type: 'peaking', frequency_hz: 1_000, q: 1, gain_db: stage.params.midGain ?? 0, enabled: stage.enabled },
        { filter_type: 'highshelf', frequency_hz: stage.params.highFreq ?? 8_500, q: 0.7, gain_db: stage.params.highGain ?? 0, enabled: stage.enabled },
      ];
    }
    const frequency = stage.params.frequency ?? stage.params.freq ?? stage.params.highFreq ?? 1_000;
    const gain = stage.params.gain ?? stage.params.reduction ?? stage.params.threshold ?? 0;
    return [
      { filter_type: 'peaking', frequency_hz: frequency, q: 1, gain_db: gain, enabled: stage.enabled },
    ];
  };

  useEffect(() => {
    let cancelled = false;
    getEffectsResponse(bandsForStage(activeStage))
      .then((data) => {
        if (!cancelled) setResponse(data);
      })
      .catch((error) => console.error('Failed to compute EQ response:', error));
    return () => {
      cancelled = true;
    };
  }, [activeStage]);

  const setParam = (key: string, value: number) => {
    setStages((prev) =>
      prev.map((stage) =>
        stage.id === activeStageId
          ? { ...stage, params: { ...stage.params, [key]: value } }
          : stage
      )
    );
  };

  const sliderFor = (name: string) => {
    if (name.toLowerCase().includes('freq')) {
      return { min: 20, max: 20_000, step: 10, unit: 'Hz' };
    }
    if (name.toLowerCase().includes('sensitivity') || name.toLowerCase().includes('ratio')) {
      return { min: 0, max: 12, step: 0.1, unit: '' };
    }
    return { min: -36, max: 12, step: 0.5, unit: 'dB' };
  };

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      {/* Workspace Header */}
      <div>
        <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
          <Wand2 className="w-4 h-4 text-[var(--color-accent)]" />
          Effects Chain & Hosted Plugins
        </h2>
        <p className="text-xs text-[var(--color-fg-muted)] mt-1">
          Polish vocal clarity with the 6-stage studio cleanup chain or host third-party CLAP and LV2 plugins inside the sandboxed DSP host.
        </p>
      </div>

      {/* Signal Path Metrics Banner */}
      <div className="grid grid-cols-3 gap-4">
        <div className="p-4 rounded-lg bg-surface border border-border flex items-center justify-between">
          <div>
            <div className="text-[11px] text-muted font-medium">Active Cleanup Stages</div>
            <div className="text-lg font-medium font-sans text-primary-text mt-0.5 tabular-nums">
              {enabledCount} / {stages.length} Active
            </div>
          </div>
          <Activity className="w-6 h-6 text-primary-text" />
        </div>

        <div className="p-4 rounded-lg bg-surface border border-border flex items-center justify-between">
          <div>
            <div className="text-[11px] text-muted font-medium">Mastering Target</div>
            <div className="text-lg font-medium font-sans text-fg mt-0.5 tabular-nums">-16.0 LUFS</div>
          </div>
          <Sliders className="w-6 h-6 text-muted" />
        </div>

        <div className="p-4 rounded-lg bg-surface border border-border flex items-center justify-between">
          <div>
            <div className="text-[11px] text-muted font-medium">True Peak Ceiling</div>
            <div className="text-lg font-medium font-sans text-fg mt-0.5 tabular-nums">-1.0 dBFS</div>
          </div>
          <Check className="w-6 h-6 text-sky" />
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left: 6-Stage Cleanup Chain List (5 cols) */}
        <div className="lg:col-span-5 space-y-4">
          <Card title="Vocal Cleanup Chain" subtitle="Fixed broadcast mastering topology">
            <div className="space-y-2 text-xs">
              {stages.map((stage, idx) => {
                const isSelected = stage.id === activeStageId;

                return (
                  <div
                    key={stage.id}
                    onClick={() => setActiveStageId(stage.id)}
                    className={`p-3 rounded-lg border transition-colors cursor-pointer flex items-center justify-between ${
                      isSelected
                        ? 'bg-[var(--bg-surface)] border-[var(--border-strong)]'
                        : 'bg-[var(--bg-app)] border-[var(--border)] hover:border-[var(--border-strong)]'
                    }`}
                  >
                    <div className="flex items-center gap-3">
                      <span className="font-mono text-[var(--color-fg-muted)] font-medium w-4">{idx + 1}</span>
                      <div>
                        <div className="font-medium text-[var(--color-fg)]">{stage.name}</div>
                        <div className="text-[10px] font-mono text-[var(--color-fg-muted)]">{stage.category}</div>
                      </div>
                    </div>

                    <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
                      <button
                        type="button"
                        onClick={() => toggleStage(stage.id)}
                        className={`p-1.5 rounded-md border transition-colors ${
                          stage.enabled
                            ? 'bg-primary-tint text-primary-text border-border-strong'
                            : 'bg-surface-2 text-faint border-border'
                        }`}
                        title={stage.enabled ? 'Bypass stage' : 'Enable stage'}
                      >
                        <Power className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </Card>
        </div>

        {/* Right: Selected Stage Controls & Hosted Plugins (7 cols) */}
        <div className="lg:col-span-7 space-y-6">
          {/* Active Stage Parameters */}
          {activeStage && (
            <Card title={`${activeStage.name} Parameters`} subtitle={`Stage Category: ${activeStage.category}`}>
              <div className="space-y-4 text-xs">
                <div className="grid grid-cols-2 gap-4">
                  {Object.entries(activeStage.params).map(([paramName, paramVal]) => {
                    const config = sliderFor(paramName);
                    return (
                      <div key={paramName}>
                        <Slider
                          label={paramName}
                          min={config.min}
                          max={config.max}
                          step={config.step}
                          value={paramVal}
                          onChange={(value) => setParam(paramName, value)}
                          unit={config.unit}
                        />
                      </div>
                    );
                  })}
                </div>

                <div className="pt-3 border-t border-[var(--color-border)] flex items-center justify-between text-[11px] text-[var(--color-fg-muted)] font-mono">
                  <span>64-bit double precision float internal DSP processing</span>
                  <span className={activeStage.enabled ? 'text-[var(--color-fg-muted)]' : 'text-[var(--color-fg-muted)]'}>
                    {activeStage.enabled ? 'ONLINE (In Signal Path)' : 'BYPASS'}
                  </span>
                </div>
              </div>
            </Card>
          )}

          {/* Real biquad response curve from alp-dsp */}
          <Card
            title="Frequency response"
            subtitle="Computed from the active BiquadChain coefficients"
          >
            <FrequencyResponseCanvas response={response} />
          </Card>

          {/* Hosted CLAP / LV2 Plugins Card */}
          <Card title="Hosted Plugins (CLAP / LV2)" subtitle={`${plugins.length} discovered native Linux audio plugins`}>
            <div className="space-y-4 text-xs">
              <div className="flex items-center justify-between">
                <span className="text-[var(--color-fg-muted)] text-xs">
                  Plugins execute inside the isolated <code className="text-[var(--color-accent)]">alp-plugin</code> sandbox.
                </span>
                <Button
                  variant="secondary"
                  size="xs"
                  onClick={handleRescan}
                  disabled={isRescanning}
                  leftIcon={
                    isRescanning ? (
                      <RefreshCw className="w-4 h-4 animate-spin" />
                    ) : (
                      <RefreshCw className="w-4 h-4" />
                    )
                  }
                >
                  {isRescanning ? 'Scanning...' : 'Rescan Plugins'}
                </Button>
              </div>

              <div className="space-y-2">
                {plugins.map((plugin) => (
                  <div
                    key={plugin.id}
                    className="p-2.5 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] flex items-center justify-between"
                  >
                    <div>
                      <div className="font-medium text-[var(--color-fg)]">{plugin.name}</div>
                      <div className="text-[10px] font-mono text-[var(--color-fg-muted)]">
                        {plugin.vendor} · {plugin.category}
                      </div>
                    </div>
                    <span className="px-2 py-0.5 rounded font-mono text-[10px] font-medium bg-surface-2 text-purple border border-border-strong">
                      {plugin.format}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </Card>

          {/* Batch DSP Macros */}
          <Card title="Batch DSP Macros" subtitle="Directory queue runner with cancellable jobs">
            <div className="space-y-3 text-xs">
              <Input
                label="Input directory"
                placeholder="/path/to/source-takes"
                value={macroInput}
                onChange={(event) => setMacroInput(event.target.value)}
              />
              <Input
                label="Output directory"
                placeholder="/path/to/processed"
                value={macroOutput}
                onChange={(event) => setMacroOutput(event.target.value)}
              />
              <Input
                label="Effect id"
                placeholder="voice_cleanup"
                value={macroEffect}
                onChange={(event) => setMacroEffect(event.target.value)}
              />
              <Button
                variant="secondary"
                size="xs"
                onClick={handleMacro}
                disabled={macroBusy}
              >
                {macroBusy ? 'Running...' : 'Run Macro Batch'}
              </Button>
              {macroNotice && (
                <div className="text-[11px] text-amber">{macroNotice}</div>
              )}
              {macroReport && (
                <div className="pt-2 border-t border-[var(--color-border)] font-mono text-[10px] text-[var(--color-fg-muted)] space-y-1">
                  <div>
                    job {macroReport.job_id} · {macroReport.report.succeeded}/{macroReport.report.processed} succeeded
                  </div>
                  {macroReport.report.files.map((file) => (
                    <div key={file.output} className="truncate" title={file.output}>
                      {file.ok ? 'ok' : 'failed'} · {file.output}
                    </div>
                  ))}
                </div>
              )}
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
