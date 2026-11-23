import { Dialog } from '../kit/Dialog';
import { Button } from '../kit/Button';

interface ShortcutsModalProps {
  open: boolean;
  onClose: () => void;
}

interface ShortcutCategory {
  title: string;
  items: { key: string; desc: string }[];
}

const SHORTCUT_CATEGORIES: ShortcutCategory[] = [
  {
    title: 'Workspaces & Navigation',
    items: [
      { key: 'Ctrl+1', desc: 'Switch to Voice Studio Lab' },
      { key: 'Ctrl+2', desc: 'Switch to Multitrack Arranger DAW' },
      { key: 'Ctrl+3', desc: 'Switch to Audiobook Production Lab' },
      { key: 'Ctrl+4', desc: 'Switch to Video Dubbing & ADR' },
      { key: 'Ctrl+5', desc: 'Switch to Audio Tools, Batch & Repair' },
      { key: 'Ctrl+6', desc: 'Switch to DSP Effects & Plugins' },
      { key: 'Ctrl+7', desc: 'Switch to Preferences & Audio Devices' },
    ],
  },
  {
    title: 'Window, Shell & System',
    items: [
      { key: 'Ctrl+K', desc: 'Open Command Palette' },
      { key: 'F1', desc: 'Open Keyboard Shortcuts (this cheatsheet)' },
      { key: 'Ctrl+D', desc: 'Cycle Studio Theme (Studio Dark / Light)' },
      { key: 'Ctrl+Shift+M', desc: 'Toggle Reduce Motion preference' },
      { key: 'Ctrl+B', desc: 'Toggle Collapsible Right Inspector Drawer' },
      { key: 'Esc', desc: 'Dismiss active dialog or floating panel' },
    ],
  },
  {
    title: 'Transport & Playback',
    items: [
      { key: 'Space', desc: 'Toggle Play / Pause transport' },
      { key: 'R', desc: 'Arm and begin recording on active track' },
      { key: 'Home', desc: 'Return playhead to project timeline start (0:00)' },
      { key: 'End', desc: 'Move playhead to timeline end' },
      { key: 'L', desc: 'Toggle Loop playback mode' },
    ],
  },
  {
    title: 'Timeline Editing & DSP',
    items: [
      { key: 'Ctrl+Z', desc: 'Undo last timeline edit or operation' },
      { key: 'Ctrl+Shift+Z', desc: 'Redo previously undone operation' },
      { key: 'Ctrl+X', desc: 'Cut selected clip or audio region to clipboard' },
      { key: 'Ctrl+C', desc: 'Copy selected clip to clipboard' },
      { key: 'Ctrl+V', desc: 'Paste clipboard audio at playhead position' },
      { key: 'S', desc: 'Split clip under playhead position' },
      { key: 'Delete', desc: 'Delete selected audio clip or range' },
      { key: 'Ctrl+N', desc: 'Normalize selected clip to -0.2 dBFS' },
      { key: 'Ctrl+E', desc: 'Open Export Master bounce dialog' },
    ],
  },
];

export function ShortcutsModal({ open, onClose }: ShortcutsModalProps) {
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Studio Keyboard Shortcuts"
      description="Complete keyboard navigation and editing hotkeys for Audio Labs Pro."
      size="xl"
      footer={
        <Button variant="primary" size="sm" onClick={onClose}>
          Got it
        </Button>
      }
    >
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {SHORTCUT_CATEGORIES.map((cat) => (
          <div key={cat.title} className="space-y-2.5">
            <h3 className="text-xs font-medium text-[var(--color-accent)]">
              {cat.title}
            </h3>
            <div className="space-y-1.5">
              {cat.items.map((item) => (
                <div
                  key={item.key}
                  className="flex items-center justify-between p-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] text-xs"
                >
                  <span className="text-[var(--color-fg-muted)]">{item.desc}</span>
                  <kbd className="px-2 py-0.5 rounded bg-[var(--color-surface-3)] font-mono text-[10px] text-[var(--color-fg)] font-medium border border-[var(--color-border)]">
                    {item.key}
                  </kbd>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </Dialog>
  );
}
