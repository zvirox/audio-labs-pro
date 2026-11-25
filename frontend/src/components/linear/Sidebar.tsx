import type { LucideIcon } from 'lucide-react';
import { Command, Keyboard, PanelLeftClose, PanelLeftOpen } from 'lucide-react';

export interface SidebarItem {
  id: string;
  label: string;
  icon: LucideIcon;
  badge?: string;
}

export interface SidebarProps {
  dashboardItem: SidebarItem;
  workspaces: SidebarItem[];
  activeId: string;
  onSelect: (id: string) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
  onOpenCommandPalette: () => void;
  onOpenShortcuts: () => void;
  utilityItems?: SidebarItem[];
}

export function Sidebar({
  dashboardItem,
  workspaces,
  activeId,
  onSelect,
  collapsed,
  onToggleCollapse,
  onOpenCommandPalette,
  onOpenShortcuts,
  utilityItems = [],
}: SidebarProps) {
  const renderNavItem = (item: SidebarItem) => {
    const Icon = item.icon;
    const isActive = activeId === item.id;
    return (
      <button
        key={item.id}
        type="button"
        onClick={() => onSelect(item.id)}
        title={collapsed ? item.label : undefined}
        aria-label={item.label}
        aria-current={isActive ? 'page' : undefined}
        className={`flex h-9 w-full items-center gap-2.5 rounded-md px-2.5 text-[13px] font-medium transition-colors duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] cursor-pointer ${
          collapsed ? 'justify-center px-0' : 'justify-start'
        } ${
          isActive
            ? 'bg-[var(--tint-blue)] text-[var(--accent-blue)] border border-[var(--border-strong)]'
            : 'border border-transparent text-[var(--text-muted)] hover:bg-[var(--bg-surface-2)] hover:text-[var(--text)]'
        }`}
      >
        <Icon className="h-4 w-4 shrink-0" />
        {!collapsed && (
          <>
            <span className="truncate">{item.label}</span>
            {item.badge && (
              <span className="ml-auto rounded border border-[var(--border)] bg-[var(--bg-surface-2)] px-1.5 py-0.5 text-[11px] font-medium leading-none text-[var(--text-muted)] tabular-nums">
                {item.badge}
              </span>
            )}
          </>
        )}
      </button>
    );
  };

  return (
    <aside
      aria-label="Workspace Sidebar"
      data-testid="linear-sidebar"
      className={`flex shrink-0 flex-col border-r border-[var(--border)] bg-[var(--bg-surface)] transition-[width] duration-[var(--duration-quick)] [transition-timing-function:var(--ease-out-expo)] ${
        collapsed ? 'w-[56px]' : 'w-[240px]'
      }`}
    >
      {/* Top Workspace Overview + Quick Search */}
      <div className="flex flex-col gap-1 border-b border-[var(--border)] p-2">
        {renderNavItem(dashboardItem)}
        <button
          type="button"
          onClick={onOpenCommandPalette}
          title={collapsed ? 'Command Palette (Ctrl+K)' : undefined}
          className={`flex h-9 w-full items-center gap-2.5 rounded-md border border-[var(--border)] bg-[var(--bg-surface-2)] px-2.5 text-[13px] text-[var(--text-muted)] hover:border-[var(--border-strong)] hover:text-[var(--text)] transition-colors duration-[var(--duration-quick)] cursor-pointer ${
            collapsed ? 'justify-center px-0' : 'justify-between'
          }`}
        >
          <span className="flex items-center gap-2">
            <Command className="h-3.5 w-3.5 shrink-0" />
            {!collapsed && <span>Search actions</span>}
          </span>
          {!collapsed && (
            <kbd className="rounded border border-[var(--border)] bg-[var(--bg-surface)] px-1.5 py-0.5 font-mono text-[11px] text-[var(--text-faint)]">
              Ctrl K
            </kbd>
          )}
        </button>
      </div>

      {/* Studio Switcher */}
      <nav aria-label="Studios" className="flex-1 overflow-y-auto p-2">
        {!collapsed && (
          <div className="px-2 pb-1.5 pt-1 text-[11px] font-medium uppercase tracking-wider text-[var(--text-faint)]">
            Studios
          </div>
        )}
        <div className="flex flex-col gap-0.5">
          {workspaces.map(renderNavItem)}
        </div>
      </nav>

      {/* Bottom Utility Navigation (NO ThemeToggle — lives in TitleBar only) */}
      <div className="flex flex-col gap-0.5 border-t border-[var(--border)] p-2">
        {utilityItems.map(renderNavItem)}
        <button
          type="button"
          onClick={onOpenShortcuts}
          title={collapsed ? 'Keyboard Shortcuts (?)' : undefined}
          aria-label="Keyboard Shortcuts"
          className={`flex h-9 w-full items-center gap-2.5 rounded-md px-2.5 text-[13px] font-medium text-[var(--text-muted)] hover:bg-[var(--bg-surface-2)] hover:text-[var(--text)] transition-colors duration-[var(--duration-quick)] cursor-pointer ${
            collapsed ? 'justify-center px-0' : 'justify-start'
          }`}
        >
          <Keyboard className="h-4 w-4 shrink-0" />
          {!collapsed && <span>Shortcuts</span>}
        </button>
        <button
          type="button"
          onClick={onToggleCollapse}
          title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          className={`flex h-9 w-full items-center gap-2.5 rounded-md px-2.5 text-[13px] font-medium text-[var(--text-muted)] hover:bg-[var(--bg-surface-2)] hover:text-[var(--text)] transition-colors duration-[var(--duration-quick)] cursor-pointer ${
            collapsed ? 'justify-center px-0' : 'justify-start'
          }`}
        >
          {collapsed ? (
            <PanelLeftOpen className="h-4 w-4 shrink-0" />
          ) : (
            <>
              <PanelLeftClose className="h-4 w-4 shrink-0" />
              <span>Collapse</span>
            </>
          )}
        </button>
      </div>
    </aside>
  );
}
