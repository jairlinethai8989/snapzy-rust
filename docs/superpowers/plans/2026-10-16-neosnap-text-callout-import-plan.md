# Editable .neosnap Text And Speech-Bubble Import Plan

Approved design: `../specs/2026-10-16-neosnap-text-callout-import-design.md`.
Rust only. Preserve the dirty worktree. Do not modify .NET, commit, push,
publish or replace the accepted `0.12.0-alpha.1` package.

- [x] RED/GREEN: add strict source fields and import editable plain text with
  retained anchor, source font layout, explicit newlines, opacity and effects.
- [x] RED/GREEN: add editable speech-bubble frame, independent fill/border/text
  colors, border width, tail geometry and source wrapping behavior.
- [x] RED/GREEN: reverse owned-image scale for all text/bubble geometry and
  preserve clipping and layer order with all-or-nothing failure.
- [x] RED/GREEN: advance `.snapzy` to schema 4; round-trip imported layout and
  keep schema 1-3 native text/callout behavior unchanged.
- [x] RED/GREEN: extend editing, resizing, effects and compact controls for the
  imported font range and independent bubble border.
- [x] RED/GREEN: verify preview, Copy/PNG, Undo/Redo and failure diagnostics.
- [x] Update EN/TH help, importer support text, version, README and test notes.
- [x] Run formatting, locked check/full tests, optimized build and diff checks.
- [x] Package and hash a separate `0.13.0-alpha.1` local candidate.
- [x] Native user testing passed on 2026-10-16; await explicit approval before
  any commit, push or publication.

The writing-plans skill is not installed in this environment, so this reviewed
local checklist is the implementation plan. Newly discovered source values that
cannot be retained fail closed instead of being clamped or flattened.
