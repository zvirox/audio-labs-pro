import React from 'react';
import {
  BookOpen,
  Film,
  Home,
  Layers,
  Mic,
  Settings,
  Sparkles,
  Wrench,
} from 'lucide-react';
import { Sidebar, type SidebarItem } from '../components/linear/Sidebar';

export interface WorkspaceItem {
  id: string;
  name: string;
  icon: React.ComponentType<{ className?: string; style?: React.CSSProperties }>;
  shortcut: string;
  desc: string;
}

export const DASHBOARD_ITEM: WorkspaceItem = {
  id: 'dashboard',
  name: 'Dashboard',
  icon: Home,
  shortcut: 'Ctrl+0',
  desc: 'Home overview with telemetry, studio metrics, and quick navigation',
};

export const WORKSPACE_LIST: WorkspaceItem[] = [
  { id: 'voice', name: 'Voice Studio', icon: Mic, shortcut: 'Ctrl+1', desc: 'Neural voice cloning, TTS synthesis, and pronunciation lexicon' },
  { id: 'multitrack', name: 'Multitrack DAW', icon: Layers, shortcut: 'Ctrl+2', desc: 'Non-linear multitrack arranger, clip envelopes, and zero-latency mixing' },
  { id: 'audiobook', name: 'Audiobook Lab', icon: BookOpen, shortcut: 'Ctrl+3', desc: 'Long-form narrative, multi-cast tagging, and ACX loudness compliance' },
  { id: 'dubbing', name: 'Video Dubbing', icon: Film, shortcut: 'Ctrl+4', desc: 'Video playback sync, stem isolation, and ADR dialog replacement' },
  { id: 'tools', name: 'Batch & Repair', icon: Wrench, shortcut: 'Ctrl+5', desc: 'Spectral de-noising, click removal, and batch audio normalizer' },
  { id: 'effects', name: 'DSP & Effects', icon: Sparkles, shortcut: 'Ctrl+6', desc: 'Parametric EQ, multiband compression, and VST/LV2 plugin hosting' },
  { id: 'settings', name: 'Preferences', icon: Settings, shortcut: 'Ctrl+7', desc: 'Audio hardware devices, buffer sizes, MCP agents, and window preferences' },
];

interface PrimaryRailProps {
  activeWorkspace: string;
  onSelectWorkspace: (id: string) => void;
  onOpenCommandPalette: () => void;
  onOpenShortcuts: () => void;
  collapsed?: boolean;
  onToggleCollapse?: () => void;
}

function toSidebarItem(ws: WorkspaceItem): SidebarItem {
  return {
    id: ws.id,
    label: ws.name,
    icon: ws.icon as SidebarItem['icon'],
  };
}

export function PrimaryRail({
  activeWorkspace,
  onSelectWorkspace,
  onOpenCommandPalette,
  onOpenShortcuts,
  collapsed = false,
  onToggleCollapse = () => {},
}: PrimaryRailProps) {
  const studioItems = WORKSPACE_LIST.filter((w) => w.id !== 'settings').map(toSidebarItem);
  const utilityItems = WORKSPACE_LIST.filter((w) => w.id === 'settings').map(toSidebarItem);

  return (
    <Sidebar
      dashboardItem={toSidebarItem(DASHBOARD_ITEM)}
      workspaces={studioItems}
      utilityItems={utilityItems}
      activeId={activeWorkspace}
      onSelect={onSelectWorkspace}
      collapsed={collapsed}
      onToggleCollapse={onToggleCollapse}
      onOpenCommandPalette={onOpenCommandPalette}
      onOpenShortcuts={onOpenShortcuts}
    />
  );
}
