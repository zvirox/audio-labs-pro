import { Dialog } from '../kit/Dialog';
import { Button } from '../kit/Button';

interface ShortcutsModalProps {
  open: boolean;
  onClose: () => void;
}

const SHORTCUT_GROUPS = [
  {
    title: 'Navigation & Shell',
    items: [
      { keys: 'Ctrl + K', desc: 'Open Command Palette' },
      { keys: 'Ctrl + 0', desc: 'Dashboard Overview' },
      { keys: 'Ctrl + 1..7', desc: 'Switch Studio Workspace' },
      { keys: 'Ctrl + D', desc: 'Toggle Dark / Light Theme' },
      { keys: '?', desc: 'Open Keyboard Shortcuts' },
    ],
  },
  {
    title: 'Transport & Playback',
    items: [
      { keys: 'Space', desc: 'Play / Pause Session' },
      { keys: 'Shift + Space', desc: 'Stop and Return to Start' },
      { keys: 'L', desc: 'Toggle Loop Region' },
      { keys: 'Left / Right', desc: 'Nudge Playhead' },
      { keys: 'Ctrl + Home', desc: 'Jump to 00:00.00' },
    ],
  },
  {
    title: 'Timeline & Spectral Editing',
    items: [
      { keys: 'S', desc: 'Split Selected Clip at Playhead' },
      { keys: 'Delete', desc: 'Remove Selected Clip / Region' },
      { keys: 'M / O', desc: 'Toggle Track Mute / Solo' },
      { keys: '+ / -', desc: 'Zoom Timeline In / Out' },
      { keys: 'Ctrl + Z / Shift + Z', desc: 'Undo / Redo Session Mutation' },
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
          Close
        </Button>
      }
    >
      <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
        {SHORTCUT_GROUPS.map((group) => (
          <div
            key={group.title}
            className="flex flex-col gap-2.5 rounded-lg border border-[var(--border)] bg-[var(--bg-surface-2)] p-3.5"
          >
            <h3 className="text-[12px] font-semibold uppercase tracking-wider text-[var(--text-muted)]">
              {group.title}
            </h3>
            <div className="flex flex-col gap-2">
              {group.items.map((item) => (
                <div
                  key={item.keys}
                  className="flex items-center justify-between gap-2 text-[12px]"
                >
                  <span className="text-[var(--text)]">{item.desc}</span>
                  <kbd className="shrink-0 rounded border border-[var(--border-strong)] bg-[var(--bg-surface)] px-1.5 py-0.5 font-mono text-[11px] text-[var(--text-muted)]">
                    {item.keys}
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
