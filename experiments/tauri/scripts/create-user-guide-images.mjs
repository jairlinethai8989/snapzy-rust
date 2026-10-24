import { chromium } from "playwright";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(process.cwd(), "../..", "docs", "images");
await mkdir(root, { recursive: true });

function sampleReportSvg() {
  const rows = [
    ["SR-2048", "Prepare weekly service summary", "Complete", "N. Somchai"],
    ["SR-2049", "Review access request", "In progress", "P. Anong"],
    ["SR-2050", "Update training materials", "Waiting", "K. Narin"],
    ["SR-2051", "Check monthly inventory", "Complete", "M. Siriporn"],
    ["SR-2052", "Resolve account issue", "In progress", "T. Kanya"],
    ["SR-2053", "Prepare handover notes", "Complete", "J. Arun"],
    ["SR-2054", "Confirm deployment window", "Waiting", "S. Mali"],
    ["SR-2055", "Refresh team dashboard", "Complete", "R. Niran"],
    ["SR-2056", "Check support queue", "In progress", "A. Pim"],
    ["SR-2057", "Archive closed requests", "Complete", "D. Preecha"],
  ];
  const rowsSvg = rows.map((row, index) => {
    const y = 226 + index * 54;
    const fill = index % 2 ? "#ffffff" : "#f8fbff";
    const statusColor = row[2] === "Complete" ? "#13835b" : row[2] === "Waiting" ? "#a96411" : "#2464b5";
    return `<rect x="48" y="${y}" width="1504" height="54" fill="${fill}"/><line x1="48" y1="${y + 54}" x2="1552" y2="${y + 54}" stroke="#e6edf4"/><text x="76" y="${y + 33}" class="cell muted">${row[0]}</text><text x="260" y="${y + 33}" class="cell">${row[1]}</text><rect x="1040" y="${y + 13}" width="116" height="28" rx="14" fill="${statusColor}" fill-opacity=".11"/><text x="1057" y="${y + 32}" class="status" fill="${statusColor}">${row[2]}</text><text x="1284" y="${y + 33}" class="cell">${row[3]}</text>`;
  }).join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="820" viewBox="0 0 1600 820"><rect width="1600" height="820" fill="#f4f8fc"/><rect x="28" y="24" width="1544" height="772" rx="18" fill="#fff" stroke="#dbe5ee"/><text x="68" y="88" class="title">Support operations</text><text x="68" y="122" class="sub">Weekly service overview · Sample workspace</text><rect x="48" y="152" width="1504" height="58" rx="8" fill="#eef4fa"/><text x="76" y="188" class="head">REQUEST</text><text x="260" y="188" class="head">TASK</text><text x="1040" y="188" class="head">STATUS</text><text x="1284" y="188" class="head">OWNER</text>${rowsSvg}<text x="68" y="770" class="sub">10 sample items · Updated just now</text><style>.title{font:600 32px 'Segoe UI',sans-serif;fill:#162b42}.sub{font:16px 'Segoe UI',sans-serif;fill:#718196}.head{font:600 13px 'Segoe UI',sans-serif;letter-spacing:1px;fill:#687a8e}.cell{font:16px 'Segoe UI',sans-serif;fill:#26384b}.cell.muted{fill:#74849a}.status{font:600 13px 'Segoe UI',sans-serif}</style></svg>`;
}

const browser = await chromium.launch({ headless: true });
try {
  const launcher = await browser.newPage({ viewport: { width: 1000, height: 520 }, deviceScaleFactor: 2 });
  await launcher.goto("http://127.0.0.1:51819/", { waitUntil: "networkidle" });
  await launcher.addStyleTag({ content: ".launcher{max-width:480px!important}.browser body{min-height:100vh}" });
  await launcher.locator("#language").click();
  const launcherTargets = [
    [1, '[data-action="area"]'], [2, '[data-action="window"]'], [3, '[data-action="scrolling"]'], [4, "#video"],
    [5, '[data-menu="delay"]'], [6, '[data-menu="scroll"]'], [7, '[data-menu="destination"]'], [8, ".header-actions"],
  ];
  await addBadges(launcher, launcherTargets);
  await launcher.locator(".launcher").screenshot({ path: resolve(root, "snapzy-launcher-overview.png"), animations: "disabled" });

  const editor = await browser.newPage({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1 });
  await editor.goto("http://127.0.0.1:51819/editor.html", { waitUntil: "networkidle" });
  const svg = Buffer.from(sampleReportSvg());
  const imageData = `data:image/svg+xml;base64,${svg.toString("base64")}`;
  const pngBuffer = await editor.evaluate(async (source) => {
    const image = new Image();
    image.src = source;
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = image.naturalWidth;
    canvas.height = image.naturalHeight;
    canvas.getContext("2d").drawImage(image, 0, 0);
    return new Promise((resolve) => canvas.toBlob(async (blob) => resolve([...new Uint8Array(await blob.arrayBuffer())]), "image/png"));
  }, imageData);
  await editor.locator("#open-image").setInputFiles({ name: "support-operations-sample.png", mimeType: "image/png", buffer: Buffer.from(pngBuffer) });
  await editor.locator("#editor-notice").evaluate((element) => { element.hidden = true; });
  await editor.locator("#document-tabs").evaluate((container) => {
    const tab = document.createElement("button");
    tab.className = "document-tab active";
    tab.type = "button";
    tab.setAttribute("role", "tab");
    tab.setAttribute("aria-selected", "true");
    tab.innerHTML = '<span class="tab-name">support-operations-sample.png</span><span class="tab-close" aria-hidden="true">×</span>';
    container.append(tab);
  });
  await editor.locator("#document-size").waitFor({ state: "visible" });
  await editor.waitForTimeout(250);
  const editorTargets = [
    [1, ".tab-strip"], [2, ".document-actions"], [3, '[data-group="selection"]'], [4, '[data-group="drawing"]'],
    [5, '[data-group="shapes"]'], [6, '[data-group="content"]'], [7, '[data-group="object"]'],
    [8, '[data-group="history"]'], [9, ".context-bar"], [10, ".zoom-controls"], [11, "#canvas-stage"],
  ];
  await addBadges(editor, editorTargets);
  await editor.locator(".editor-app").screenshot({ path: resolve(root, "snapzy-editor-overview.png"), animations: "disabled" });
} finally {
  await browser.close();
}

async function addBadges(page, targets) {
  const positions = await page.evaluate((selectors) => selectors.map(([number, selector]) => {
    const element = document.querySelector(selector);
    if (!element) throw new Error(`Missing screenshot target: ${selector}`);
    const rect = element.getBoundingClientRect();
    return { number, selector, x: rect.left, y: rect.top, width: rect.width, height: rect.height };
  }), targets);
  await page.evaluate((items) => {
    const layer = document.createElement("div");
    Object.assign(layer.style, { position: "fixed", inset: "0", zIndex: "99999", pointerEvents: "none" });
    for (const item of items) {
      const badge = document.createElement("span");
      badge.textContent = item.number;
      Object.assign(badge.style, {
        position: "fixed",
        left: `${Math.max(5, item.x + (item.selector === ".tab-strip" ? 208 : item.selector === ".context-bar" ? 610 : item.selector === ".zoom-controls" ? -62 : item.selector === ".header-actions" ? -20 : item.selector === "#canvas-stage" ? 8 : -8))}px`,
        top: `${Math.max(2, item.y + (item.selector === ".tab-strip" ? 3 : item.selector === "#canvas-stage" ? 8 : -8))}px`,
        width: "22px", height: "22px", display: "grid", placeItems: "center", borderRadius: "50%",
        color: "white", background: "#086de5", border: "2px solid white", boxShadow: "0 1px 5px #10253d55",
        font: "700 12px/1 'Segoe UI',sans-serif",
      });
      layer.append(badge);
    }
    document.body.append(layer);
  }, positions);
}

console.log(`Created annotated guide images in ${root}`);
