import assert from "node:assert/strict";
import { chromium } from "playwright";

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1180, height: 720 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(`${process.env.SNAPZY_TEST_URL || "http://127.0.0.1:51819"}/editor.html`);
  const png = await page.evaluate(() => {
    const canvas = document.createElement("canvas");
    canvas.width = 1366; canvas.height = 1456;
    const ctx = canvas.getContext("2d");
    ctx.fillStyle = "#123456"; ctx.fillRect(0, 0, 1366, 1456);
    return canvas.toDataURL().split(",")[1];
  });
  await page.locator("#open-image").setInputFiles({ name: "tall.png", mimeType: "image/png", buffer: Buffer.from(png, "base64") });
  await page.locator("#zoom-in:enabled").waitFor();
  await page.evaluate(() => {
    const ctx = document.querySelector("#editor-canvas").getContext("2d");
    const translate = ctx.translate.bind(ctx);
    ctx.translate = (x, y) => { window.renderedPan = { x, y }; translate(x, y); };
  });
  await page.locator("#zoom-in").click();
  const before = await page.evaluate(() => window.renderedPan);
  await page.locator("#pan-view").click({ timeout: 3000 });
  const canvas = await page.locator("#editor-canvas").boundingBox();
  const x = canvas.x + canvas.width / 2, y = canvas.y + canvas.height / 2;
  async function drag(dx, dy, button = "left") {
    await page.mouse.move(x, y); await page.mouse.down({ button });
    await page.mouse.move(x + dx, y + dy, { steps: 5 }); await page.mouse.up({ button });
  }
  await drag(70, 80);
  let pan = await page.evaluate(() => window.renderedPan);
  assert.equal(Math.round(pan.x - before.x), 70);
  assert.equal(Math.round(pan.y - before.y), 80);
  assert.equal(await page.locator("#undo-button").isDisabled(), true, "Panning must not edit the document");
  await page.mouse.move(x, y); await page.mouse.wheel(0, 120);
  await page.waitForFunction((oldY) => window.renderedPan.y < oldY, pan.y);
  pan = await page.evaluate(() => window.renderedPan);
  await page.keyboard.down("Shift"); await page.mouse.wheel(0, 80); await page.keyboard.up("Shift");
  await page.waitForFunction((oldX) => window.renderedPan.x < oldX, pan.x);
  assert.equal(Math.round((await page.evaluate(() => window.renderedPan)).x - pan.x), -80);
  await page.locator("#pan-view").click();
  pan = await page.evaluate(() => window.renderedPan);
  await page.keyboard.down("Space"); await drag(0, 50); await page.keyboard.up("Space");
  assert.equal(Math.round((await page.evaluate(() => window.renderedPan)).y - pan.y), 50);
  pan = await page.evaluate(() => window.renderedPan);
  await drag(0, -40, "middle");
  assert.equal(Math.round((await page.evaluate(() => window.renderedPan)).y - pan.y), -40);
  await page.mouse.wheel(0, 100000);
  await page.waitForTimeout(100);
  pan = await page.evaluate(() => window.renderedPan);
  const zoom = Number((await page.locator("#zoom-value").textContent()).replace("%", "")) / 100;
  assert.ok(pan.y + 1456 * zoom >= 48, "Extreme scrolling must leave the image reachable");
  await page.locator("#fit-view").click();
  pan = await page.evaluate(() => window.renderedPan);
  assert.ok(pan.x >= 0 && pan.y >= 0, "Fit restores the full image inside the viewport");
  const fitZoom = await page.locator("#zoom-value").textContent();
  await page.mouse.move(x, y); await page.keyboard.down("Control");
  await page.mouse.wheel(0, -120); await page.keyboard.up("Control");
  await page.waitForFunction((oldZoom) => document.querySelector("#zoom-value").textContent !== oldZoom, fitZoom);
  await page.locator("#fit-view").click();
  assert.deepEqual(errors, []);
  console.log("Passed: zoom, hand drag, wheel, Space, middle drag, reachable bounds and Fit.");
} finally { await browser.close(); }
