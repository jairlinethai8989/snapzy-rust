import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { chromium } from "playwright";

const png = [...await readFile(new URL("../../../assets/snapzy-rust.png", import.meta.url))];
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1180, height: 710 } });
  page.on("pageerror", (error) => console.error("Page error:", error.message));
  await page.addInitScript(({ png }) => {
    const tabs = [
      { session_id: "a".repeat(32), title: "First image" },
      { session_id: "b".repeat(32), title: "Second image" },
    ];
    const view = (width, height) => ({ width, height, revision: 0, tool: "Select", color: [255, 0, 0], parameter: 5, opacity: 100, pattern: "Solid", font: "noto_sans_thai", selected: false, can_undo: false, can_redo: false, can_delete: false, crop_ready: false, dirty: true });
    window.combineRequest = null;
    window.__TAURI__ = { core: { invoke: async (command, args = {}) => {
      switch (command) {
        case "editor_window_mode": return null;
        case "editor_tabs": return { tabs, active: tabs.at(-1).session_id };
        case "editor_activate": return args.sessionId === "c".repeat(32) ? view(528, 256) : view(256, 256);
        case "editor_preview": return png;
        case "editor_combine_candidates": return tabs.map((tab) => ({ ...tab, width: 256, height: 256 }));
        case "editor_combine":
          window.combineRequest = args;
          tabs.push({ session_id: "c".repeat(32), title: "Combined images" });
          return { session_id: "c".repeat(32), view: view(528, 256) };
        default: throw new Error(`Unexpected command: ${command}`);
      }
    } } };
  }, { png });
  await page.goto(`${process.env.SNAPZY_TEST_URL || "http://127.0.0.1:51819"}/editor.html`);
  await page.locator("#combine-button").waitFor({ state: "visible" });
  try { await page.locator("#combine-button:enabled").waitFor({ timeout: 4000 }); }
  catch (error) {
    console.error("Combine bootstrap:", await page.locator("#editor-notice").textContent(), await page.locator(".document-tab").count());
    throw error;
  }
  await page.locator("#combine-button").click();
  assert.equal(await page.locator("#combine-dialog").isVisible(), true);
  assert.equal(await page.locator("#combine-confirm").isDisabled(), true);
  await page.locator(".combine-source input").first().check();
  assert.match(await page.locator("#combine-size").textContent(), /256 × 528/);
  await page.locator("#combine-layout").selectOption("horizontal");
  assert.match(await page.locator("#combine-size").textContent(), /528 × 256/);
  await page.locator(".combine-source button[data-direction='-1']").nth(1).click();
  await page.locator("#combine-confirm").click();
  await page.locator(".document-tab").nth(2).waitFor();
  assert.deepEqual(await page.evaluate(() => window.combineRequest), {
    sessionIds: ["b".repeat(32), "a".repeat(32)], layout: "horizontal", gap: 16,
  });
  assert.equal(await page.locator(".document-tab").count(), 3);
  console.log("Passed: Combine button selects, orders and joins two tabs into a new editable tab.");
} finally {
  await browser.close();
}
