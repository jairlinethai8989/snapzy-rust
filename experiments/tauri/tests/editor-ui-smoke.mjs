import assert from "node:assert/strict";
import { mkdir, readFile } from "node:fs/promises";
import { createRequire } from "node:module";

const require = createRequire(process.env.SNAPZY_PLAYWRIGHT_PACKAGE || import.meta.url);
const { chromium } = require("playwright");
const version = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8")).version;
const browser = await chromium.launch({ headless: true });
const errors = [];
const baseUrl = process.env.SNAPZY_TEST_URL || "http://127.0.0.1:51819";
await mkdir("test-results", { recursive: true });

async function assertEditorFit(page) {
  const layout = await page.evaluate(() => ({
    documentWidth: document.documentElement.scrollWidth,
    viewportWidth: innerWidth,
    overflow: [...document.querySelectorAll("button, .context-control")]
      .filter((element) => element.scrollWidth > element.clientWidth + 1)
      .map((element) => element.getAttribute("aria-label") || element.textContent.trim()),
  }));
  assert.equal(layout.documentWidth, layout.viewportWidth, "Editor must not scroll horizontally");
  assert.deepEqual(layout.overflow, [], "Controls must not clip labels or icons");
}

try {
  for (const [width, height, scale] of [[780, 560, 1], [1024, 720, 1.25], [1280, 800, 1.5], [1440, 900, 2]]) {
    const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: scale });
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto(`${baseUrl}/editor.html`);
    await page.waitForSelector(".tool-button svg");
    assert.match(await page.title(), new RegExp(version.replaceAll(".", "\\.")), "Editor title must expose the installed UI version");
    await assertEditorFit(page);
    assert.equal(await page.locator(".tool-button").count(), 13, "Duplicate Area is visible while four shape choices share two compact buttons");
    await page.locator('[data-tool="DuplicateArea"]').hover();
    await page.waitForTimeout(650);
    assert.equal(await page.locator("#editor-tooltip").textContent(), "Duplicate area");
    await page.locator('[data-tool="Line"]').hover();
    await page.waitForTimeout(650);
    assert.equal(await page.locator("#editor-tooltip").isVisible(), true);
    assert.equal(await page.locator("#editor-tooltip").textContent(), "Line");
    await page.locator("#editor-language").click();
    assert.equal(await page.locator("html").getAttribute("lang"), "th");
    assert.equal(await page.locator('[data-tool="Line"]').getAttribute("aria-label"), "เส้นตรง");
    await assertEditorFit(page);
    await page.locator("#open-image").setInputFiles("../../tests/fixtures/ocr/english.png");
    await page.waitForFunction(() => document.querySelector("#document-size").textContent.includes("x"));
    await page.waitForFunction(() => {
      const stage = document.querySelector("#canvas-stage");
      const [width, height] = document.querySelector("#document-size").textContent.match(/\d+/g).map(Number);
      const expected = Math.min(1, (stage.clientWidth - 48) / width, (stage.clientHeight - 48) / height);
      return Math.abs(Number.parseFloat(document.querySelector("#zoom-value").textContent) / 100 - expected) < 0.005;
    }, null, { timeout: 2000 });
    assert.equal(await page.locator("#copy-button").isEnabled(), true);
    assert.equal(await page.locator("#save-png").isEnabled(), true);
    assert.equal(await page.locator("#zoom-out").isVisible(), true);
    assert.equal(await page.locator("#zoom-in").isVisible(), true);
    await page.locator('[data-tool="Line"]').click();
    await page.locator("#tool-color").click();
    assert.equal(await page.locator("#editor-popup .palette-swatch").count(), 20);
    await page.locator("#editor-popup .palette-swatch").nth(8).click();
    await page.locator("#line-pattern").click();
    assert.equal(await page.locator("#editor-popup .pattern-item").count(), 3);
    await page.locator("#editor-popup .pattern-item").nth(1).click();
    assert.equal(await page.locator("#line-pattern .line-sample").getAttribute("data-pattern"), "Dashed");
    const canvasReady = await page.locator("#editor-canvas").evaluate((canvas) => canvas.width > 0 && canvas.height > 0);
    assert.equal(canvasReady, true);
    const beforeZoom = await page.locator("#zoom-value").textContent();
    await page.locator("#zoom-in").click();
    assert.notEqual(await page.locator("#zoom-value").textContent(), beforeZoom);
    await page.locator("#zoom-value").click();
    assert.equal(await page.locator("#editor-popup").isVisible(), true);
    await page.getByRole("menuitem", { name: "100%", exact: true }).click();
    assert.equal(await page.locator("#zoom-value").textContent(), "100%");
    await page.locator("#more-button").click();
    assert.equal(await page.getByRole("menuitem", { name: "พื้นที่ทำงาน", exact: true }).isVisible(), true);
    await page.getByRole("menuitem", { name: "พื้นที่ทำงาน", exact: true }).click();
    assert.equal(await page.locator("#canvas-dialog").isVisible(), true);
    const [canvasWidth, canvasHeight] = await page.locator("#document-size").textContent().then((text) => text.match(/\d+/g).map(Number));
    await page.locator("#canvas-width").fill(String(canvasWidth + 32));
    await page.locator("#canvas-height").fill(String(canvasHeight + 24));
    await page.locator("#canvas-apply").click();
    assert.match(await page.locator("#document-size").textContent(), new RegExp(`${canvasWidth + 32} × ${canvasHeight + 24}`));
    page.once("dialog", (dialog) => dialog.accept());
    await page.locator("#more-button").click();
    await page.getByRole("menuitem", { name: "พื้นที่ทำงาน", exact: true }).click();
    await page.locator("#canvas-width").fill(String(canvasWidth));
    await page.locator("#canvas-height").fill(String(canvasHeight));
    await page.locator("#canvas-apply").click();
    assert.match(await page.locator("#document-size").textContent(), new RegExp(`${canvasWidth} × ${canvasHeight}`));
    if (await page.locator("#effects-button").isVisible()) {
      await page.locator("#effects-button").click();
    } else {
      await page.locator("#more-button").click();
      await page.getByRole("menuitem", { name: "เอฟเฟกต์", exact: true }).click();
    }
    assert.equal(await page.locator("#effects-dialog").isVisible(), true);
    assert.equal(await page.locator("#outline-enabled").isDisabled(), true);
    await page.locator("#shadow-enabled").check();
    await page.locator("#shadow-color").click();
    assert.equal(await page.locator("#effects-dialog #editor-popup").isVisible(), true);
    await page.locator("#effects-dialog #editor-popup .palette-swatch").nth(2).click();
    await page.locator("#shadow-blur").fill("8");
    await page.locator("#effects-apply").click();
    assert.equal(await page.locator("#effects-dialog").isVisible(), false);
    await page.locator("#editor-canvas").click({ button: "right", position: { x: 40, y: 40 } });
    assert.equal(await page.locator("#editor-popup").isVisible(), true);
    assert.deepEqual(await page.locator("#editor-popup [role='menuitem']").allTextContents(), ["สำเนาพื้นที่"]);
    await page.locator("#editor-popup [data-context-action='duplicate_area']").click();
    assert.match(await page.locator('[data-tool="DuplicateArea"]').getAttribute("class"), /selected/);
    await page.locator("#editor-canvas").focus();
    await page.keyboard.press("Shift+F10");
    assert.equal(await page.locator("#editor-popup").isVisible(), true);
    await page.keyboard.press("Escape");
    assert.equal(await page.locator("#editor-popup").isHidden(), true);
    if (width === 780) {
      await page.locator('[data-tool="Text"]').click();
      await assertEditorFit(page);
      await page.locator("#text-font").selectOption("kanit");
      await page.locator("#text-background-enabled").check();
      await page.locator("#editor-canvas").click({ position: { x: 320, y: 220 } });
      assert.equal(await page.locator("#text-dialog").isVisible(), true);
      await page.locator("#text-value").fill("ข้อความทดสอบ");
      await page.locator("#text-apply").click();
      assert.equal(await page.locator("#text-dialog").isVisible(), false);
      assert.equal(await page.locator("#undo-button").isEnabled(), true);
      await page.locator("#more-button").click();
      await page.getByRole("menuitem", { name: "เอฟเฟกต์", exact: true }).click();
      assert.equal(await page.locator("#outline-enabled").isEnabled(), true);
      await page.locator("#outline-enabled").check();
      await page.locator("#effects-apply").click();

      await page.locator('[data-tool="Callout"]').click();
      await page.locator("#editor-canvas").click({ position: { x: 380, y: 260 } });
      assert.equal(await page.locator("#text-dialog").getAttribute("data-kind"), "Callout");
      await page.locator("#text-value").fill("กรอบข้อความ");
      await page.locator("#text-apply").click();
      assert.equal(await page.locator("#duplicate-button").isEnabled(), true);
      await page.locator("#editor-canvas").click({ button: "right", position: { x: 380, y: 260 } });
      assert.equal(await page.locator("#editor-popup").isVisible(), true);
      assert.deepEqual(await page.locator("#editor-popup [role='menuitem']").allTextContents(), [
        "สำเนาวัตถุ", "ลำดับชั้น", "ลบ",
      ]);
      await page.keyboard.press("ArrowDown");
      assert.equal(await page.locator("#editor-popup [role='menuitem']:focus").textContent(), "ลำดับชั้น");
      await page.keyboard.press("End");
      assert.equal(await page.locator("#editor-popup [role='menuitem']:focus").textContent(), "ลบ");
      await page.keyboard.press("Home");
      assert.equal(await page.locator("#editor-popup [role='menuitem']:focus").textContent(), "สำเนาวัตถุ");
      await page.keyboard.press("Escape");
      await page.locator("#duplicate-button").click();
      assert.equal(await page.locator('[data-tool="Select"]').getAttribute("class"), "tool-button selected");

      await page.locator('[data-tool="DuplicateArea"]').click();
      const canvas = page.locator("#editor-canvas");
      await canvas.dragTo(canvas, {
        sourcePosition: { x: 300, y: 180 },
        targetPosition: { x: 420, y: 280 },
      });
      await page.waitForFunction(() => document.querySelector('[data-tool="Select"]').classList.contains("selected"));

      const downloadPromise = page.waitForEvent("download");
      await page.locator("#save-png").click();
      const download = await downloadPromise;
      assert.equal(download.suggestedFilename(), "english.png");
    }
    await page.screenshot({ path: `test-results/editor-th-${width}.png` });
    await page.close();
  }

  const page = await browser.newPage({ viewport: { width: 1024, height: 720 } });
  await page.goto(`${baseUrl}/editor.html`);
  await page.emulateMedia({ forcedColors: "active" });
  await page.screenshot({ path: "test-results/editor-high-contrast.png" });
  await page.close();
  assert.deepEqual(errors, []);
  console.log("Passed: editor layout, text/callout, Duplicate Area/object, grouped shapes, EN/TH, tooltips, image open, PNG, DPI and High Contrast.");
} finally {
  await browser.close();
}
