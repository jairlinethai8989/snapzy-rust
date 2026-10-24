# Duplicate Area And Object Duplication Implementation Plan

Approved design: `../specs/2026-10-21-duplicate-area-object-design.md`.

SnapZy Rust and Tauri Vanilla JS editor only. Preserve the dirty worktree and
all accepted behavior. Do not modify .NET, push, publish or replace the current
Alpha release. Build a separate local test candidate only after verification.

The `writing-plans` skill is not installed in this environment, so this reviewed
checklist is the implementation plan. Every production change begins with a
failing focused test.

## Implementation Checklist

- [x] **Baseline and boundaries:** inspect document identity allocation, nested
  ownership, history, image insertion, renderer snapshots, Tauri command routing,
  editor state and compact toolbar tests. Record the current passing focused tests.

- [x] **RED/GREEN - editable object duplication:** add failing Rust tests for
  every object category, child versus group duplication, exact property copying,
  independent IDs, `16,16` offset, sibling layer order, selection and atomic
  validation. Implement one document operation without changing project schema.

- [x] **RED/GREEN - group ownership and history:** add failing tests for complete
  subtree ID/parent remapping, nested groups, immutable asset sharing, limits,
  one-step Undo/Redo, dirty state and save/reopen. Implement atomic cloning and
  deterministic placement fallback.

- [x] **RED/GREEN - Duplicate Area service:** add failing service tests for exact
  committed pixels, rectangle normalization/clamping, one inserted image,
  selection/history, stale results and failure atomicity. Implement bounded
  render/crop/insert orchestration using existing image objects.

- [x] **RED/GREEN - Tauri editor state and commands:** add failing JavaScript and
  engine-smoke tests for the UI-only Duplicate Area mode, drag preview, Escape,
  command payload, Duplicate enablement, `Ctrl+D`, input-focus suppression and
  automatic transition to Select. Wire Rust bridge commands without adding a
  persisted tool enum.

- [x] **RED/GREEN - compact UI and localization:** add the two sharp icon-only
  controls, EN/TH tooltips and Help entries. Verify keyboard focus, high contrast,
  compact widths and stable toolbar alignment.

- [x] **Regression verification:** run formatting, focused tests, all JS tests,
  full Rust tests, release build and `git diff --check`. Recheck Text/Callout,
  Crop, shapes, layers, projects, OCR, long capture and PNG/copy workflows.

- [x] **Packaged acceptance candidate:** package a distinct local Tauri trial,
  calculate its SHA-256 and run packaged UI/engine smoke tests. Do not push or
  publish before the user completes hands-on testing and explicitly approves it.
