# Save Video After Recording

User correction on 2026-10-14: the 15-second recording works, but selecting
a filename before countdown is the wrong workflow. This supersedes that part
of the Stage One specification and authorizes the following focused change.

- Select monitor/window, prepare, count down 3-2-1, then record immediately.
- Stream to a unique instance-owned file under Recordings/Pending. Retain
  completed and nonempty partial files independently of IPC cleanup.
- Stop/Esc finalizes the clip before opening the localized Save video chooser.
- Cancel keeps the completed clip and shows its location. A chooser or copy
  failure keeps the completed original; cancelling save also cancels pending exit.
- Save moves the existing file on the same volume, without allocating another
  full-size clip. Across volumes it copies with bounded memory on a background
  thread, flushes and publishes without replacing an existing destination.
  Remove only the original owned file after successful copy/publication and
  an identity check.
- Block competing capture and explicit Exit while saving. Existing source-loss,
  countdown cancellation, low-space stops and project exit checks stay active.
- Preserve old packages, build a separate after-recording-save candidate with
  the current version. No .NET changes, audio, preview, commit or push.

Verify real file transfer, Unicode paths, destination collision, missing/empty
source and source retention on error. Run the full suite and release checks.
Record native test gaps explicitly and include the new workflow in EN/TH notes.
