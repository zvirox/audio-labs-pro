# `alp-ui` — `jobs`

## Purpose

`jobs` is the cooperative worker queue behind import, peak verification and
export stubs. It proves the Phase 2 requirement that long work never blocks the
UI thread and remains cancellable.

## Public API

- `JobStatus { Running, Completed, Cancelled, Failed }` with `is_running`.
- `Job { id, name, progress, detail, status }`.
- `JobReporter` with `progress`, `detail` and `is_cancelled`.
- `JobQueue` with `spawn`, `spawn_demo_export`, `poll`, `cancel`,
  `jobs`, `has_running`, `running_count`, `clear_finished`; `Default`.

## Data flow

```text
UI spawn ──► std::thread ──► work(JobReporter) ──► mpsc events
                                                   │
UI frame ──► JobQueue::poll (try_recv) ◄───────────┘
                 │
                 └─► Job status/progress ──► Export panel
```

Each job owns a stable id, a `Sender` clone and an `AtomicBool` cancellation
flag. `poll` drains all queued events without blocking and updates only running
jobs. `Finished` removes the cancellation entry.

## Allowed dependencies

`std` only (`thread`, `mpsc`, `AtomicBool`, `Arc`). No iced or ALP API appears in
the worker contract, so the same queue can drive real decoders and exporters
later.

## Threading / real-time notes

Work runs on standard threads. The UI calls `poll` only from a frame, and frames
are subscribed only while `has_running()` is true (or diagnostics are enabled),
so an idle queue costs nothing. The audio callback is never touched. Progress is
coalesced to 0.1% by sending an integer.

## Error handling

A worker returns `Result<String, String>`; a `"cancelled"` error maps to
`Cancelled`, any other error to `Failed` with its detail preserved. A dropped
sender or a missing job id is ignored. Progress is clamped to 0..=1.

## Testing

Four tests run workers and wait for completion with a deadline: progress and
completion, cooperative cancellation, failure text, and clearing finished jobs.
No test asserts timing beyond a three-second deadline.

## Known limits

- Threads are unbounded; a real import/export scheduler should cap concurrency.
- Cancellation is cooperative; a worker that never checks the flag cannot be
  stopped.
- There is no panic recovery: a panicking worker leaves the job running until
  the thread's sender drops, and then the UI has no `Finished` event. A future
  version should catch unwinds or use a supervisor.
- Jobs are in-memory only; they are not restored after a restart.
