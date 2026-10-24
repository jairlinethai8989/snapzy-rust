# Tauri And Vanilla JavaScript UI Trial

## Goal

Try a real Tauri 2 launcher for SnapZy using HTML, CSS and vanilla JavaScript.
The first trial replaces the launcher only. Capture, annotation, projects, OCR,
scrolling and video continue through the existing Rust native engine/editor.

## Interface

- Compact light Windows-style launcher with aligned thin Lucide icons.
- Area, Window/Desktop, Scrolling, Video, Open Image, EN/TH and More.
- Delay 0/3/5/10 seconds applies to the next image capture only.
- Automatic/manual scrolling, tab/window destination, video monitor/window.
- Tooltips, keyboard focus, Escape dismissal, localized labels and errors.
- Browser preview shows an explicit desktop-only message for native actions.

## Architecture

The experiment lives in `experiments/tauri`. Vite is development/build tooling,
not a JavaScript UI framework. Tauri owns a local engine child process started
with `--tauri-engine`. It can send only typed, bounded commands via WM_COPYDATA
to the launcher window belonging to that child PID. Arbitrary executable names,
shell commands and file paths are not accepted from JavaScript.

The engine has its own instance guard and `%LOCALAPPDATA%/SnapZyTauriPreview`
settings. Normal native launches retain the existing behavior. The webview
hides before capture and restores after selection/recording has completed;
it must not appear in captured pixels. Long operations run outside the Tauri
event thread. Closing the trial requests the engine's existing safe shutdown,
including unsaved-work prompts; it never forcibly terminates an open editor.

## Verification And Delivery

Validate the bridge command boundary, frontend preferences and one-shot delay,
native regression suite, frontend build, Tauri compile and real desktop window.
Check EN/TH, menus, keyboard behavior and actual command handoff. Deliver a local
portable trial folder and browser preview URL. This task does not publish a
GitHub release. A full web-based editor and replacement installer are later work.
