/**
 * Audio Labs Pro - Models Workspace
 *
 * Local neural model catalogue, SPDX license verification, offline download policy,
 * integrity verification (SHA-256), and hardware acceleration telemetry.
 */

import { useState, useEffect } from 'react';
import { Card } from '../kit/Card';
import { Button } from '../kit/Button';
import { Toggle } from '../kit/Toggle';
import { getModelCatalogue, getHardwareProfile, verifyModelIntegrity } from '../ipc/client';
import type { HardwareProfile, ModelItem } from '../ipc/types';
import {
  Brain,
  Download,
  Shield,
  CheckCircle2,
  HardDrive,
  Cpu,
  RefreshCw,
} from 'lucide-react';

export function ModelsWorkspace() {
  const [models, setModels] = useState<ModelItem[]>([]);
  const [hardware, setHardware] = useState<HardwareProfile | null>(null);
  const [offlineOnly, setOfflineOnly] = useState<boolean>(true);
  const [verifyingId, setVerifyingId] = useState<string | null>(null);
  const [verifyMessage, setVerifyMessage] = useState<string | null>(null);

  useEffect(() => {
    getModelCatalogue().then(setModels).catch(() => {});
    getHardwareProfile().then(setHardware).catch(() => {});
  }, []);

  const handleVerify = async (id: string) => {
    setVerifyingId(id);
    setVerifyMessage(null);
    try {
      const updated = await verifyModelIntegrity(id);
      setModels((prev) => prev.map((m) => (m.id === id ? updated : m)));
      setVerifyMessage(`${updated.name}: ${updated.status}`);
    } catch (error) {
      setVerifyMessage(`Verify failed for ${id}: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setVerifyingId(null);
    }
  };

  return (
    <div className="space-y-6 max-w-6xl mx-auto p-2 pb-12 select-none">
      {/* Workspace Header */}
      <div>
        <h2 className="text-xl font-medium text-[var(--color-fg)] flex items-center gap-2">
          <Brain className="w-4 h-4 text-[var(--color-fg-muted)]" />
          Neural Models & Hardware Engine
        </h2>
        <p className="text-xs text-[var(--color-fg-muted)] mt-1">
          Manage local AI speech models, offline verification, SPDX licenses, and on-device hardware accelerators.
        </p>
      </div>

      {verifyMessage && (
        <div className="p-3 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-strong)] text-xs font-mono text-[var(--color-fg-muted)]">
          {verifyMessage}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Model Catalogue (2 cols) */}
        <div className="lg:col-span-2 space-y-4">
          <Card title="Local Model Catalogue" subtitle="Installed neural weights with SPDX license attribution">
            <div className="space-y-3">
              {models.map((model) => (
                <div
                  key={model.id}
                  className="p-4 rounded-lg bg-[var(--bg-surface)] border border-[var(--border)] hover:border-[var(--border-strong)] transition-colors space-y-2 text-xs"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-[var(--text)] text-sm">{model.name}</span>
                      {model.is_builtin && (
                        <span className="px-1.5 py-0.5 rounded bg-primary-tint text-primary-text border border-border-strong font-mono text-[10px]">
                          Built-in
                        </span>
                      )}
                    </div>
                    <span
                      className={`px-2 py-0.5 rounded font-mono text-[11px] font-medium border ${
                        model.status === 'verified'
                          ? 'bg-chip-sky text-sky-text border-border-strong'
                          : model.status === 'installed'
                          ? 'bg-primary-tint text-primary-text border-border-strong'
                          : 'bg-surface-2 text-amber border-amber'
                      }`}
                    >
                      {model.status.toUpperCase()}
                    </span>
                  </div>

                  <p className="text-[var(--color-fg-muted)] text-xs">{model.description}</p>

                  <div className="pt-2 border-t border-[var(--color-border)] flex flex-wrap items-center justify-between gap-2 font-mono text-[11px] text-[var(--color-fg-muted)]">
                    <div className="flex items-center gap-3">
                      <span>Size: {model.size_mb} MB</span>
                      <span>License: {model.spdx}</span>
                    </div>

                    <div className="flex items-center gap-2">
                      <Button
                        variant="secondary"
                        size="xs"
                        onClick={() => handleVerify(model.id)}
                        disabled={verifyingId === model.id}
                        leftIcon={
                          verifyingId === model.id ? (
                            <RefreshCw className="w-4 h-4 animate-spin" />
                          ) : (
                            <CheckCircle2 className="w-4 h-4 text-[var(--color-fg-muted)]" />
                          )
                        }
                      >
                        {verifyingId === model.id ? 'Checking SHA-256...' : 'Verify Hash'}
                      </Button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </Card>
        </div>

        {/* Policy & Hardware Telemetry (1 col) */}
        <div className="space-y-6">
          {/* Download & Offline Policy */}
          <Card title="Download Policy" subtitle="Network boundary constraints">
            <div className="space-y-4 text-xs">
              <div className="flex items-center justify-between py-1">
                <div>
                  <div className="font-medium text-[var(--color-fg)]">Offline Enforcement</div>
                  <div className="text-[var(--color-fg-muted)] text-[11px]">Strictly zero outbound web traffic</div>
                </div>
                <Toggle checked={offlineOnly} onChange={setOfflineOnly} label="" />
              </div>

              <div className="p-3 rounded-lg bg-[var(--color-bg)] border border-[var(--color-border)] space-y-2">
                <div className="flex items-center gap-1.5 text-[var(--color-fg-muted)] font-medium">
                  <Shield className="w-4 h-4 text-[var(--color-fg-muted)]" />
                  <span>Integrity Safeguards</span>
                </div>
                <p className="text-[var(--color-fg-muted)] text-[11px] leading-relaxed">
                  Audio Labs Pro does not fetch neural weights silently. Models must be explicitly placed in the storage folder and audited with SHA-256 hashes via <code className="text-[var(--color-accent)]">alp-cli model verify</code>.
                </p>
              </div>
            </div>
          </Card>

          {/* Hardware Accelerators */}
          <Card title="Hardware Telemetry" subtitle="Probed host compute and memory profile">
            <div className="space-y-3 text-xs font-mono">
              <div className="flex items-center justify-between">
                <span className="text-[var(--color-fg-muted)] flex items-center gap-1.5">
                  <Cpu className="w-4 h-4 text-[var(--color-accent)]" />
                  CPU Cores:
                </span>
                <span className="text-[var(--color-fg)]">
                  {hardware ? `${hardware.cpu_cores} logical threads` : 'Probing...'}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[var(--color-fg-muted)] flex items-center gap-1.5">
                  <HardDrive className="w-4 h-4 text-[var(--color-fg-muted)]" />
                  System RAM:
                </span>
                <span className="text-[var(--color-fg)]">
                  {hardware ? `${hardware.available_ram_gb} / ${hardware.total_ram_gb} GB free` : 'Probing...'}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[var(--color-fg-muted)] flex items-center gap-1.5">
                  <Download className="w-4 h-4 text-[var(--color-fg-muted)]" />
                  Recommended Tier:
                </span>
                <span className="text-[var(--color-fg)] font-medium">
                  {hardware?.recommended_tier || 'Fast (Lightweight)'}
                </span>
              </div>
              {hardware?.gpu_description && (
                <div className="pt-2 border-t border-[var(--color-border)] text-[10px] text-[var(--color-fg-muted)] truncate" title={hardware.gpu_description}>
                  GPU: {hardware.gpu_description}
                </div>
              )}
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
