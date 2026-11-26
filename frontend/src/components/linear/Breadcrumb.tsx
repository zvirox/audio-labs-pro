import {
  Activity,
  ChevronRight,
  Columns2,
  FolderSync,
  LayoutGrid,
  Plus,
  RotateCcw,
  SlidersHorizontal,
} from 'lucide-react';
import { StatusPill } from './StatusPill';
import { LinearButton } from './LinearButton';
import type { PanelKind } from '../../ipc/types';

export interface BreadcrumbProps {
  workspaceName: string;
  subtabLabel?: string;
  isNative: boolean;
  revision?: number;
  isDirty?: boolean;
  isLoading?: boolean;
  onSync: () => void;
  isInspectorOpen: boolean;
  onToggleInspector: () => void;
  onTogglePanel: (kind: PanelKind) => void;
  onResetLayout: () => void;
  onTriggerEdit: () => void;
  viewMode: 'workspace' | 'panels';
  onToggleViewMode: () => void;
  hideStudioActions?: boolean;
}

export function Breadcrumb({
  workspaceName,
  subtabLabel,
  isNative,
  revision,
  isDirty,
  isLoading,
  onSync,
  isInspectorOpen,
  onToggleInspector,
  onTogglePanel,
  onResetLayout,
  onTriggerEdit,
  viewMode,
  onToggleViewMode,
  hideStudioActions = false,
}: BreadcrumbProps) {
  return (
    <div
      data-testid="linear-breadcrumb"
      className="flex h-11 shrink-0 items-center justify-between gap-4 border-b border-border bg-app px-4"
    >
      {/* Left: Linear Breadcrumb Path + Session State */}
      <div className="flex min-w-0 items-center gap-1.5 text-[13px]">
        <span className="shrink-0 font-medium text-muted">Audio Labs Pro</span>
        <ChevronRight className="h-3.5 w-3.5 shrink-0 text-faint" />
        <span className="truncate font-semibold text-fg">{workspaceName}</span>
        {subtabLabel && (
          <>
            <ChevronRight className="hidden md:inline h-3.5 w-3.5 shrink-0 text-faint" />
            <span className="hidden md:inline truncate font-medium text-muted">
              {subtabLabel}
            </span>
          </>
        )}
        <div className="ml-2 hidden shrink-0 items-center gap-1.5 xl:flex">
          <StatusPill
            variant={isNative ? 'success' : 'neutral'}
            label={isNative ? 'Native' : 'Preview'}
            className="whitespace-nowrap"
          />
          {revision !== undefined && (
            <span className="whitespace-nowrap rounded border border-border bg-surface-2 px-1.5 py-0.5 font-mono text-[11px] text-muted tabular-nums">
              rev {revision}
            </span>
          )}
          {isDirty && (
            <span className="whitespace-nowrap rounded border border-amber bg-surface-2 px-1.5 py-0.5 text-[11px] font-medium text-amber">
              Unsaved
            </span>
          )}
        </div>
      </div>

      {/* Right: Studio Controls */}
      <div className="flex shrink-0 items-center gap-1.5">
        {!hideStudioActions && (
          <>
            <LinearButton
              size="sm"
              variant="secondary"
              onClick={onSync}
              disabled={isLoading}
              title="Refresh session snapshot"
            >
              <FolderSync className={`h-3.5 w-3.5 ${isLoading ? 'animate-spin' : ''}`} />
              <span className="hidden 2xl:inline">Sync</span>
            </LinearButton>
            <LinearButton
              size="sm"
              variant="secondary"
              onClick={onTriggerEdit}
              title="Add sample track"
            >
              <Plus className="h-3.5 w-3.5" />
              <span className="hidden 2xl:inline">Add Track</span>
            </LinearButton>
            <LinearButton
              size="sm"
              variant={viewMode === 'panels' ? 'primary' : 'secondary'}
              onClick={onToggleViewMode}
              title="Toggle Split Panel View"
            >
              {viewMode === 'panels' ? (
                <LayoutGrid className="h-3.5 w-3.5" />
              ) : (
                <Columns2 className="h-3.5 w-3.5" />
              )}
              <span className="hidden lg:inline">
                {viewMode === 'panels' ? 'Studio View' : 'Panels'}
              </span>
            </LinearButton>
            <LinearButton
              size="sm"
              variant="ghost"
              onClick={() => onTogglePanel('timeline')}
              title="Open Timeline Panel"
            >
              <Activity className="h-3.5 w-3.5" />
            </LinearButton>
            <LinearButton
              size="sm"
              variant="ghost"
              onClick={onResetLayout}
              title="Reset Panel Layout"
            >
              <RotateCcw className="h-3.5 w-3.5" />
            </LinearButton>
          </>
        )}
        <LinearButton
          size="sm"
          variant={isInspectorOpen ? 'primary' : 'secondary'}
          onClick={onToggleInspector}
          title="Toggle Context Inspector"
        >
          <SlidersHorizontal className="h-3.5 w-3.5" />
          <span className="hidden lg:inline">Inspector</span>
        </LinearButton>
      </div>
    </div>
  );
}
