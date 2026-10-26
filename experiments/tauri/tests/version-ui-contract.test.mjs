import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const expectedVersion = "0.15.0-alpha.8";
const versionPattern = new RegExp(expectedVersion.replaceAll(".", "\\."));
const root = new URL("..", import.meta.url);
const read = (path) => readFile(new URL(path, root), "utf8");

test("SnapZy manifests, engine and visible UI report one product version", async () => {
  const packageJson = JSON.parse(await read("package.json"));
  const packageLock = JSON.parse(await read("package-lock.json"));
  const tauriConfig = JSON.parse(await read("src-tauri/tauri.conf.json"));
  const cargoToml = await read("src-tauri/Cargo.toml");
  const cargoLock = await read("src-tauri/Cargo.lock");
  const engineCargoToml = await read("../../Cargo.toml");
  const launcher = await read("ui/index.html");
  const editor = await read("ui/editor.html");
  const launcherText = await read("ui/i18n.js");
  const packaging = await read("scripts/package-trial.mjs");

  assert.equal(packageJson.version, expectedVersion);
  assert.equal(packageLock.version, expectedVersion);
  assert.equal(packageLock.packages[""].version, expectedVersion);
  assert.equal(tauriConfig.version, expectedVersion);
  assert.equal(tauriConfig.app.windows[0].visible, true);
  assert.equal(tauriConfig.app.windows[0].skipTaskbar, false);
  assert.equal(tauriConfig.productName, "SnapZy Rust");
  assert.equal(tauriConfig.identifier, "com.jairlinethai.snapzy.rust");
  assert.doesNotMatch(JSON.stringify(tauriConfig), /trial/i);
  assert.match(cargoToml, /^name = "snapzy-tauri"$/m);
  assert.match(cargoToml, new RegExp(`^version = "${expectedVersion}"$`, "m"));
  assert.match(cargoLock, new RegExp(`name = "snapzy-tauri"\\r?\\nversion = "${expectedVersion}"`));
  assert.match(engineCargoToml, new RegExp(`^version = "${expectedVersion}"$`, "m"));
  assert.match(launcher, versionPattern);
  assert.match(editor, versionPattern);
  assert.match(launcherText, versionPattern);
  assert.doesNotMatch(launcherText, /UI trial|รุ่นทดลอง UI/i);
  assert.match(packaging, /SnapZy-\$\{version\}-win-x64/);
  assert.match(packaging, /SnapZy-Setup-\$\{version\}-win-x64\.exe/);
  assert.match(packaging, /startsWith\("SnapZy Rust_"\)/);
});

test("native image open and PNG save use Windows file dialogs instead of WebView downloads", async () => {
  const editor = await read("ui/editor.js");
  const nativeMain = await read("src-tauri/src/main.rs");
  assert.match(editor, /invoke\("editor_open_image"/);
  assert.match(editor, /invoke\("editor_save_png"/);
  assert.match(nativeMain, /fn editor_open_image\(/);
  assert.match(nativeMain, /fn editor_save_png\(/);
  assert.match(nativeMain, /editor_open_image,.*editor_save_png/s);
});

test("launcher Open image opens a real image instead of creating a blank editor", async () => {
  const launcherScript = await read("ui/main.js");
  const nativeMain = await read("src-tauri/src/main.rs");
  assert.match(launcherScript, /invoke\("editor_open_image_window"/);
  assert.doesNotMatch(launcherScript, /action === "open"\)\s*await[^;]+editor_new_window/);
  assert.match(nativeMain, /fn editor_open_image_window\(/);
  assert.match(nativeMain, /editor_open_image_window,/);
});

test("launcher image open returns before constructing the new WebView", async () => {
  const nativeMain = await read("src-tauri/src/main.rs");

  assert.match(
    nativeMain,
    /fn editor_open_image_window\([\s\S]*?Arc::clone\(engine\.inner\(\)\)[\s\S]*?thread::spawn\(move \|\|/,
  );
  assert.match(nativeMain, /fn open_image_path_in_new_window\(/);
});

test("Windows bundle is a current-user upgrade with guarded legacy migration", async () => {
  const tauriConfig = JSON.parse(await read("src-tauri/tauri.conf.json"));
  const hook = await read("src-tauri/windows/installer-hooks.nsh");
  const migration = await read("src-tauri/resources/migration/migrate-legacy.ps1");

  assert.deepEqual(tauriConfig.bundle.targets, ["nsis"]);
  assert.equal(tauriConfig.bundle.active, true);
  assert.equal(tauriConfig.bundle.windows.nsis.installMode, "currentUser");
  assert.match(tauriConfig.bundle.windows.nsis.installerHooks, /installer-hooks\.nsh$/);
  assert.match(hook, /NSIS_HOOK_POSTINSTALL/);
  assert.match(hook, /migrate-legacy\.ps1/);
  assert.match(migration, /SnapZy Rust\.lnk/);
  assert.match(migration, /Programs[\\/]SnapZyRust[\\/]SnapZy-Rust\.exe/);
  assert.match(migration, /TargetPath/);
  assert.match(migration, /SnapZy Tauri Trial/);
  assert.doesNotMatch(migration, /["']SnapZy\.lnk["']/);
  assert.match(migration, /UninstallString/);
  assert.match(migration, /-not \(Test-Path -LiteralPath \$uninstaller\)/);
  assert.doesNotMatch(migration, /Remove-Item[^\r\n]*SnapZyRust[^\r\n]*-Recurse/i);
});

test("Duplicate controls use clear object-copy and drag-area icons", async () => {
  const markup = await read("ui/editor.html");
  const script = await read("ui/editor.js");
  const text = await read("ui/editor-i18n.js");

  assert.match(markup, /data-tool="DuplicateArea"[^>]*>[\s\S]*?data-lucide="square-dashed-mouse-pointer"/);
  assert.match(markup, /id="duplicate-button"[^>]*>[\s\S]*?data-lucide="copy"/);
  assert.doesNotMatch(markup, /id="duplicate-button"[^>]*>[\s\S]*?data-lucide="copy-plus"/);
  assert.match(script, /DuplicateArea:\s*"square-dashed-mouse-pointer"/);
  assert.match(text, /Duplicate area[^\n]*drag/i);
  assert.match(text, /สำเนาพื้นที่[^\n]*ลาก/);
  assert.match(text, /Ctrl\+D/);
});

test("editor windows explicitly retain the native Windows frame", async () => {
  const rust = await read("src-tauri/src/main.rs");
  const createWindow = rust.match(/fn create_editor_window[\s\S]*?\n\}/)?.[0] ?? "";

  assert.match(createWindow, /\.decorations\(true\)/);
  assert.match(createWindow, /\.resizable\(true\)/);
  assert.match(createWindow, /\.maximizable\(true\)/);
  assert.match(createWindow, /\.minimizable\(true\)/);
  assert.match(createWindow, /\.closable\(true\)/);
  assert.doesNotMatch(createWindow, /\.fullscreen\(true\)/);
});

test("editor default height keeps the Windows frame visible on a 768 px display", async () => {
  const rust = await read("src-tauri/src/main.rs");
  const createWindow = rust.match(/fn create_editor_window[\s\S]*?\n\}/)?.[0] ?? "";

  assert.match(createWindow, /\.inner_size\(1180\.,\s*680\.\)/);
  assert.doesNotMatch(createWindow, /\.inner_size\(1180\.,\s*760\.\)/);
});

test("editor startup does not resize the window so tall that its Windows frame leaves a 768 px display", async () => {
  const rust = await read("src-tauri/src/main.rs");
  const startupMode = rust.match(/fn editor_window_mode[\s\S]*?\n\}/)?.[0] ?? "";

  assert.match(startupMode, /\.set_size\(Size::Logical\(LogicalSize::new\(1180\.,\s*680\.\)\)\)/);
  assert.doesNotMatch(startupMode, /LogicalSize::new\(1180\.,\s*760\.\)/);
});

test("Effects remains available from More when compact layout hides its toolbar group", async () => {
  const script = await read("ui/editor.js");
  assert.match(script, /popupItem\(t\("effects"\),\s*"sparkles",\s*openEffects\)/);
  assert.doesNotMatch(script, /effects-button[^\n]*notAvailable/);
});

test("editor uses palette popovers, visual line samples and an approved font picker", async () => {
  const markup = await read("ui/editor.html");
  const script = await read("ui/editor.js");

  assert.match(markup, /id="tool-color"[^>]*class="[^\"]*color-trigger/);
  assert.doesNotMatch(markup, /id="tool-color"[^>]*type="color"/);
  assert.doesNotMatch(markup, /<select[^>]*id="line-pattern"/);
  assert.match(markup, /id="line-pattern"[^>]*class="[^\"]*pattern-trigger/);
  assert.match(markup, /id="text-font"/);
  assert.match(markup, /id="text-background-enabled"/);
  assert.match(markup, /id="text-background-color"/);
  assert.match(markup, /id="custom-color-input"[^>]*type="color"[^>]*hidden/);
  assert.match(script, /EDITOR_PALETTE/);
  assert.match(script, /EDITOR_FONTS/);
});
