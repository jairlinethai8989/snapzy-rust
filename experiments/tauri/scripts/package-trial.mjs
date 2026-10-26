import { cp, copyFile, mkdir, readdir, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const trial = fileURLToPath(new URL("../", import.meta.url));
const version = "0.15.0-alpha.6";
const destination = path.join(root, `dist/SnapZy-${version}-win-x64`);
await mkdir(destination, { recursive: true });
await copyFile(path.join(trial, "src-tauri/target/x86_64-pc-windows-msvc/release/snapzy-tauri.exe"), path.join(destination, "SnapZy.exe"));
await cp(path.join(trial, "src-tauri/resources/engine"), path.join(destination, "engine"), { recursive: true });
await copyFile(path.join(root, "LICENSE"), path.join(destination, "LICENSE"));
await copyFile(path.join(root, "THIRD-PARTY-NOTICES.md"), path.join(destination, "THIRD-PARTY-NOTICES.md"));
await cp(path.join(root, "third-party-licenses"), path.join(destination, "third-party-licenses"), { recursive: true });
await copyFile(path.join(trial, "README.md"), path.join(destination, "README.md"));
const metadata = spawnSync("cargo", ["metadata", "--locked", "--format-version", "1", "--manifest-path", path.join(trial, "src-tauri/Cargo.toml")], { encoding: "utf8", shell: false, maxBuffer: 32 * 1024 * 1024 });
if (metadata.status !== 0) throw new Error(`Cannot collect the Tauri dependency license manifest: ${metadata.error?.message || metadata.stderr}`);
const notes = ["# SnapZy Dependency Licenses", "", "Lucide 1.60.0: ISC. See third-party-licenses/lucide-1.60.0/LICENSE.", "", "| Rust package | License |", "| --- | --- |"];
const iconLicense = path.join(destination, "third-party-licenses/lucide-1.60.0");
await mkdir(iconLicense, { recursive: true });
await copyFile(path.join(trial, "node_modules/lucide/LICENSE"), path.join(iconLicense, "LICENSE"));
for (const dependency of JSON.parse(metadata.stdout).packages) {
  if (dependency.name === "snapzy-tauri") continue;
  notes.push(`| ${dependency.name} ${dependency.version} | ${dependency.license || "See license files"} |`);
  const directory = path.dirname(dependency.manifest_path);
  const licenses = (await readdir(directory, { withFileTypes: true })).filter((file) => file.isFile() && /^(licen[cs]e|copying|notice)([.\-_]|$)/i.test(file.name));
  const target = path.join(destination, "third-party-licenses/tauri", `${dependency.name}-${dependency.version}`);
  if (licenses.length) await mkdir(target, { recursive: true });
  for (const file of licenses) await copyFile(path.join(directory, file.name), path.join(target, file.name));
}
await writeFile(path.join(destination, "TAURI-THIRD-PARTY-NOTICES.md"), `${notes.join("\n")}\n`, "utf8");

const nsisDirectory = path.join(trial, "src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis");
const installers = (await readdir(nsisDirectory)).filter((name) =>
  name.startsWith("SnapZy Rust_") && name.endsWith("-setup.exe") && name.includes(version)
);
if (installers.length !== 1) throw new Error(`Expected one ${version} NSIS installer, found: ${installers.join(", ") || "none"}`);
const installer = path.join(root, `dist/SnapZy-Setup-${version}-win-x64.exe`);
await copyFile(path.join(nsisDirectory, installers[0]), installer);
console.log(JSON.stringify({ portable: destination, installer }));
