# SnapZy Font, Hotkey, and Taskbar Design

Date: 2026-10-26  
Status: Approved from the visual mockup; implementation in progress.

## Scope

This change covers three related usability issues in the Windows application:

1. New text defaults to Noto Sans Thai while existing saved font choices remain unchanged.
2. Capture hotkeys can use letters as the primary key in addition to the existing function and Print Screen keys.
3. The Tauri launcher remains visible in the Windows Taskbar so a running application can always be found and restored.

## Design

### Font default

`FontFamily::default()` becomes `NotoSansThai`. Existing serialized text objects keep their stored font, and users can still choose Segoe UI, Arial, Calibri, Tahoma, Kanit, Prompt, Noto Sans Thai, or Sarabun. The native editor and Tauri editor use the same default.

### Hotkey editor

Each action keeps its enabled checkbox, modifier selector, and primary-key selector. The modifier choices remain the current Windows-safe combinations: Ctrl + Alt, Ctrl + Shift, Alt + Shift, and Ctrl + Alt + Shift. The primary-key selector adds A–Z while retaining F8–F11 and Print Screen. A binding is valid only when both the modifier and primary key are supported. Duplicate enabled combinations are rejected before registration or saving, and a failed registration restores the previous working set.

### Taskbar presence

The Tauri main window explicitly uses `visible: true` and `skipTaskbar: false`. Editor windows retain normal decorations and remain taskbar-visible. Capture/engine helper windows remain non-taskbar windows. This keeps one discoverable SnapZy launcher entry while avoiding helper-window clutter.

## Error handling

- Invalid or duplicate hotkeys show the existing localized error and leave the previous bindings active.
- A missing or closed Tauri window is not treated as a successful taskbar registration.
- Existing tray behavior is preserved; the launcher can still be minimized to the tray when the user explicitly chooses it.

## Verification

- Core tests verify Noto Sans Thai is the default font and letter bindings are valid.
- Native UI tests verify the hotkey list contains letters and legacy keys.
- Tauri contract tests verify the main window is explicitly visible and taskbar-enabled.
- Full Rust, Tauri, frontend, UI smoke, and release installer checks run before delivery.
