import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { rm, readFile, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const engine = path.join(root, "dist/SnapZy-0.15.0-alpha.8-win-x64/engine/snapzy-engine.exe");
const input = path.join(root, "tests/fixtures/ocr/english.png");
const output = path.join(tmpdir(), "snapzy-tauri-editor-smoke.png");
const project = path.join(tmpdir(), "snapzy-tauri-editor-smoke.snapzy");
await rm(output, { force: true });
await rm(project, { force: true });

const child = spawn(engine, ["--editor-service"], {
  cwd: path.dirname(engine),
  stdio: ["pipe", "pipe", "inherit"],
});
const lines = createInterface({ input: child.stdout });
const waiting = [];
lines.on("line", (line) => waiting.shift()?.resolve(JSON.parse(line)));
child.once("error", (error) => waiting.splice(0).forEach(({ reject }) => reject(error)));

function send(request) {
  return new Promise((resolve, reject) => {
    waiting.push({ resolve, reject });
    child.stdin.write(`${JSON.stringify(request)}\n`);
  });
}

try {
  const opened = await send({ type: "open", path: input });
  assert.equal(opened.ok, true);
  assert.ok(opened.view.width > 0 && opened.view.height > 0);

  const chosen = await send({ type: "apply", command: { type: "choose", tool: "Line" } });
  assert.equal(chosen.view.tool, "Line");
  await send({ type: "apply", command: { type: "press", point: { x: 10, y: 10 }, tolerance: 5 } });
  await send({ type: "apply", command: { type: "move", point: { x: 80, y: 40 } } });
  const drawn = await send({ type: "apply", command: { type: "release", point: { x: 80, y: 40 } } });
  assert.ok(drawn.view.revision > opened.view.revision);

  const text = await send({
    type: "apply",
    command: { type: "add_text", point: { x: 18, y: 48 }, text: "SnapZy text", callout: false },
  });
  assert.equal(text.ok, true);
  const callout = await send({
    type: "apply",
    command: { type: "add_text", point: { x: 92, y: 58 }, text: "SnapZy callout", callout: true },
  });
  assert.equal(callout.ok, true);
  assert.ok(callout.view.revision > text.view.revision);

  const duplicatedObject = await send({ type: "apply", command: { type: "duplicate_selection" } });
  assert.equal(duplicatedObject.ok, true);
  assert.equal(duplicatedObject.view.tool, "Select");
  assert.equal(duplicatedObject.view.selected, true);
  const duplicatedArea = await send({
    type: "apply",
    command: { type: "duplicate_area", start: { x: 8, y: 8 }, end: { x: 72, y: 56 } },
  });
  assert.equal(duplicatedArea.ok, true);
  assert.equal(duplicatedArea.view.selected, true);
  assert.ok(duplicatedArea.view.revision > duplicatedObject.view.revision);

  const preview = await send({ type: "preview", path: output });
  assert.equal(preview.ok, true);
  const bytes = await readFile(output);
  assert.deepEqual([...bytes.subarray(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10]);
  assert.ok((await stat(output)).size > 100);

  const saved = await send({ type: "save_project", path: project });
  assert.equal(saved.ok, true);
  assert.equal(saved.view.dirty, false);
  assert.ok((await stat(project)).size > 100);
  const reopened = await send({ type: "open_project", path: project });
  assert.equal(reopened.ok, true);
  assert.equal(reopened.view.dirty, false);
  assert.equal(reopened.view.width, duplicatedArea.view.width);
  assert.equal(reopened.view.height, duplicatedArea.view.height);

  const shutdown = await send({ type: "shutdown" });
  assert.equal(shutdown.ok, true);
  child.stdin.end();
  const code = await new Promise((resolve) => child.once("exit", resolve));
  assert.equal(code, 0);
  console.log("Passed: packaged Rust editor engine opens, draws, adds text/callout, duplicates objects/areas, renders PNG and shuts down.");
} finally {
  if (!child.killed) child.kill();
  await rm(output, { force: true });
  await rm(project, { force: true });
}
