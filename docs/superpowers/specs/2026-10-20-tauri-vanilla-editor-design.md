# Tauri Vanilla JavaScript Image Editor

## Goal

Replace the visible native Win32 image editor with a Tauri 2 editor built from
HTML, CSS and vanilla JavaScript. Keep Rust as the authority for document state,
rendering, project compatibility, OCR and exports so the new interface produces
the same image as the current Rust editor.

The launcher trial remains the entry point. After a capture or file open, the
same Tauri window changes to the editor workspace instead of presenting the
native editor window. English remains the default language and Thai remains an
in-place language toggle.

## Chosen Architecture

Use a Tauri frontend with a Rust editor service in the Tauri process. The
frontend owns presentation and input only. It never becomes the source of truth
for annotations, history, layer order or project save state.

The existing Rust package will expose reusable editor, renderer, project, OCR,
composition and image modules through a library target. The native executable
continues to provide capture selection, scrolling capture and video. Captured
images are transferred to the Tauri process through a bounded, instance-owned
temporary result. The Tauri backend validates the result, creates a Rust editor
session and deletes the transfer file after adoption.

Rust editor sessions are held in a registry keyed by unguessable session IDs.
Every Tauri command identifies one live session and carries an expected revision
where stale operations would be unsafe. A closed session rejects late OCR,
composition, save or render results.

### Why This Architecture

- It preserves one renderer for editor preview, clipboard and PNG output.
- It preserves `.snapzy` and supported `.neosnap` behavior without translating
  the project model into JavaScript.
- It avoids duplicating hit testing, Undo/Redo, layers, curves, blur and effects.
- It lets the visible editor use normal web layout and accessible controls.
- It keeps capture and long-scroll behavior isolated from the UI migration.

## Component Boundaries

### Vanilla JavaScript Frontend

The frontend contains small ES modules with no UI framework:

- `editor-shell.js`: active document, tabs and high-level routing.
- `editor-toolbar.js`: tool buttons, grouped menus and context controls.
- `editor-canvas.js`: canvas presentation, zoom/pan and pointer normalization.
- `editor-state.js`: immutable normalization of backend view models.
- `editor-i18n.js`: English and Thai visible strings and tooltips.
- `editor-dialogs.js`: compact save, close, effects, OCR and combine dialogs.

The frontend may retain view-only state such as open menus, zoom, scroll offset
and keyboard focus. It must not independently mutate editable document state.

### Tauri Rust Backend

The backend exposes typed commands for:

- creating, opening, closing and switching sessions;
- selecting tools and changing context properties;
- pointer begin/move/end/cancel gestures;
- undo, redo, delete and layer ordering;
- opening/saving projects and exporting/copying PNG;
- OCR, effects, combine and canvas operations;
- requesting a current view model and rendered preview.

Commands validate numeric ranges, session identity, revision and payload size.
The frontend receives bounded error codes plus localized-safe details. File
paths are selected by native dialogs in Rust and are never accepted as arbitrary
webview command strings.

### Native Capture Engine

Capture selection, target detection, long scrolling and video remain in the
existing native engine. In Tauri mode an image capture writes an owned temporary
result and returns its token to the Tauri backend. Video keeps its current
preview workflow until a later dedicated UI migration.

## Editor Layout

The editor uses four stable horizontal regions and a full remaining canvas:

1. A 32 px document tab strip with unsaved markers and close buttons.
2. A 44 px command bar with document status on the left and language, Help,
   OCR, open/save, Copy and Save PNG on the right.
3. A 42 px tool bar with visually separated tool groups.
4. A 38 px context bar showing only controls relevant to the active tool.
5. The canvas fills the remaining client area.

Tool groups, from left to right:

- Select, owning image and Crop.
- Pen, Highlighter, Line and Arrow.
- Outline shape and filled shape grouped menus.
- Blur, numbered step, Text and Speech Bubble.
- Effects and Layers.
- Delete, Undo and Redo.

Icon-only commands use 36 px stable cells with 22–24 px thin-stroke Lucide
icons. Grouped controls reserve space for a menu chevron. Every icon has an
English and Thai tooltip, accessible name, hover state, pressed state, disabled
state and visible keyboard focus.

At narrow supported widths, secondary document commands move into More in this
order: Add image, Paste image, Resize workspace, Effects, Combine images, Open
project and Save project. Copy and Save PNG remain directly visible. The toolbar
does not horizontally scroll or clip labels. Context controls may wrap into a
second context row only below the supported desktop minimum width.

## Canvas And Interaction

Rust renders the current composition into a bounded BGRA preview buffer. The
Tauri backend converts or transfers it as an image payload without base64. The
frontend paints it to a canvas at the current zoom and draws UI-only overlays
for selection bounds, resize handles, curve points, crop drafts and callout
tails from geometry supplied by Rust.

Pointer coordinates are converted from CSS pixels through device scale, zoom,
pan and crop origin before being sent to Rust. Rust remains responsible for hit
testing and gesture results. Pointer move rendering is coalesced to one request
per animation frame. A monotonically increasing preview sequence prevents an
older render from replacing a newer edit.

Zoom behavior:

- Fit to workspace when a document first opens.
- Ctrl plus wheel zooms around the pointer.
- Space plus drag pans without changing the active tool.
- A view menu offers Fit, 100%, Zoom In and Zoom Out.
- Export resolution never changes with editor zoom.

Keyboard behavior preserves current shortcuts: Undo/Redo, Delete, Escape,
Enter for crop, copy/paste, open and project save. Escape closes the topmost
menu/dialog first, cancels an active gesture next, and otherwise leaves the
document unchanged.

## State And Data Flow

For an edit gesture:

1. JavaScript sends normalized pointer input for the active session.
2. Rust validates the session and updates a draft gesture.
3. Rust returns a compact view model and a preview sequence number.
4. JavaScript updates control state and requests the matching preview.
5. On pointer release Rust commits one history entry and increments revision.

For capture:

1. Tauri hides its own windows and asks the native engine to capture.
2. The engine returns a bounded temporary result token.
3. Tauri validates and adopts the image into a Rust editor session.
4. The editor view opens as a new tab or window according to the saved setting.
5. The transfer is removed after successful adoption or failure cleanup.

For save/close, Rust supplies the dirty state and owns the final decision. A
Save/Discard/Cancel result applies only to the original session revision. Closing
multiple tabs stops before changing the tab set when the user selects Cancel.

## Feature Parity

The migration must retain the current Rust editor capabilities:

- tab and separate-window workspaces;
- Select, move, resize, parent image selection and per-image crop;
- Pen, Highlighter, Line, Arrow and bend controls;
- outline and filled rectangle/ellipse shapes and stroke patterns;
- Text, Speech Bubble, background, border and tail controls;
- numbered steps, Blur, opacity, shadow and text outline;
- Layers, Undo/Redo, Delete and canvas expansion;
- Add/Paste image and editable image combination;
- Open/Save/Save As `.snapzy` and supported `.neosnap` import;
- OCR, Copy image and Save PNG;
- unsaved work prompts and isolated state for every tab.

The first usable build is not accepted with placeholder buttons. A command may
move into More or a compact dialog, but every listed capability must remain
reachable and functional before replacing the native editor in distributed
packages.

## Performance And Limits

- Opening the editor must not load OCR models or video components.
- Toolbar actions update immediately without waiting for a full PNG encode.
- Pointer move requests are coalesced and stale preview work is canceled.
- Preview buffers and source image limits keep the existing Rust safety bounds.
- Large and long images use viewport-sized preview work where the current
  renderer permits it; export continues to use full document resolution.
- Closing a tab releases preview buffers, source references and pending work.

The migration will record startup-to-editor, pointer-to-preview and export timing
using the existing trace system. It must not regress the accepted native capture
response time.

## Error Handling

- A backend error keeps the current document and displays one compact notice.
- Failed saves preserve the existing destination and keep the document dirty.
- Invalid or stale session requests are rejected without affecting another tab.
- Capture transfer failures restore the launcher and clean only owned temporary
  files.
- Renderer failure keeps the last valid preview and offers Retry.
- JavaScript exceptions are caught at command boundaries and do not close the
  Rust editor session.

## Accessibility And Localization

The editor supports keyboard navigation, logical tab order, visible focus,
tooltips, accessible labels and Windows High Contrast colors. Text must fit at
100%, 125%, 150% and 200% display scaling. English and Thai switch in place
without rebuilding the session or changing current selections. Visible labels
use no negative letter spacing and do not rely on color alone for state.

## Delivery Stages

1. Extract the reusable Rust editor service and add guarded capture-result
   transfer without changing the distributed native editor.
2. Add the Tauri editor shell, tabs, command/tool/context bars and read-only
   canvas with open image and capture results.
3. Connect gestures, core drawing tools, context controls, history, selection,
   layers and export.
4. Connect text/callout, effects, OCR, project files, image combination and all
   close/save workflows.
5. Run parity, DPI, High Contrast, EN/TH, performance and long-image checks; then
   build a separate local trial package for user acceptance.

Each stage keeps the previous native editor available. The Tauri editor becomes
the default only after the full parity gate passes and the user accepts the
trial installer.

## Verification

- Unit tests for command validation, state revisions and coordinate transforms.
- Rust renderer parity tests comparing preview, Copy and PNG pixels.
- Browser tests at 780, 1024 and 1440 px widths and scale factors 1, 1.25, 1.5
  and 2 for overflow, focus, tooltips and EN/TH.
- Real Windows tests for pointer capture, keyboard shortcuts, file dialogs,
  clipboard, High Contrast and multiple monitors.
- Workflow tests for every listed editor feature, multiple tabs, Cancel during
  close, OCR completion after tab changes and stale render rejection.
- Regression tests for area/window/desktop/scroll capture, Escape/Stop, video,
  project compatibility and current native editor operation during migration.

## Out Of Scope

- A JavaScript reimplementation of the Rust renderer or document model.
- Rich text with multiple styles inside one text object.
- Video editing or migration of the video preview in this editor phase.
- Cloud storage, accounts or online processing.
