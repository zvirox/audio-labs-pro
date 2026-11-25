import { Tabs } from '../kit/Tabs';

interface SubtabTopBarProps {
  activeSubtab: string;
  onSelectSubtab: (id: string) => void;
  subtabs: { id: string; label: string }[];
}

export function SubtabTopBar({
  activeSubtab,
  onSelectSubtab,
  subtabs,
}: SubtabTopBarProps) {
  if (subtabs.length === 0) return null;

  return (
    <nav
      className="flex h-10 shrink-0 items-center border-b border-[var(--border)] bg-[var(--bg-surface-2)] px-4 select-none"
      aria-label="Sub-navigation Bar"
    >
      <Tabs
        variant="subtab"
        size="sm"
        items={subtabs.map((s) => ({ id: s.id, label: s.label }))}
        activeId={activeSubtab}
        onChange={onSelectSubtab}
      />
    </nav>
  );
}
