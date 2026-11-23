import { MoreHorizontal, PanelRight } from 'lucide-react';
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
  hideStudioActions?: boolean;
}

/**
 * Contextual sub-navigation bar. Keeps a single scrolling row of subtabs and
 * collapses the studio's secondary actions into one overflow menu so the bar
 * stays uncluttered at narrow widths.
 */
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
  hideStudioActions = false,
}: SubtabTopBarProps) {
  const studioMenuItems = [
    { id: 'edit', label: 'Trigger demo timeline edit', onClick: onTriggerEdit },
    {
      id: 'viewmode',
      label: viewMode === 'workspace' ? 'Switch to floating panels' : 'Switch to workspace view',
      onClick: () => onToggleViewMode?.(),
    },
    { id: 'reset', label: 'Reset panel layout', onClick: onResetLayout, dividerBefore: true },
    { id: 'p_timeline', label: 'Toggle Timeline panel', onClick: () => onTogglePanel('timeline') },
    { id: 'p_mixer', label: 'Toggle Mixer panel', onClick: () => onTogglePanel('mixer') },
    { id: 'p_transport', label: 'Toggle Transport panel', onClick: () => onTogglePanel('transport') },
    { id: 'p_voice', label: 'Toggle Voice Lab panel', onClick: () => onTogglePanel('voice_lab'), dividerBefore: true },
    { id: 'p_effects', label: 'Toggle DSP Effects panel', onClick: () => onTogglePanel('effects') },
    { id: 'p_models', label: 'Toggle Neural Models panel', onClick: () => onTogglePanel('models') },
    { id: 'p_media', label: 'Toggle Media Bin panel', onClick: () => onTogglePanel('media_bin'), dividerBefore: true },
    { id: 'p_ai', label: 'Toggle MCP Agent Activity panel', onClick: () => onTogglePanel('ai_activity') },
    { id: 'p_export', label: 'Toggle Export Queue panel', onClick: () => onTogglePanel('export_queue') },
  ];

  return (
    <nav
      className="h-10 border-b border-[var(--color-border)] bg-[var(--color-surface-2)] px-3 flex items-center gap-3 flex-shrink-0 select-none z-20"
      aria-label="Sub-navigation Bar"
    >
      {/* Left: workspace title + single-line scrolling subtabs */}
      <div className="flex flex-1 min-w-0 items-center gap-3">
        <div className="flex items-center gap-2 flex-shrink-0">
          <span
            className="w-2 h-2 rounded-full"
            style={{ backgroundColor: activeColor }}
            aria-hidden="true"
          />
          <span className="text-xs font-bold tracking-tight uppercase text-[var(--color-fg)] whitespace-nowrap">
            {activeWorkspaceName}
          </span>
          {isDirty && (
            <span
              className="w-1.5 h-1.5 rounded-full bg-amber-400"
              title="Unsaved changes"
              aria-label="Unsaved changes"
            />
          )}
        </div>

        {subtabs.length > 0 && (
          <div className="h-3.5 w-[1px] bg-[var(--color-border)] flex-shrink-0" aria-hidden="true" />
        )}

        <div className="min-w-0 flex-1 overflow-hidden">
          <Tabs
            variant="subtab"
            items={subtabs}
            activeId={activeSubtab}
            onChange={onSelectSubtab}
          />
        </div>
      </div>

      {/* Right: one overflow menu + inspector toggle */}
      {!hideStudioActions && (
        <div className="flex items-center gap-1.5 flex-shrink-0">
          <Menu
            align="right"
            trigger={
              <Button
                variant="secondary"
                size="xs"
                leftIcon={<MoreHorizontal className="w-3.5 h-3.5" />}
                aria-label="Studio actions and panels"
                title="Studio actions and panels"
              >
                Studio
              </Button>
            }
            items={studioMenuItems}
          />
          <Button
            variant={isInspectorOpen ? 'primary' : 'secondary'}
            size="xs"
            onClick={onToggleInspector}
            aria-label={isInspectorOpen ? 'Close inspector' : 'Open inspector'}
            aria-pressed={isInspectorOpen}
            title="Toggle inspector (Ctrl+B)"
            leftIcon={<PanelRight className="w-3.5 h-3.5" />}
          />
        </div>
      )}
    </nav>
  );
}
