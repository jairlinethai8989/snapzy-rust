# Native Tab Workspace: Stage 2B

Date: 2026-10-12 (Asia/Bangkok).
Status: written specification approved by the user on 2026-10-12; implementation started.
Target: separate local SnapZy Rust 0.9.0-alpha.16 candidate.
Rust only. No .NET edits, commit, push, installer or public release.
Preserve Alpha 14-r2, Alpha 15 packages and the existing dirty checkout.

## Context And Decisions

Stage 2A added independent editable image combination and schema-2 projects.
The user reported working combination, PNG saving, clipboard copying, OCR,
scrolling capture and Esc stopping in Alpha 15. Detailed native edge cases in
its test notes remain gaps; these reports do not certify every case or exact
export pixel parity. Carry those gaps into verification rather than erasing them.

The user selected tabs as the default, with a persistent New window alternative.
This stage changes document hosting and routing, not annotation semantics or the
overall editor design. Each open project remains an independent editable work.

Approaches considered:

1. Native host with per-document child editors (recommended): retain each
   editor session, controls, history, caches and guarded worker identity while
   changing only its hosting and routing. This limits changes to existing tools.
2. One editor/control set rebound to the selected document: less control storage,
   but broad changes to gesture state, modal ownership and asynchronous results.
3. Keep separate windows with a window-switcher strip: smaller change but does
   not provide the approved .NET-like tab workspace.

No dragging/reordering/detaching tabs, session restore, autosave, new layer panel,
new capture modes, video, .neosnap interchange or broad UI redesign in this stage.
Do not add a browser runtime. Existing editor content remains native Windows.

## Opening And Routing

- Add a compact two-option New tab / New window setting on the launcher, with
  EN/TH labels. Persist it independently of language and hotkeys. Old settings
  without this field default to New tab; invalid values are rejected by existing
  settings validation. Existing English language default remains unchanged.
- Changing the setting affects future opens only. Never move, merge, close or
  replace existing documents as a side effect of changing it.
- In New tab mode, capture, opening a standalone image/project and combining
  documents create and select a new independent tab. Never overwrite an existing
  tab, even when opening the same file twice.
- An editor-initiated open or combination targets that editor's host. A launcher
  or hotkey capture/open targets the last activated live editor host. If none
  exists, create a host. Track activation explicitly, not by unordered-map order.
- Capture the destination host identity and open mode when the operation starts.
  Later activation or setting changes must not reroute its pending result.
- In New window mode, each newly opened result creates a separate host containing
  one tab. Existing multi-tab hosts remain intact. Every host uses the same tab
  infrastructure, including the single-tab case.
- If the destination host closes before completion, do not resurrect it or add
  the result to an unrelated host. Discard the pending result and release its
  reservation. A request that began without a host may create its first host
  only if still valid and the app is not closing.
- Opening/combining failure publishes no partial tab and leaves all source
  documents, dirty markers, selections and history unchanged.
- Retain the initiating document token for editor-originated requests even in
  New window mode. Closing/reusing that document invalidates its pending result;
  a new destination host is not permission to bypass the existing origin guard.
- Keep the existing maximum of 16 open documents across all hosts, counting
  pending reservations. A host is not a document and does not increase the limit.

## Tab Surface And State

- Use a native tab strip above the existing editor controls and canvas. Keep
  the current editor's practical minimum dimensions; account for the added strip
  in layout rather than overlapping or shrinking controls unpredictably.
- Each tab shows its project filename, or a stable numbered Image label for a
  capture/unsaved composition, a dirty marker and a close icon. Show the complete
  name in a tooltip. Truncate long labels without covering the close target.
- Keep tab hit targets and close targets stable. Overflow scrolls horizontally;
  selecting an offscreen tab brings it into view. No font scaling with viewport.
- Active styling and EN/TH close tooltips distinguish the current tab clearly.
  Rename/save updates its label and the host title without selecting another tab.
- Each document retains its own source assets, session, selected object, tool
  settings, document crop/view, history, project path, saved revision, composition
  cache and pending operations. Switching never flattens or reopens the project.
- Show only the selected child editor and route editing commands to it. Retain
  inactive work in memory; do not duplicate its pixels solely to represent a tab.
- Ctrl+Tab / Ctrl+Shift+Tab cycle within the current host, wrapping at either end.
  Ctrl+W requests closing the active tab. Existing Ctrl+O, Ctrl+S, Save As,
  Undo/Redo and clipboard shortcuts apply to the correct active document.
- Do not intercept these shortcuts inside text entry, file dialogs, OCR result
  text or other focused editable dialogs. Keyboard focus returns to the selected
  editor when switching completes.
- Block switching/closing during an unfinished pointer gesture or modal text/
  Effects edit. Let the user finish or use existing Esc cancellation first;
  never silently commit, discard or transfer a draft to another document.
- Commit valid pending numeric control edits through existing focus-loss logic
  before changing active editor. Do not apply an old control's value to a new tab.

## Closing And Saving

- Tab close icon / Ctrl+W closes that document only. Dirty work uses the existing
  Save project / Discard / Cancel decision. Copying or PNG export still does not
  count as saving an editable project.
- Save failure or cancelled Save As keeps the document open. Block duplicate
  close requests and never discard a newer revision after a late save completion.
- With multiple tabs, the host close button offers Close current tab / Close all
  tabs in this window / Cancel. With one tab it requests closing that tab directly.
  This scope is the current host, not every window in the application.
- Collect all required close decisions and successful saves before removing any
  document in a Close all operation. Cancel or failure keeps the entire tab set
  open. Saves already completed remain saved; cancellation does not undo disk I/O.
- Fix the close set by stable document identity and revision. Prevent conflicting
  edits/new destinations during the transaction, recheck identities/revisions
  before removal, and reject stale completion. Do not treat tab indices as IDs.
- When closing the active tab, select its right neighbor, else its left neighbor.
  Closing an inactive tab does not change the active document. Closing the last
  tab closes the empty host but leaves the launcher/tray application running.
- App Exit retains its all-document scope across all hosts, with the same
  collect-before-close safety. Cancel must not close some hosts or their tabs.
- Release child controls, caches and associated OCR windows only after approved
  document closure; release a host only after its last document is removed.

## Async Work, Dialogs And Capture

- Keep document identity/generation separate from host identity and display order.
  Preserve the existing per-document composition/save/import/OCR operation guards.
  Never resolve a worker result using whichever tab is active at completion time.
- Passive save/import/composition/OCR callbacks may update the originating
  inactive document's cache, saved status or bound result window without
  activating its editor. Closed/reused identities reject late results and release
  reservations. These callbacks must not steal focus. Deliberately requested new
  captures/opens/combinations select their new tab in the captured destination;
  they are not passive updates to an existing tab.
- OCR snapshots the originating document and revision. Its result remains bound
  to that document even after switching tabs; use a host-owned native result
  window for modality while retaining the document token for data routing.
  Closing the document closes its OCR windows and invalidates pending UI results.
- Switching is allowed during worker work when no modal or unfinished gesture
  blocks it. Existing global concurrency limits remain; tabs do not create new
  unbounded workers or bypass busy guards.
- Native dialogs use the host as their top-level owner, while commands and
  results retain the child document identity. Refocus/reveal the originating tab
  when a user-requested close/save decision needs attention.
- Capture hides visible top-level editor hosts, launcher and app-owned transient
  result windows that could contaminate the capture. Preserve hidden/minimized
  state and active tab; restoration must not expose all inactive child editors.
  Restore only surviving recorded host identities on cancel/failure/success.
- Long capture keeps the existing non-activating panel, detector and Esc/Stop
  behavior. Tab shortcuts must not consume Esc intended for scrolling capture.

## Combination And Persistence

- List all open documents across hosts in Combine, using stable document identity
  and unique numbered labels to distinguish duplicate filenames. The initiating
  document starts checked. Source order changes affect only the combination.
- Snapshot current committed source revisions. Preserve Stage 2A editable groups,
  local transforms, clips, effects and shared assets. Do not import source tab
  identities, project paths, selections or Undo/Redo into the new combination.
- Continue using schema 2. Save/Open remains one editable document per .snapzy
  file; tab layout, host placement and routing preferences are not project data.
- Old schema-1/schema-2 projects retain appearance and editability. Opening a
  project adds a tab/window according to routing without changing any open work.
- Maintain the original-data warning for source pixels beneath crop/blur.

## Boundaries And Performance

Separate a testable workspace state model (document/host identities, ordered tabs,
active tab, routing and close sets) from the native host/strip adapter. Retain the
existing editor/renderer/session modules. Targeted routing/ownership changes in
ui, project_ui and combine_ui are necessary; unrelated refactors are not.

Reuse child editor windows and their existing caches. A tab switch is visibility,
layout and control-state synchronization, not file decoding, PNG encoding or
project reconstruction. Do not run full composition repeatedly for inactive tabs;
retain valid results and schedule necessary rendering through existing workers.

Keep the aggregate 32 MiB distinct source/history allowance, 32 MiB canvas
buffers, 256 MiB composition scratch bound and document/object/depth limits.
Count shared assets once across hosts and pending workers. Native tab controls
and per-editor metadata have overhead; do not claim zero overhead or relax limits.
No added preload/model scan on startup or capture-to-first-preview paths.

## Verification And Delivery

Use test-first increments for routing, activation, neighboring-tab choice,
settings migration/validation, close collection/cancellation, stale worker and
host generations, capacity/reservations and shared asset accounting. Cover
switching after save starts, closing before work returns, duplicate filenames,
multiple hosts, hidden/minimized capture restoration and empty-host cleanup.

Native checks after fresh input-free consent:

- New tab default and New window override, persistence across restart, language
  changes, image/project/capture/combine routing, and switching between hosts.
- Independent tools, selection, numeric edits, history, project save markers and
  dirty-state Undo; duplicate project opens must remain independent.
- Tab overflow, long Thai/English filenames, keyboard focus/shortcuts, compact
  window sizes, maximize/resize and supported DPI changes.
- Single and bulk close: Save/Discard/Cancel, failed/cancelled save, repeated close,
  close-last-tab, app Exit, and no partially closed set after cancellation.
- OCR/save/import/composition completion while another tab is active; no wrong-tab
  content, result focus stealing or use-after-close behavior.
- Editable combine, owner selection/resize/crop, schema-1/schema-2 reopening,
  clipboard and PNG rendering parity; carry remaining Alpha 15 edge cases forward.
- Area/window/desktop capture, long capture detection, final-frame retention and
  both Esc and on-screen Stop, including Lark tester acceptance where available.
- Compare startup, capture-to-first-paint and tab-switch responsiveness with the
  preserved Alpha 15 package on generated fixtures; record actual observations,
  payload sizes and memory limits rather than making unsupported speed claims.

Run the full Rust test suite, formatting and warning-free release build after
implementation. Package Alpha 16 separately with accurate EN/TH notes and hashes;
preserve older packages. Document remaining verification gaps explicitly.
No commit or push without a separate user request.

## Design Review Checklist

- [x] Existing Rust window/session/worker/settings/close paths inspected.
- [x] .NET tab/open-mode/close behavior inspected read-only for workflow parity.
- [x] Alternatives and default routing presented; user selected tabs by default.
- [x] Stage scope presented and approved by the user.
- [x] Written design self-reviewed for scope, ownership, cancellation and limits.
- [x] User review of this written specification.
- [x] Implementation plan after written specification approval.
