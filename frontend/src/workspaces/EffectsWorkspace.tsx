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
import { getEffectsChain, getDiscoveredPlugins } from '../ipc/client';
import type { EffectStage, DiscoveredPlugin } from '../ipc/types';
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

  const activeStage = stages.find((s) => s.id === activeStageId) || stages[0];
  const enabledCount = stages.filter((s) => s.enabled).length;

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      {/* Workspace Header */}
      <div>
        <h2 className="text-xl font-bold tracking-tight text-slate-100 flex items-center gap-2">
          <Wand2 className="w-5 h-5 text-cyan-400" />
          Effects Chain & Hosted Plugins
        </h2>
        <p className="text-xs text-slate-400 mt-1">
          Polish vocal clarity with the 6-stage studio cleanup chain or host third-party CLAP and LV2 plugins inside the sandboxed DSP host.
        </p>
      </div>

      {/* Signal Path Metrics Banner */}
      <div className="grid grid-cols-3 gap-4">
        <div className="p-3.5 rounded-xl bg-slate-900/60 border border-white/10 flex items-center justify-between">
          <div>
            <div className="text-[11px] text-slate-400 font-medium">Active Cleanup Stages</div>
            <div className="text-lg font-bold font-mono text-cyan-400 mt-0.5">
              {enabledCount} / {stages.length} Active
            </div>
          </div>
          <Activity className="w-6 h-6 text-cyan-500/40" />
        </div>

        <div className="p-3.5 rounded-xl bg-slate-900/60 border border-white/10 flex items-center justify-between">
          <div>
            <div className="text-[11px] text-slate-400 font-medium">Mastering Target</div>
            <div className="text-lg font-bold font-mono text-purple-400 mt-0.5">-16.0 LUFS</div>
          </div>
          <Sliders className="w-6 h-6 text-purple-500/40" />
        </div>

        <div className="p-3.5 rounded-xl bg-slate-900/60 border border-white/10 flex items-center justify-between">
          <div>
            <div className="text-[11px] text-slate-400 font-medium">True Peak Ceiling</div>
            <div className="text-lg font-bold font-mono text-emerald-400 mt-0.5">-1.0 dBFS</div>
          </div>
          <Check className="w-6 h-6 text-emerald-500/40" />
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
                    className={`p-3 rounded-xl border transition-all cursor-pointer flex items-center justify-between ${
                      isSelected
                        ? 'bg-slate-900 border-cyan-500/60 shadow-md shadow-cyan-500/10'
                        : 'bg-slate-950/60 border-white/5 hover:border-white/15'
                    }`}
                  >
                    <div className="flex items-center gap-3">
                      <span className="font-mono text-slate-500 font-bold w-4">{idx + 1}</span>
                      <div>
                        <div className="font-semibold text-slate-200">{stage.name}</div>
                        <div className="text-[10px] font-mono text-slate-400">{stage.category}</div>
                      </div>
                    </div>

                    <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
                      <button
                        type="button"
                        onClick={() => toggleStage(stage.id)}
                        className={`p-1.5 rounded-lg border transition-colors ${
                          stage.enabled
                            ? 'bg-cyan-500/20 text-cyan-400 border-cyan-500/40'
                            : 'bg-white/5 text-slate-600 border-white/5'
                        }`}
                        title={stage.enabled ? 'Bypass stage' : 'Enable stage'}
                      >
                        <Power className="w-3.5 h-3.5" />
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
                  {Object.entries(activeStage.params).map(([paramName, paramVal]) => (
                    <div key={paramName}>
                      <Slider
                        label={paramName}
                        min={paramName.includes('Freq') ? 20 : -36}
                        max={paramName.includes('Freq') ? 20000 : 12}
                        step={0.5}
                        value={paramVal}
                        onChange={() => {}}
                        unit={paramName.includes('Freq') ? 'Hz' : 'dB'}
                      />
                    </div>
                  ))}
                </div>

                <div className="pt-3 border-t border-white/10 flex items-center justify-between text-[11px] text-slate-500 font-mono">
                  <span>64-bit double precision float internal DSP processing</span>
                  <span className={activeStage.enabled ? 'text-emerald-400' : 'text-slate-500'}>
                    {activeStage.enabled ? 'ONLINE (In Signal Path)' : 'BYPASS'}
                  </span>
                </div>
              </div>
            </Card>
          )}

          {/* Hosted CLAP / LV2 Plugins Card */}
          <Card title="Hosted Plugins (CLAP / LV2)" subtitle={`${plugins.length} discovered native Linux audio plugins`}>
            <div className="space-y-4 text-xs">
              <div className="flex items-center justify-between">
                <span className="text-slate-400 text-xs">
                  Plugins execute inside the isolated <code className="text-cyan-400">alp-plugin</code> sandbox.
                </span>
                <Button
                  variant="secondary"
                  size="xs"
                  onClick={handleRescan}
                  disabled={isRescanning}
                  leftIcon={
                    isRescanning ? (
                      <RefreshCw className="w-3 h-3 animate-spin" />
                    ) : (
                      <RefreshCw className="w-3 h-3" />
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
                    className="p-2.5 rounded-lg bg-slate-900 border border-white/10 flex items-center justify-between"
                  >
                    <div>
                      <div className="font-semibold text-slate-200">{plugin.name}</div>
                      <div className="text-[10px] font-mono text-slate-400">
                        {plugin.vendor} · {plugin.category}
                      </div>
                    </div>
                    <span className="px-2 py-0.5 rounded font-mono text-[10px] font-bold bg-purple-500/20 text-purple-300 border border-purple-500/30">
                      {plugin.format}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
