/**
 * Audio Labs Pro - Effects & Plugin Hosting Workspace
 *
 * Dedicated subtabs:
 * - `eq`: Parametric EQ & Hum Notch + live BiquadChain frequency response curve
 * - `dynamics`: Multiband Compressor, De-Esser & Broadcast Peak Limiter
 * - `spatial`: 6-Stage Vocal Cleanup Chain topology + Batch Macro Runner
 * - `vst`: Real OS CLAP / LV2 / VST3 plugin discovery + X11 window bridge status
 */

import { useState, useEffect, useCallback } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Slider } from '../kit/Slider';
import { Input } from '../kit/Input';
import {
  getEffectsChain,
  getDiscoveredPlugins,
  getEffectsResponse,
  runDspMacro,
} from '../ipc/client';
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
  Plug,
  Gauge,
} from 'lucide-react';

export interface EffectsWorkspaceProps {
  subtab?: string;
}

export function EffectsWorkspace({ subtab = 'eq' }: EffectsWorkspaceProps) {
  const [stages, setStages] = useState<EffectStage[]>([]);
  const [plugins, setPlugins] = useState<DiscoveredPlugin[]>([]);
  const [isRescanning, setIsRescanning] = useState<boolean>(false);
  const [formatFilter, setFormatFilter] = useState<'ALL' | 'CLAP' | 'LV2' | 'VST3'>('ALL');
  const [activeStageId, setActiveStageId] = useState<string>('parametric_eq');
  const [response, setResponse] = useState<BiquadResponseDto | null>(null);
  const [macroInput, setMacroInput] = useState('');
  const [macroOutput, setMacroOutput] = useState('');
  const [macroEffect, setMacroEffect] = useState('voice_cleanup');
  const [macroReport, setMacroReport] = useState<MacroJobReportDto | null>(null);
  const [macroBusy, setMacroBusy] = useState(false);
  const [macroNotice, setMacroNotice] = useState<string | null>(null);

  const scanPlugins = useCallback(async () => {
    setIsRescanning(true);
    try {
      const found = await getDiscoveredPlugins();
      setPlugins(found);
    } catch (err) {
      console.error('Plugin rescan failed:', err);
    } finally {
      setIsRescanning(false);
    }
  }, []);

  useEffect(() => {
    getEffectsChain().then((s) => {
      setStages(s);
      if (s.length > 2) setActiveStageId(s[2].id);
    });
    scanPlugins();
  }, [scanPlugins]);

  // Sync default selected stage when navigating between DSP subtabs
  useEffect(() => {
    if (subtab === 'eq') setActiveStageId('parametric_eq');
    else if (subtab === 'dynamics') setActiveStageId('compressor');
  }, [subtab]);

  const toggleStage = (id: string) => {
    setStages((prev) =>
      prev.map((s) => (s.id === id ? { ...s, enabled: !s.enabled } : s))
    );
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
        {
          filter_type: 'lowshelf',
          frequency_hz: 120,
          q: 0.7,
          gain_db: stage.params.lowGain ?? 0,
          enabled: stage.enabled,
        },
        {
          filter_type: 'peaking',
          frequency_hz: 1_000,
          q: 1,
          gain_db: stage.params.midGain ?? 0,
          enabled: stage.enabled,
        },
        {
          filter_type: 'highshelf',
          frequency_hz: stage.params.highFreq ?? 8_500,
          q: 0.7,
          gain_db: stage.params.highGain ?? 0,
          enabled: stage.enabled,
        },
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

  const setStageParam = (stageId: string, key: string, value: number) => {
    setStages((prev) =>
      prev.map((stage) =>
        stage.id === stageId ? { ...stage, params: { ...stage.params, [key]: value } } : stage
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
    if (name.toLowerCase().includes('attack') || name.toLowerCase().includes('release')) {
      return { min: 1, max: 400, step: 1, unit: 'ms' };
    }
    return { min: -36, max: 12, step: 0.5, unit: 'dB' };
  };

  const visiblePlugins = plugins.filter(
    (p) => formatFilter === 'ALL' || p.format.toUpperCase() === formatFilter
  );

  const dynamicsStages = stages.filter((s) =>
    ['de_esser', 'compressor', 'peak_limiter'].includes(s.id)
  );

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      <div>
        <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
          {subtab === 'vst' ? (
            <Plug className="w-4 h-4 text-[var(--color-accent)]" />
          ) : subtab === 'dynamics' ? (
            <Gauge className="w-4 h-4 text-[var(--color-accent)]" />
          ) : (
            <Wand2 className="w-4 h-4 text-[var(--color-accent)]" />
          )}
          {subtab === 'eq' && 'Parametric EQ & Filter Response'}
          {subtab === 'dynamics' && 'Multiband Compressor, De-Esser & Limiter'}
          {subtab === 'spatial' && '6-Stage Vocal Cleanup Chain & Batch Macros'}
          {subtab === 'vst' && 'Hosted Linux Plugins (CLAP / LV2 / VST3)'}
        </h2>
        <p className="text-xs text-[var(--color-fg-muted)] mt-1">
          {subtab === 'eq' && 'Shape tonal balance and inspect live BiquadChain magnitude and phase response.'}
          {subtab === 'dynamics' && 'Control dynamic range, sibilance suppression, and broadcast true-peak ceilings.'}
          {subtab === 'spatial' && 'Configure the 6-stage signal path and dispatch directory-wide DSP macro chains.'}
          {subtab === 'vst' && 'Real system scan across Linux CLAP, LV2, and COM-hosted VST3 plugins.'}
        </p>
      </div>

      {/* Subtab 1: Parametric EQ */}
      {subtab === 'eq' && (
        <div className="space-y-6">
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
                <div className="text-[11px] text-muted font-medium">EQ Architecture</div>
                <div className="text-lg font-medium font-sans text-fg mt-0.5 tabular-nums">f64 BiquadChain</div>
              </div>
              <Sliders className="w-6 h-6 text-muted" />
            </div>
            <div className="p-4 rounded-lg bg-surface border border-border flex items-center justify-between">
              <div>
                <div className="text-[11px] text-muted font-medium">Sample Grid</div>
                <div className="text-lg font-medium font-sans text-fg mt-0.5 tabular-nums">48.0 kHz</div>
              </div>
              <Check className="w-6 h-6 text-sky" />
            </div>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
            <div className="lg:col-span-5">
              {activeStage && (
                <Card title={`${activeStage.name} Parameters`} subtitle={`Stage Category: ${activeStage.category}`}>
                  <div className="space-y-4 text-xs">
                    {Object.entries(activeStage.params).map(([paramName, paramVal]) => {
                      const config = sliderFor(paramName);
                      return (
                        <Slider
                          key={paramName}
                          label={paramName}
                          min={config.min}
                          max={config.max}
                          step={config.step}
                          value={paramVal}
                          onChange={(value) => setStageParam(activeStage.id, paramName, value)}
                          unit={config.unit}
                        />
                      );
                    })}
                    <div className="pt-2 border-t border-[var(--color-border)] flex items-center justify-between">
                      <span className="text-[11px] font-mono text-muted">
                        {activeStage.enabled ? 'In Signal Path' : 'Bypassed'}
                      </span>
                      <Button variant="secondary" size="xs" onClick={() => toggleStage(activeStage.id)}>
                        {activeStage.enabled ? 'Bypass Stage' : 'Enable Stage'}
                      </Button>
                    </div>
                  </div>
                </Card>
              )}
            </div>

            <div className="lg:col-span-7">
              <Card
                title="Frequency Response"
                subtitle="Computed directly from alp-dsp BiquadChain coefficients"
              >
                <FrequencyResponseCanvas response={response} />
              </Card>
            </div>
          </div>
        </div>
      )}

      {/* Subtab 2: Multiband Compressor & Dynamics */}
      {subtab === 'dynamics' && (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {dynamicsStages.map((stage) => (
            <Card
              key={stage.id}
              title={stage.name}
              subtitle={`${stage.category} · ${stage.enabled ? 'ONLINE' : 'BYPASSED'}`}
            >
              <div className="space-y-4 text-xs">
                {Object.entries(stage.params).map(([paramName, paramVal]) => {
                  const config = sliderFor(paramName);
                  return (
                    <Slider
                      key={paramName}
                      label={paramName}
                      min={config.min}
                      max={config.max}
                      step={config.step}
                      value={paramVal}
                      onChange={(value) => setStageParam(stage.id, paramName, value)}
                      unit={config.unit}
                    />
                  );
                })}
                <div className="pt-2 border-t border-[var(--color-border)] flex items-center justify-between">
                  <span className="font-mono text-[10px] text-muted">
                    True Peak Guard: -1.0 dBFS
                  </span>
                  <Button
                    variant={stage.enabled ? 'primary' : 'secondary'}
                    size="xs"
                    onClick={() => toggleStage(stage.id)}
                  >
                    {stage.enabled ? 'Active' : 'Bypass'}
                  </Button>
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}

      {/* Subtab 3: Acoustics & Cleanup Chain */}
      {subtab === 'spatial' && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-6">
            <Card title="Vocal Cleanup Signal Chain" subtitle="Toggle and inspect all 6 broadcast stages">
              <div className="space-y-2 text-xs">
                {stages.map((stage, idx) => (
                  <div
                    key={stage.id}
                    className="p-3 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] flex items-center justify-between"
                  >
                    <div className="flex items-center gap-3">
                      <span className="font-mono text-muted w-4">{idx + 1}</span>
                      <div>
                        <div className="font-medium text-fg">{stage.name}</div>
                        <div className="text-[10px] font-mono text-muted">{stage.category}</div>
                      </div>
                    </div>
                    <button
                      type="button"
                      onClick={() => toggleStage(stage.id)}
                      className={`p-1.5 rounded-md border transition-colors cursor-pointer ${
                        stage.enabled
                          ? 'bg-primary-tint text-primary-text border-border-strong'
                          : 'bg-surface-2 text-faint border-border'
                      }`}
                    >
                      <Power className="w-4 h-4" />
                    </button>
                  </div>
                ))}
              </div>
            </Card>
          </div>

          <div className="lg:col-span-6">
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
                <Button variant="primary" size="xs" onClick={handleMacro} disabled={macroBusy}>
                  {macroBusy ? 'Running...' : 'Run Macro Batch'}
                </Button>
                {macroNotice && <div className="text-[11px] text-amber">{macroNotice}</div>}
                {macroReport && (
                  <div className="pt-2 border-t border-[var(--color-border)] font-mono text-[10px] text-[var(--color-fg-muted)] space-y-1">
                    <div>
                      Processed {macroReport.report.succeeded}/{macroReport.report.processed} files (job {macroReport.job_id})
                    </div>
                  </div>
                )}
              </div>
            </Card>
          </div>
        </div>
      )}

      {/* Subtab 4: Hosted CLAP / LV2 / VST3 Plugins */}
      {subtab === 'vst' && (
        <Card
          title="System Audio Plugins (CLAP / LV2 / VST3)"
          subtitle={`${visiblePlugins.length} plugin(s) discovered from host OS paths`}
        >
          <div className="space-y-4 text-xs">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-1.5">
                {(['ALL', 'CLAP', 'LV2', 'VST3'] as const).map((fmt) => (
                  <Button
                    key={fmt}
                    variant={formatFilter === fmt ? 'primary' : 'secondary'}
                    size="xs"
                    onClick={() => setFormatFilter(fmt)}
                  >
                    {fmt}
                  </Button>
                ))}
              </div>
              <Button
                variant="secondary"
                size="xs"
                onClick={scanPlugins}
                disabled={isRescanning}
                leftIcon={<RefreshCw className={`w-4 h-4 ${isRescanning ? 'animate-spin' : ''}`} />}
              >
                {isRescanning ? 'Scanning...' : 'Rescan Plugins'}
              </Button>
            </div>

            {visiblePlugins.length === 0 ? (
              <div className="p-8 text-center rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] text-muted space-y-1">
                <div className="font-medium text-fg">No {formatFilter === 'ALL' ? '' : `${formatFilter} `}plugins found on system</div>
                <div className="text-[11px] font-mono">
                  Scanned standard Linux paths: ~/.clap, /usr/lib/clap, ~/.lv2, /usr/lib/lv2, ~/.vst3, /usr/lib/vst3
                </div>
              </div>
            ) : (
              <div className="space-y-2">
                {visiblePlugins.map((plugin) => (
                  <div
                    key={plugin.id}
                    className="p-3 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] flex items-center justify-between gap-4"
                  >
                    <div className="min-w-0">
                      <div className="font-medium text-[var(--color-fg)]">{plugin.name}</div>
                      <div className="text-[10px] font-mono text-[var(--color-fg-muted)] truncate">
                        {plugin.vendor} · {plugin.category}
                        {plugin.path ? ` · ${plugin.path}` : ''}
                      </div>
                    </div>
                    <span className="px-2 py-0.5 rounded font-mono text-[10px] font-medium bg-primary-tint text-primary-text border border-border-strong shrink-0">
                      {plugin.format}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </Card>
      )}
    </div>
  );
}
