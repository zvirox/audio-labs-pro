# `alp-core::composite`

## Purpose

Groups several already-applied commands into a single undoable edit. This is
what makes a batch atomic from the user's point of view: reverting the group
walks the commands in reverse, re-applying walks them forward, and a failure
mid-group leaves the project exactly as it was before the group ran.

## Public API

- `CompositeCommand::new(label: &'static str, commands: Vec<Box<dyn Command>>) -> Self`
- `CompositeCommand::len(&self) -> usize`
- `CompositeCommand::is_empty(&self) -> bool`
- Implements `Command` (`label`, `apply`, `revert`) and `Debug`.

The label is `&'static str`, matching `Command::label`, so the composite appears
in the undo history with a stable descriptive name (for example `"Batch edit"`).

## Data flow

1. The caller executes N commands normally, so the project already reflects
   them, and records the stack depth before it started.
2. `CommandStack::take_last(count)` removes those entries; the caller passes
   them to `CompositeCommand::new`.
3. `CommandStack::push_applied` pushes the composite onto the undo stack without
   re-applying it (the project is already in the applied state).

`apply` iterates forward; if a command fails it reverts `commands[..applied]` in
reverse and returns the error. `revert` iterates in reverse.

## Allowed dependencies

- `crate::command::{Command, CommandError}`
- `crate::project::Project`
- `std::fmt`

No external crates.

## Threading / real-time notes

Owned by the session/UI thread like every other command; not synchronised and
never used on the audio callback.

## Error handling

`apply` returns the first failing command's `CommandError` after attempting to
undo the commands already applied in this group. `revert` propagates the first
error it meets. Commands are expected to be no-ops or harmless if reverted twice,
which is what lets the rollback loop ignore individual revert errors and still
return the original failure.

## Testing

Two unit tests in `composite.rs`:

- `composite_applies_and_reverts_as_one_step` — two `AddTrack` commands apply and
  revert together.
- `a_failing_command_leaves_the_project_untouched` — the second command fails and
  the first is rolled back, leaving no tracks.

## Known limits

- A group is immutable once built: there is no API to append or remove a command
  afterwards.
- `push_applied` trusts the caller that the commands were really applied; it
  does not verify their effects.
- Nested composites are allowed but not produced by any current code path.
