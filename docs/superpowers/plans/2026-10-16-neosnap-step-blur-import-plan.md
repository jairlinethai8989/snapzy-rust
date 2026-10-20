# Editable .neosnap Step And Blur Import Plan

Approved design: `../specs/2026-10-15-neosnap-step-blur-import-design.md`.
Rust only. Preserve the dirty worktree. Do not modify .NET, commit, push, publish
or replace the accepted 0.11.0-alpha.1 package.

- [x] RED: importer rejects current `number`/`blur` fixtures for the expected
  unsupported-tool reason; add accepted mapping assertions before production edits.
- [x] GREEN: parse and validate Number/Blur source fields, source ranges, owner
  scaling, effects, opacity, clipping and all-or-nothing diagnostics.
- [x] RED/GREEN: add DotNet Step radius/ring/digit behavior and editable source
  size while leaving Native Step diameter behavior unchanged.
- [x] RED/GREEN: expand Blur to 1-64 px, editable opacity and bounded halo/tile
  rendering while preserving Native default 8 px.
- [x] RED/GREEN: persist imported Step/Blur source rendering and values through
  schema 3 Save/Open, ownership, Undo/Redo, layers, Copy and PNG rendering.
- [x] Update EN/TH help, importer support text, version, README and test notes.
- [x] Run formatting, locked check/full tests, optimized build and diff checks.
- [x] Package and hash a separate local candidate.
- [ ] Await native user testing before any commit, push or publication.

The writing-plans skill is not installed in this environment, so this reviewed
local checklist is the implementation plan. Any newly discovered source value
that cannot be retained fails closed instead of being clamped or flattened.
