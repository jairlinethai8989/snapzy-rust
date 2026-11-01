import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { setTimeout } from "node:timers/promises";

const script = fileURLToPath(new URL("../src-tauri/windows/migrate-legacy.ps1", import.meta.url));
const engine = fileURLToPath(new URL("../src-tauri/resources/engine/snapzy-engine.exe", import.meta.url));
// Limit real process enumeration to our own fixture so user sessions stay untouched.
const check = (pid = 0) => spawnSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command",
  `function Get-Process { param([string]$ErrorAction) Microsoft.PowerShell.Management\\Get-Process -ErrorAction $ErrorAction | Where-Object { $_.Id -eq ${pid} } }; & '${script.replaceAll("'", "''")}' -Mode CheckRunning; exit $LASTEXITCODE`,
], { encoding: "utf8" });
assert.equal(check().status, 0, "No fixture process should allow installation");
const child = spawn(engine, ["--editor-service"], { stdio: ["pipe", "ignore", "pipe"], windowsHide: true });
try {
  await new Promise((resolve, reject) => { child.once("spawn", resolve); child.once("error", reject); });
  await setTimeout(500);
  assert.equal(child.exitCode, null, "The real editor engine must remain alive for the lock check");
  const running = check(child.pid);
  assert.equal(running.status, 20, `Installer must block replacement when only snapzy-engine.exe is running: ${running.stderr}`);
} finally {
  child.stdin.end();
  if (child.exitCode === null) {
    await Promise.race([new Promise((resolve) => child.once("exit", resolve)), setTimeout(2000)]);
    if (child.exitCode === null) child.kill();
  }
}
assert.equal(check(child.pid).status, 0, "Installation may proceed once the engine exits");
console.log("Passed: an engine without a launcher blocks installation; exiting it releases the guard.");
