import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { createRequire } from "node:module";

const require = createRequire(process.env.SNAPZY_PLAYWRIGHT_PACKAGE || import.meta.url);
const { chromium } = require("playwright");
const browser = await chromium.launch({ headless: true });
const errors = [];
await mkdir("test-results", { recursive: true });

async function checkFit(page) {
  const overflow = await page.evaluate(() => {
    const result = [];
    for (const element of document.querySelectorAll("main button, main button span")) {
      const rect = element.getBoundingClientRect();
      if (element.scrollWidth > element.clientWidth + 1 || rect.right > innerWidth + 1 || rect.left < 0) result.push(element.textContent.trim());
    }
    return result;
  });
  assert.deepEqual(overflow, [], "Visible controls must not overflow");
  assert.equal(await page.locator("#logo").evaluate((image) => image.complete && image.naturalWidth > 0), true);
}

try {
  for (const [width, height, scale] of [[424, 254, 1], [390, 254, 1.25], [360, 254, 2], [1280, 720, 1]]) {
    const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: scale });
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto("http://127.0.0.1:51819/");
    await page.waitForSelector(".capture-button svg");
    await checkFit(page);
    await page.screenshot({ path: `test-results/launcher-en-${width}.png` });
    await page.locator("#language").click();
    assert.equal(await page.locator("html").getAttribute("lang"), "th");
    await checkFit(page);
    await page.screenshot({ path: `test-results/launcher-th-${width}.png` });
    await page.locator('[data-menu="delay"]').click();
    await page.getByRole("menuitemradio", { name: "5 วินาที", exact: true }).click();
    assert.equal(await page.locator("#delay-value").textContent(), "5 วินาที");
    await page.locator('[data-action="area"]').click();
    await page.locator("#notice").waitFor({ state: "visible" });
    assert.equal(await page.locator("#delay-value").textContent(), "5 วินาที", "Browser preview must not claim capture success");
    await page.locator("#dismiss").click();
    await page.locator('[data-menu="scroll"]').click();
    await page.keyboard.press("Escape");
    assert.equal(await page.locator('[data-menu="scroll"]').getAttribute("aria-expanded"), "false");
    await page.locator("#video").click();
    await page.locator('#video-dialog[open]').waitFor();
    const dialogBounds = await page.locator("#video-dialog").boundingBox();
    assert.ok(dialogBounds.y >= 0 && dialogBounds.y + dialogBounds.height <= height + 1);
    await page.keyboard.press("Escape");
    await page.locator('[data-action="area"]').hover();
    await page.waitForTimeout(650);
    assert.equal(await page.locator("#tooltip").isVisible(), true);
    assert.equal(await page.locator("#tooltip").textContent(), "จับภาพเฉพาะพื้นที่ที่เลือก");
    await page.close();
  }

  const page = await browser.newPage({ viewport: { width: 424, height: 254 } });
  await page.addInitScript(() => {
    window.invocations = [];
    window.__TAURI__ = { core: { invoke: async (command, args) => {
      window.invocations.push({ command, ...args });
      if (window.rejectCapture) throw new Error("Test engine failure");
    } } };
  });
  await page.goto("http://127.0.0.1:51819/");
  await page.locator('[data-menu="delay"]').click();
  await page.getByRole("menuitemradio", { name: "3 seconds", exact: true }).click();
  await page.locator('[data-action="area"]').click();
  await page.waitForFunction(() => document.querySelector("#delay-value").textContent === "No delay");
  const call = await page.evaluate(() => window.invocations.at(-1));
  assert.equal(call.command, "desktop_action");
  assert.equal(call.request.action, "area");
  assert.equal(call.request.preferences.delay_seconds, 3);
  await page.locator('[data-menu="delay"]').click();
  await page.getByRole("menuitemradio", { name: "10 seconds", exact: true }).click();
  await page.evaluate(() => { window.rejectCapture = true; });
  await page.locator('[data-action="area"]').click();
  await page.locator("#notice").waitFor({ state: "visible" });
  assert.equal(await page.locator("#delay-value").textContent(), "10 seconds");
  await page.emulateMedia({ forcedColors: "active" });
  await page.locator("#dismiss").click();
  await page.screenshot({ path: "test-results/launcher-high-contrast.png" });
  await page.close();
  assert.deepEqual(errors, []);
  console.log("Passed: EN/TH, control bounds, assets, DPI preview, tooltips, menus/Esc, video dialog, browser guard, command handoff and one-shot delay.");
} finally {
  await browser.close();
}
