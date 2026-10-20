# Compact Native UI Design

**Candidate:** SnapZy Rust `0.14.0-alpha.1`  
**Status:** Approved visual direction and implementation candidate; native
acceptance remains partially open for host DPI/High Contrast and timing
comparison
**Scope:** Rust launcher and image editor only

## Objective

Make the Rust interface visually consistent with SnapZy .NET while using less
screen space. Preserve the fast native startup path and every accepted capture,
editing, project, OCR, scrolling and video behavior. The redesign must improve
icon clarity, discoverability and compactness without adding WebView or a
network dependency.

## Approved Direction

Use the approved **Adaptive Compact** layout:

- Keep common commands visible.
- Show object-specific properties only when they apply.
- Move secondary commands into a single `More` menu as width becomes limited.
- Keep `Copy` and `Save PNG` visible at every supported editor width.
- Use one consistent outlined icon family with localized tooltips.

## Launcher

The target launcher client area is approximately `360 x 140` logical pixels at
100% DPI. Exact outer dimensions may vary with the Windows title bar.

The launcher contains:

1. A compact header with the SnapZy icon/name, short version, real status,
   language toggle and `More` menu.
2. Four equal primary capture buttons: Area, Window, Scrolling and Video.
3. One compact options row for delay and the default open destination.

The Window control is a split button. Clicking its main area immediately starts
window capture. Clicking its chevron opens a two-item menu for Window and
Desktop capture. Desktop capture remains a first-class command and retains its
existing behavior; it is not inferred from a failed window selection.

The Video control remains a menu command. Clicking anywhere on it opens the
monitor/window source choices, matching the accepted behavior and avoiding an
accidental recording start.
The `More` menu holds hotkey settings, About and Hide to Tray. Recording and
capture status replace the idle status in place and must never claim success
before a real operation completes.

## Editor Layout

The editor keeps a minimum supported width of `780` logical pixels and uses
three fixed control bands above the canvas:

1. **Work bar:** brand, active document name and dimensions, status, language,
   Help, core file commands, Copy and Save PNG.
2. **Tool bar:** selection, crop, drawing and annotation tools, layer access,
   delete, Undo/Redo and More.
3. **Context bar:** only the properties relevant to the active tool or selected
   object, plus the view control at the right edge.

With tabs visible, the complete controls including the tab strip target about
`129` logical pixels. A single-work window targets about `104` logical pixels.
Control dimensions remain stable while selection and tool state change.

At wide widths the work bar shows Add Image, Combine, Open Project and Save
Project directly. At narrower widths those commands move into `More` in the
same logical order. The responsive transition must not recreate documents,
change focus, cancel gestures or alter Undo/Redo state.

The two shape families remain grouped split buttons: outline rectangle/ellipse
and filled rectangle/ellipse. Layer access is one compact button which opens
Bring to Front, Move Forward, Move Backward and Send to Back. A disabled layer
command remains visible in the menu but cannot be invoked.

## Icon System

Use the Lucide outlined icon family already established in the .NET product.
Do not substitute letters, unrelated Windows glyphs or ambiguous symbols for
available icons.

- Standard button: `34 x 34` logical pixels.
- Standard icon: `19 x 19` logical pixels at approximately `1.9 px` stroke.
- Normal state: dark neutral icon on white.
- Hover/focus: green-tinted surface with a clear border/focus indicator.
- Selected state: green surface and green icon with a persistent border.
- Disabled state: reduced contrast while retaining the recognizable shape.
- Destructive actions and video recording: restrained red accent only.
- Save PNG: icon plus text on the primary green command button.

Icons are embedded vector data and rendered through the existing native paint
path. Rendered icons are cached by icon, state and DPI. No icon parsing, disk
access or allocation-heavy rasterization occurs on launcher startup or normal
repaint. The implementation must preserve crisp output from 100% through 200%
DPI and provide a safe cached fallback if a DPI-specific render fails.

The Lucide license and attribution are added to the existing third-party notice
set if not already present.

## Tooltips And Localization

Every icon-only command has a Windows native tooltip. Tooltips:

- appear after approximately `400 ms` hover;
- use the active EN/TH application language;
- name the action directly and may include the shortcut when one exists;
- close immediately after click, pointer exit, language change or window close;
- remain within the current monitor work area;
- never activate a window, steal focus or interfere with drawing/dragging.

Changing language refreshes button accessibility names, menu text, tooltips,
status and help together. English remains the default language.

## Native Architecture

Keep the current Win32 implementation and split the redesign into focused
units:

- `native_icons`: embedded Lucide geometry, DPI-aware rendering and cache;
- `native_tooltips`: one native tooltip manager per launcher/editor window;
- `launcher_layout`: compact geometry, split commands and launcher menus;
- `editor_layout`: responsive command placement and context-band geometry;
- existing editor/capture modules: unchanged command behavior and state.

Layout consumes existing application state and routes the same command IDs used
today. Moving a command into a menu changes only presentation, not its action,
validation, busy state or keyboard shortcut.

## Failure And Edge Handling

- If a command cannot fit in its direct band, move it to `More`; never overlap,
  clip or silently remove it.
- If icon rendering fails, reuse the last valid cached icon and keep the command
  accessible through its localized tooltip/menu name.
- If a tooltip cannot be created, commands remain usable and Help still lists
  every action.
- Busy and disabled states come from the existing operation state; the redesign
  does not synthesize completion messages.
- Menus close before starting capture, opening modal UI or changing windows.
- No UI failure may mutate the active document or capture settings.

## Verification

Development follows test-first changes for layout and behavior. Automated tests
cover:

- launcher and editor geometry at 100%, 125%, 150% and 200% DPI;
- no overlap, clipping or missing commands at supported minimum widths;
- correct direct-versus-`More` command routing;
- Window/Desktop and Video split-button routing;
- icon mapping, state colors and cache reuse;
- complete EN/TH tooltip and menu coverage;
- context controls for every tool and selectable object family;
- unchanged keyboard shortcuts and command IDs;
- full existing Rust test suite and optimized build.

Native acceptance checks cover launcher size, tooltip readability, keyboard
focus, tray behavior, every capture mode, Editor/OCR, scrolling capture with
Esc/Stop, video, tabs, project open/save, Copy/PNG and multiple monitor DPI
scales. Startup and capture-ready timings are compared with the accepted
`0.13.0-alpha.1` baseline; the redesign is not accepted if it introduces a
clear repeatable regression.

## Exclusions

This increment does not add a layer list, installer, startup registration,
autosave, OCR accuracy changes, video pause/region recording or new editing
behavior. It does not modify the .NET repository, commit, push, publish or
replace the accepted `0.13.0-alpha.1` package.
