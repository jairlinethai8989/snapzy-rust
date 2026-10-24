import { spawnSync } from "node:child_process";
import { copyFile, cp, mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const build = spawnSync("cargo", ["build", "--release", "--locked"], {
  cwd: root, stdio: "inherit", shell: false,
});
if (build.status !== 0) process.exit(build.status ?? 1);
const source = path.join(root, "target/x86_64-pc-windows-msvc/release");
const destination = fileURLToPath(new URL("../src-tauri/resources/engine/", import.meta.url));
await mkdir(destination, { recursive: true });
for (const [input, output] of [
  ["snapzy-rust.exe", "snapzy-engine.exe"],
  ["tesseract50.dll", "tesseract50.dll"],
  ["leptonica-1.82.0.dll", "leptonica-1.82.0.dll"],
]) await copyFile(path.join(source, input), path.join(destination, output));
await cp(fileURLToPath(new URL("../ui/fonts/", import.meta.url)), path.join(destination, "fonts"), { recursive: true });
