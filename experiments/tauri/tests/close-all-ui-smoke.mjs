import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { chromium } from "playwright";

const png = [...await readFile(new URL("../../../assets/snapzy-rust.png", import.meta.url))];
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  await page.addInitScript(({ png }) => {
    const tabs = ["a", "b"].map((key) => ({ session_id: key.repeat(32), title: key }));
    window.closedHost = false;
    window.closedWindow = false;
    let active = tabs[1].session_id;
    window.__TAURI__ = {
      window: { getCurrentWindow: () => ({ close: async () => { window.closedWindow = true; } }) },
      event: { listen: async (name, callback) => { (window.editorListeners ??= {})[name] = callback; return () => {}; } },
      core: { invoke: async (command, args = {}) => {
        if (command === "editor_window_mode") return;
        if (command === "editor_tabs") return { tabs, active };
        if (command === "editor_activate") {
          active = args.sessionId;
          return { width: 256, height: 256, revision: 0, tool: "Select", color: [255, 0, 0], parameter: 5, opacity: 100, pattern: "Solid", font: "noto_sans_thai", selected: false, can_undo: false, can_redo: false, can_delete: false, crop_ready: false, dirty: true };
        }
        if (command === "editor_preview") {
          if (active === tabs[0].session_id) throw new Error("Invalid image viewport or nonuniform transform.");
          return png;
        }
        if (command === "editor_close_window") { window.closedHost = true; return; }
        if (command === "editor_close") {
          tabs.splice(tabs.findIndex((tab) => tab.session_id === args.sessionId), 1);
          active = tabs.at(-1)?.session_id ?? null;
          return { tabs, active };
        }
        throw new Error(`Unexpected command: ${command}`);
      } },
    };
  }, { png });
  await page.goto(`${process.env.SNAPZY_TEST_URL || "http://127.0.0.1:51819"}/editor.html`);
  await page.locator(".document-tab").nth(1).waitFor();
  await page.locator("#more-button:enabled").click();
  await page.getByRole("menuitem", { name: "Close all tabs", exact: true }).click({ timeout: 3000 });
  await page.locator("#close-dialog[open]").waitFor();
  await page.locator("#close-cancel").click();
  assert.equal(await page.locator(".document-tab").count(), 2);
  assert.equal(await page.evaluate(() => window.closedHost), false);
  await page.locator("#more-button").click();
  await page.getByRole("menuitem", { name: "Close all tabs", exact: true }).click();
  await page.locator("#close-dialog[open]").waitFor();
  await page.locator("#close-discard").click();
  await page.waitForFunction(() => window.closedWindow);
  assert.equal(await page.evaluate(() => window.closedHost), true);

  await page.reload();
  await page.locator(".document-tab").nth(1).waitFor();
  await page.evaluate(() => window.editorListeners["editor-window-close-requested"]());
  await page.locator("#close-scope-dialog[open]").waitFor();
  assert.equal(await page.locator("#scope-current").isVisible(), true);
  assert.equal(await page.locator("#scope-all").isVisible(), true);
  await page.locator("#scope-cancel").click();
  assert.equal(await page.locator(".document-tab").count(), 2);
  await page.evaluate(() => window.editorListeners["editor-window-close-requested"]());
  await page.locator("#scope-current").click();
  await page.locator("#close-dialog[open]").waitFor();
  await page.locator("#close-discard").click();
  await page.waitForFunction(() => document.querySelectorAll(".document-tab").length === 1);
  assert.equal(await page.evaluate(() => window.closedHost), false);
  await page.evaluate(() => window.editorListeners["editor-window-close-requested"]());
  await page.locator("#scope-all").click();
  await page.locator("#close-dialog[open]").waitFor();
  await page.locator("#close-discard").click();
  await page.waitForFunction(() => window.closedWindow);
  console.log("Passed: Close All remains usable with a failed preview; Cancel preserves all tabs.");
} finally {
  await browser.close();
}
