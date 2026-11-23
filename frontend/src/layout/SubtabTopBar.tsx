import {
  RotateCcw,
  Sliders,
  PanelRight,
  Zap,
  LayoutGrid,
  AppWindow,
} from 'lucide-react';
import { Tabs } from '../kit/Tabs';
import { Button } from '../kit/Button';
import { Menu } from '../kit/Menu';
import type { PanelKind } from '../ipc/types';

interface SubtabTopBarProps {
  activeWorkspaceName: string;
  activeColor: string;
  activeSubtab: string;
  onSelectSubtab: (id: string) => void;
  subtabs: { id: string; label: string }[];
  isInspectorOpen: boolean;
  onToggleInspector: () => void;
  onTogglePanel: (kind: PanelKind) => void;
  onResetLayout: () => void;
  onTriggerEdit: () => void;
  isDirty?: boolean;
  viewMode?: 'workspace' | 'panels';
  onToggleViewMode?: () => void;
}

export function SubtabTopBar({
  activeWorkspaceName,
  activeColor,
  activeSubtab,
  onSelectSubtab,
  subtabs,
  isInspectorOpen,
  onToggleInspector,
  onTogglePanel,
  onResetLayout,
  onTriggerEdit,
  isDirty = false,
  viewMode = 'workspace',
  onToggleViewMode,
}: SubtabTopBarProps) {
  const panelMenuItems = [
    { id: 'timeline', label: 'Timeline / Tracks', onClick: () => onTogglePanel('timeline') },
    { id: 'mixer', label: 'Mixer', onClick: () => onTogglePanel('mixer') },
    { id: 'transport', label: 'Transport Controls', onClick: () => onTogglePanel('transport') },
    { id: 'inspector', label: 'Inspector Drawer', onClick: onToggleInspector, shortcut: 'Ctrl+B' },
    { id: 'voice_lab', label: 'Voice Lab', onClick: () => onTogglePanel('voice_lab'), dividerBefore: true },
    { id: 'effects', label: 'DSP Effects Rack', onClick: () => onTogglePanel('effects') },
    { id: 'models', label: 'Neural Models', onClick: () => onTogglePanel('models') },
    { id: 'media_bin', label: 'Media Bin', onClick: () => onTogglePanel('media_bin'), dividerBefore: true },
    { id: 'ai_activity', label: 'MCP Agent Activity', onClick: () => onTogglePanel('ai_activity') },
    { id: 'export_queue', label: 'Export Queue', onClick: () => onTogglePanel('export_queue') },
  ];

  return (
    <nav
      className="h-10 border-b border-[var(--color-border)] bg-[var(--color-surface-2)] px-4 flex items-center justify-between flex-shrink-0 select-none z-20"
      aria-label="Sub-navigation Bar"
    >
      {/* Left: Active Workspace Title & Contextual Subtabs */}
      <div className="flex items-center gap-4 overflow-x-auto">
        <div className="flex items-center gap-2 flex-shrink-0">
          <span
            className="w-2 h-2 rounded-full"
            style={{ backgroundColor: activeColor }}
          />
          <span className="text-xs font-bold tracking-tight uppercase text-[var(--color-fg)]">
            {activeWorkspaceName}
          </span>
        </div>

        <div className="h-3.5 w-[1px] bg-[var(--color-border)] flex-shrink-0" />

        <Tabs
          variant="subtab"
          items={subtabs}
          activeId={activeSubtab}
          onChange={onSelectSubtab}
        />
      </div>

      {/* Right: Actions, Panels Dropdown & Inspector Toggle */}
      <div className="flex items-center gap-2 flex-shrink-0 pl-2">
        {/* Trigger Demo Edit */}
        <Button
          variant="secondary"
          size="xs"
          onClick={onTriggerEdit}
          leftIcon={<Zap className="w-3 h-3 text-amber-400 fill-amber-400/20" />}
          title="Simulate timeline edit and emit revision event"
        >
          <span>Edit</span>
          {isDirty && (
            <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse ml-0.5" />
          )}
        </Button>

        {/* Floating Panels Menu */}
        <Menu
          align="right"
          trigger={
            <Button
              variant="secondary"
              size="xs"
              leftIcon={<Sliders className="w-3 h-3 text-[var(--color-accent)]" />}
            >
              Panels
            </Button>
          }
          items={panelMenuItems}
        />

        {/* View Mode Switcher (Workspaces vs Panels) */}
        {onToggleViewMode && (
          <Button
            variant={viewMode === 'workspace' ? 'secondary' : 'primary'}
            size="xs"
            onClick={onToggleViewMode}
            leftIcon={
              viewMode === 'workspace' ? (
                <AppWindow className="w-3 h-3 text-cyan-400" />
              ) : (
                <LayoutGrid className="w-3 h-3 text-amber-400" />
              )
            }
            title={
              viewMode === 'workspace'
                ? 'Switch to Floating Panels view'
                : 'Switch to Studio Workspace view'
            }
          >
            {viewMode === 'workspace' ? 'Panels' : 'Workspace'}
          </Button>
        )}

        {/* Reset Layout */}
        <Button
          variant="secondary"
          size="xs"
          onClick={onResetLayout}
          leftIcon={<RotateCcw className="w-3 h-3" />}
          title="Reset floating panel layout to default"
        >
          Reset
        </Button>

        {/* Right Inspector Toggle */}
        <Button
          variant={isInspectorOpen ? 'primary' : 'secondary'}
          size="xs"
          onClick={onToggleInspector}
          shortcut="Ctrl+B"
          leftIcon={<PanelRight className="w-3.5 h-3.5" />}
          title="Toggle right inspector drawer (Ctrl+B)"
        >
          Inspector
        </Button>
      </div>
    </nav>
  );
}
