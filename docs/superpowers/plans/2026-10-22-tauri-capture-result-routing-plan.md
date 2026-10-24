# Tauri Capture Result Routing Implementation Plan

Approved design: `../specs/2026-10-22-tauri-capture-result-routing-design.md`.

SnapZy Rust/Tauri only. Preserve unrelated dirty work and native behavior. Do
not modify .NET, push or publish. Produce a separate local installer only after
verification. The `writing-plans` skill is not installed in this environment,
so this reviewed checklist is the implementation plan. Every production change
begins with a focused failing test.

## Implementation Checklist

- [x] **Baseline and boundaries:** record focused Rust and JavaScript baselines;
  inspect capture completion, engine lifetime, editor service ownership, Tauri
  capabilities and current webview navigation.

- [x] **RED/GREEN - capture-result protocol:** add failing tests for versioned
  JSON events, opaque token validation, capture kind/destination validation and
  message limits. Implement shared encode/decode types without accepting paths.

- [x] **RED/GREEN - engine result export:** add failing tests for atomic PNG
  transfer, owned-directory confinement, cleanup and native/Tauri presentation
  choice. In Tauri mode emit still results for Area, Window, Desktop and
  Scrolling; preserve normal native presentation and video behavior.

- [x] **RED/GREEN - Tauri event reader:** add failing backend tests for bounded
  line reading, malformed events, missing/oversized/invalid images, child exit
  and transfer cleanup. Pipe capture-engine stdout and process events outside
  the UI thread.

- [x] **RED/GREEN - editor session registry:** replace the single editor slot
  with isolated session IDs and a 16-session cap. Add failing tests for open,
  command routing, preview routing, close and independent Undo/Redo state.

- [x] **RED/GREEN - editor hosts and tabs:** add failing frontend state and UI
  tests for New tab versus New window, active session switching, close, localized
  capture names, command session IDs and late events. Create restricted Tauri
  editor windows and keep the launcher available.

- [ ] **Integration and regression:** verify launcher Area, Window, Desktop and
  Scrolling route to the Tauri editor; verify at least one global hotkey and
  scrolling Escape/Stop; recheck native mode, video, EN/TH, DPI, High Contrast,
  duplicate tools, Text/Callout, Copy and PNG.

  Automated regression is complete. Hands-on capture routing remains pending
  because the Windows UI automation helper was unavailable on 2026-10-22.

- [x] **Packaged candidate:** run formatting, complete JS/Rust suites, release
  build and `git diff --check`; build a distinct unsigned installer, calculate
  SHA-256 and provide it for hands-on testing without pushing or publishing.
