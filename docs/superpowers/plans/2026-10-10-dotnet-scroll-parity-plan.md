# Scrolling parity implementation

1. Add failing tests for the .NET sheet heuristic and hover confirmation state.
2. Port sheet detection as a pure BGRA function, preserving screenshot pixels.
3. Change provider probing to target-root UIA traversal with Document/window
   fallback, native child hit-testing and bounded isolated worker lifecycle.
4. Add debounced hover probing to a non-activating overlay, visible frame and
   click-release confirmation. Keep legacy buttons as optional alternatives.
5. Extend native smoke checks for hover before click, no hide/focus changes,
   Escape, stale results, automatic/manual capture and exact pixels.
6. Build/test/package a new local Alpha 6; report remaining hands-on checks.

Only scoped edits; preserve the dirty working tree and do not push.

## Implementation Status

- Steps 1-5 implemented. The new hover-before-click regression failed on Alpha 5
  and passed on Alpha 6. Sheet/state tests include negative origins, cancellation,
  stale generations and held-frame identity.
- Added a real WPF ScrollViewer provider fixture under a covering test window.
  Its initial failure exposed the cold-provider 250 ms budget problem; the
  1500 ms soft budget passed while retaining the four-second hard supervisor.
- Windows x64 unit suite: 127 passed, zero failed. The tester's supplied Lark
  screenshot was also run explicitly through the grid detector and passed.
- Step 6: local portable Alpha 6 packaging and final evidence recorded in
  `docs/testing/0.9.0-alpha.6.md`. Actual Lark/Excel and hardware-input/DPI
  acceptance are still pending; no Git push or .NET UI redesign in this increment.
