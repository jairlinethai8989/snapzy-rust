# SnapZy 0.15.0-alpha.6

The SnapZy Windows launcher and editor use Tauri 2, HTML, CSS and vanilla JavaScript.
There is no React, Vue, Svelte or other JavaScript UI framework. Vite builds
the static frontend; Lucide supplies icons.

The Tauri launcher and editor are a user interface for the existing Rust
capture and image-editing engine. Alpha 6 supports capture, scrolling, video,
hotkeys, editable `.snapzy` projects and the editor tools described in the
bilingual user guides. OCR and the editor's add/combine image commands remain
visible but are not connected in this version.

## Run The Portable Build

Open `SnapZy.exe` in the portable folder. Keep its `engine` subfolder beside
it. Windows 10/11 x64 and Microsoft Edge WebView2 Runtime are required.

The launcher supports Area, Window/Desktop, Scrolling and Video, Open Image
or Project, English/Thai, capture delay 0/3/5/10 seconds, automatic/manual
scrolling, tab/window destination, recording source/audio and native hotkeys.
Delay is consumed by the next image capture. Video still uses its countdown.

Settings and generated recordings are kept under
`%LOCALAPPDATA%\SnapZyTauriPreview`. Closing the Tauri window requests normal
native shutdown and preserves the existing unsaved-work confirmation. It does
not forcibly kill the editor when a close prompt is cancelled.

## Develop

From `experiments/tauri`:

```powershell
npm ci
npm test
npm run dev
npm run desktop:dev
npm run desktop:build
```

`desktop:build` builds the native engine, the frontend and the Tauri shell, then
assembles `dist/SnapZy-0.15.0-alpha.6-win-x64` and an NSIS installer in the repository root.
The browser preview is `http://127.0.0.1:51819`. Browser-only capture commands
show a desktop-app notice; they do not simulate successful captures.

The bridge validates a bounded, typed local request and targets only the
engine child process. It does not expose arbitrary shell execution or paths.
Startup/capture responsiveness must be measured on the actual desktop build;
no performance improvement is claimed from the framework choice alone.

Tauri: MIT/Apache-2.0. Lucide icons: ISC. SnapZy: MIT. Existing native engine
dependencies are listed in `THIRD-PARTY-NOTICES.md`.
