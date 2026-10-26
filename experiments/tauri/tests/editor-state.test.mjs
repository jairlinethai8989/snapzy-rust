import test from "node:test";
import assert from "node:assert/strict";
import {
  EDITOR_FONTS,
  EDITOR_PALETTE,
  EDITOR_TOOL_GROUPS,
  backendCommand,
  clampContextMenuPosition,
  contextMenuActions,
  createCommandQueue,
  isContextMenuKey,
  moveMenuFocus,
  normalizeEditor,
  normalizeSelectionOverlay,
  pngDownloadName,
  selectionHandleAt,
  selectionGuide,
  selectionOverlayToScreen,
  screenToDocument,
  shouldHandleCanvasShortcuts,
  shouldDuplicateShortcut,
  translateVisiblePoint,
  steppedZoom,
  zoomAround,
} from "../ui/editor-state.js";

test("editor exposes a stable 20-color palette and the approved font families", () => {
  assert.equal(EDITOR_PALETTE.length, 20);
  assert.equal(new Set(EDITOR_PALETTE).size, 20);
  assert.ok(EDITOR_PALETTE.every((color) => /^#[0-9a-f]{6}$/.test(color)));
  assert.deepEqual(EDITOR_FONTS.map((font) => font.id), [
    "segoe_ui", "arial", "calibri", "tahoma",
    "kanit", "prompt", "noto_sans_thai", "sarabun",
  ]);
});

test("selection overlays are bounded validated and keep screen-size handles", () => {
  const overlay = normalizeSelectionOverlay({
    kind: "image",
    bounds: { start: { x: 10, y: 20 }, end: { x: 110, y: 70 } },
    handles: [
      { role: "top_left", point: { x: 10, y: 20 } },
      { role: "bottom_right", point: { x: 110, y: 70 } },
    ],
    movable: true,
  });
  assert.equal(overlay.kind, "image");
  const screen = selectionOverlayToScreen(overlay, { zoom: 2, pan: { x: 5, y: 7 } });
  assert.deepEqual(screen.bounds, {
    start: { x: 25, y: 47 },
    end: { x: 225, y: 147 },
  });
  assert.deepEqual(screen.handles[0], {
    role: "top_left",
    point: { x: 25, y: 47 },
    size: 10,
  });

  assert.equal(normalizeSelectionOverlay({ ...overlay, handles: Array(13).fill(overlay.handles[0]) }), null);
  assert.equal(normalizeSelectionOverlay({ ...overlay, bounds: { start: { x: NaN, y: 0 }, end: { x: 1, y: 1 } } }), null);
  assert.equal(normalizeSelectionOverlay({ ...overlay, kind: "unknown" }), null);

  assert.equal(selectionHandleAt(overlay, { x: 26, y: 48 }, { zoom: 2, pan: { x: 5, y: 7 } })?.role, "top_left");
  assert.equal(selectionHandleAt(overlay, { x: 50, y: 50 }, { zoom: 2, pan: { x: 5, y: 7 } }), null);
});

test("selection guides always use the fixed dashed transform style", () => {
  const line = selectionGuide({
    kind: "line",
    bounds: { start: { x: 10, y: 20 }, end: { x: 70, y: 40 } },
    handles: [
      { role: "start", point: { x: 10, y: 20 }, size: 10 },
      { role: "end", point: { x: 70, y: 40 }, size: 10 },
      { role: "curve", point: { x: 42, y: 18 }, size: 10 },
    ],
  });
  assert.deepEqual(line.dash, [5, 4]);
  assert.equal(line.kind, "quadratic");
  assert.equal("pattern" in line, false);

  const rectangle = selectionGuide({
    kind: "shape",
    bounds: { start: { x: 1, y: 2 }, end: { x: 30, y: 40 } },
    handles: [],
  });
  assert.deepEqual(rectangle, {
    kind: "rectangle",
    start: { x: 1, y: 2 },
    end: { x: 30, y: 40 },
    dash: [5, 4],
  });
});

test("context menus expose object commands or the drag-area tool", () => {
  assert.deepEqual(contextMenuActions(true), [
    "duplicate_selection", "layers", "delete",
  ]);
  assert.deepEqual(contextMenuActions(false), ["duplicate_area"]);
});

test("context menus stay inside the visible editor viewport", () => {
  assert.deepEqual(
    clampContextMenuPosition({ x: 995, y: 695 }, { width: 190, height: 160 }, { width: 1000, height: 700 }),
    { x: 802, y: 532 },
  );
  assert.deepEqual(
    clampContextMenuPosition({ x: -20, y: -10 }, { width: 190, height: 160 }, { width: 1000, height: 700 }),
    { x: 8, y: 8 },
  );
});

test("context menu keyboard navigation wraps and recognizes Windows menu keys", () => {
  assert.equal(moveMenuFocus(0, "ArrowDown", 3), 1);
  assert.equal(moveMenuFocus(2, "ArrowDown", 3), 0);
  assert.equal(moveMenuFocus(0, "ArrowUp", 3), 2);
  assert.equal(moveMenuFocus(1, "Home", 3), 0);
  assert.equal(moveMenuFocus(1, "End", 3), 2);
  assert.equal(isContextMenuKey({ key: "ContextMenu", shiftKey: false }), true);
  assert.equal(isContextMenuKey({ key: "F10", shiftKey: true }), true);
  assert.equal(isContextMenuKey({ key: "F10", shiftKey: false }), false);
});

test("pointer commands stay ordered while the editor service is busy", async () => {
  let finishPress;
  const pressFinished = new Promise((resolve) => { finishPress = resolve; });
  const calls = [];
  const enqueue = createCommandQueue(async (command) => {
    calls.push(`start:${command.type}`);
    if (command.type === "press") await pressFinished;
    calls.push(`end:${command.type}`);
  });

  const pressing = enqueue({ type: "press" });
  const releasing = enqueue({ type: "release" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.deepEqual(calls, ["start:press"]);

  finishPress();
  await Promise.all([pressing, releasing]);
  assert.deepEqual(calls, ["start:press", "end:press", "start:release", "end:release"]);
});

test("editor defaults keep the established red 5 px drawing style and Noto Sans Thai", () => {
  assert.deepEqual(normalizeEditor({}), {
    language: "en",
    activeTool: "Select",
    color: "#ff0000",
    parameter: 5,
    opacity: 100,
    pattern: "Solid",
    font: "noto_sans_thai",
    textBackground: { enabled: false, color: "#ffffff" },
    zoom: 1,
    pan: { x: 0, y: 0 },
    view: null,
  });
});

test("toolbar groups expose every editor tool once with shapes grouped", () => {
  const tools = EDITOR_TOOL_GROUPS.flatMap((group) => group.tools);
  assert.deepEqual(tools, [
    "Select", "Crop", "DuplicateArea",
    "Pen", "Highlighter", "Line", "Arrow",
    "Rectangle", "Ellipse", "FilledRectangle", "FilledEllipse",
    "Blur", "Step", "Text", "Callout",
  ]);
  assert.equal(new Set(tools).size, tools.length);
  assert.deepEqual(EDITOR_TOOL_GROUPS[2].tools, ["Rectangle", "Ellipse", "FilledRectangle", "FilledEllipse"]);
});

test("Duplicate Area remains a UI-only tool and emits a bounded service command", () => {
  assert.equal(normalizeEditor({ activeTool: "DuplicateArea" }).activeTool, "DuplicateArea");
  assert.deepEqual(backendCommand("duplicate_area", {
    start: { x: 40, y: 60 }, end: { x: 10, y: 20 },
  }), {
    type: "duplicate_area", start: { x: 40, y: 60 }, end: { x: 10, y: 20 },
  });
  assert.deepEqual(backendCommand("duplicate_selection"), { type: "duplicate_selection" });
  assert.throws(
    () => backendCommand("duplicate_area", { start: { x: 1, y: 2 }, end: { x: Number.NaN, y: 4 } }),
    /Invalid duplicate area/,
  );
  assert.throws(() => backendCommand("choose", { tool: "DuplicateArea" }), /UI-only/);
});

test("Ctrl+D duplicates only a selected object outside editable controls", () => {
  assert.equal(shouldDuplicateShortcut({ key: "d", ctrlKey: true, editing: false, repeat: false }, true), true);
  assert.equal(shouldDuplicateShortcut({ key: "D", ctrlKey: true, editing: true, repeat: false }, true), false);
  assert.equal(shouldDuplicateShortcut({ key: "d", ctrlKey: true, editing: false, repeat: false }, false), false);
  assert.equal(shouldDuplicateShortcut({ key: "d", ctrlKey: true, editing: false, repeat: true }, true), false);
});

test("canvas coordinates account for canvas bounds zoom and pan", () => {
  assert.deepEqual(
    screenToDocument({ x: 350, y: 260 }, { left: 100, top: 60 }, { zoom: 2, pan: { x: 50, y: 20 } }),
    { x: 100, y: 90 },
  );
});

test("visible editor coordinates translate through a cropped document origin exactly once", () => {
  assert.deepEqual(
    translateVisiblePoint({ x: 10, y: 12 }, { x: 360, y: 54 }),
    { x: 370, y: 66 },
  );
  assert.deepEqual(
    translateVisiblePoint({ x: 370, y: 66 }, { x: -360, y: -54 }),
    { x: 10, y: 12 },
  );
});

test("canvas shortcuts are disabled while text controls own the keyboard", () => {
  assert.equal(shouldHandleCanvasShortcuts({ editing: true, composing: false }), false);
  assert.equal(shouldHandleCanvasShortcuts({ editing: false, composing: true }), false);
  assert.equal(shouldHandleCanvasShortcuts({ editing: false, composing: false }), true);
});

test("zooming around the pointer keeps the same document point under it", () => {
  const viewport = { zoom: 1, pan: { x: 20, y: 30 } };
  const pointer = { x: 240, y: 180 };
  const before = screenToDocument(pointer, { left: 0, top: 0 }, viewport);
  const next = zoomAround(viewport, pointer, 2);
  const after = screenToDocument(pointer, { left: 0, top: 0 }, next);
  assert.deepEqual(after, before);
});

test("zoom buttons follow compact presets and clamp manual zoom to 25-400 percent", () => {
  assert.equal(steppedZoom(1, 1), 1.25);
  assert.equal(steppedZoom(1, -1), 0.75);
  assert.equal(steppedZoom(0.25, -1), 0.25);
  assert.equal(steppedZoom(4, 1), 4);
  assert.equal(steppedZoom(0.76, 1), 1);
});

test("backend commands carry typed Rust tool names and validated pointer data", () => {
  assert.deepEqual(backendCommand("choose", { tool: "Arrow" }), { type: "choose", tool: "Arrow" });
  assert.deepEqual(backendCommand("press", { point: { x: 12.25, y: 19.5 }, tolerance: 4 }), {
    type: "press", point: { x: 12.25, y: 19.5 }, tolerance: 4,
  });
  assert.throws(() => backendCommand("choose", { tool: "Magic" }), /Unsupported editor tool/);
  assert.throws(() => backendCommand("press", { point: { x: Number.NaN, y: 2 }, tolerance: 4 }), /Invalid pointer/);
  assert.deepEqual(backendCommand("reset_curve"), { type: "reset_curve" });
});

test("effect commands validate shadow and text outline settings", () => {
  const effects = {
    shadow: { enabled: true, rgb: [12, 34, 56], blur: 6, x: 3, y: -2 },
    outline: { enabled: true, rgb: [0, 0, 0], width: 2 },
  };
  assert.deepEqual(backendCommand("set_effects", { effects }), { type: "set_effects", effects });
  assert.throws(() => backendCommand("set_effects", {
    effects: { ...effects, shadow: { ...effects.shadow, blur: 33 } },
  }), /effect/i);
  assert.throws(() => backendCommand("set_effects", {
    effects: { ...effects, outline: { ...effects.outline, width: 0 } },
  }), /effect/i);
});

test("text commands carry a validated position, value and callout mode", () => {
  assert.deepEqual(backendCommand("add_text", {
    point: { x: 48, y: 72 }, text: "Hello\nSnapZy", callout: true,
  }), {
    type: "add_text", point: { x: 48, y: 72 }, text: "Hello\nSnapZy", callout: true,
  });
  assert.throws(() => backendCommand("add_text", {
    point: { x: 48, y: 72 }, text: "   ", callout: false,
  }), /nonblank/);
});

test("text styling commands validate fonts and transparent backgrounds", () => {
  assert.deepEqual(backendCommand("set_font", { font: "kanit" }), {
    type: "set_font", font: "kanit",
  });
  assert.throws(() => backendCommand("set_font", { font: "comic_sans" }), /font/i);
  assert.deepEqual(backendCommand("set_text_background", {
    background: { enabled: true, rgb: [245, 206, 58] },
  }), {
    type: "set_text_background",
    background: { enabled: true, rgb: [245, 206, 58] },
  });
  assert.deepEqual(backendCommand("set_text_background", {
    background: { enabled: false, rgb: [255, 255, 255] },
  }), {
    type: "set_text_background",
    background: { enabled: false, rgb: [255, 255, 255] },
  });
  assert.throws(() => backendCommand("set_text_background", {
    background: { enabled: true, rgb: [300, 0, 0] },
  }), /background/i);
});

test("PNG export keeps a readable base name and replaces the source extension", () => {
  assert.equal(pngDownloadName("daily report.jpg"), "daily report.png");
  assert.equal(pngDownloadName("capture"), "capture.png");
  assert.equal(pngDownloadName(""), "SnapZy-image.png");
});
