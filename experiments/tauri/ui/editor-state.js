const TOOLS = new Set([
  "Select", "Crop", "Pen", "Highlighter", "Line", "Arrow",
  "Rectangle", "Ellipse", "FilledRectangle", "FilledEllipse",
  "Blur", "Step", "Text", "Callout", "DuplicateArea",
]);

const BACKEND_TOOLS = new Set([...TOOLS].filter((tool) => tool !== "DuplicateArea"));

export const EDITOR_TOOL_GROUPS = Object.freeze([
  { id: "selection", tools: ["Select", "Crop", "DuplicateArea"] },
  { id: "drawing", tools: ["Pen", "Highlighter", "Line", "Arrow"] },
  { id: "shapes", tools: ["Rectangle", "Ellipse", "FilledRectangle", "FilledEllipse"] },
  { id: "content", tools: ["Blur", "Step", "Text", "Callout"] },
]);

const patterns = new Set(["Solid", "Dashed", "Dotted"]);
export const EDITOR_PALETTE = Object.freeze([
  "#000000", "#ffffff", "#4b5563", "#ef4444", "#f97316",
  "#facc15", "#22c55e", "#14b8a6", "#0ea5e9", "#2563eb",
  "#7c3aed", "#db2777", "#7f1d1d", "#92400e", "#3f6212",
  "#166534", "#155e75", "#1e3a8a", "#581c87", "#831843",
]);
export const EDITOR_FONTS = Object.freeze([
  { id: "segoe_ui", label: "Segoe UI", family: '"Segoe UI", sans-serif' },
  { id: "arial", label: "Arial", family: "Arial, sans-serif" },
  { id: "calibri", label: "Calibri", family: "Calibri, sans-serif" },
  { id: "tahoma", label: "Tahoma", family: "Tahoma, sans-serif" },
  { id: "kanit", label: "Kanit", family: "Kanit, sans-serif" },
  { id: "prompt", label: "Prompt", family: "Prompt, sans-serif" },
  { id: "noto_sans_thai", label: "Noto Sans Thai", family: '"Noto Sans Thai", sans-serif' },
  { id: "sarabun", label: "Sarabun", family: "Sarabun, sans-serif" },
]);
const fonts = new Set(EDITOR_FONTS.map((font) => font.id));
const selectionKinds = new Set(["image", "shape", "region", "line", "stroke", "text", "step"]);
const handleRoles = new Set([
  "top_left", "top", "top_right", "right", "bottom_right", "bottom", "bottom_left", "left",
  "start", "end", "curve", "tail",
]);

export const ZOOM_STEPS = Object.freeze([0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4]);

function boundedPoint(point) {
  return point
    && Number.isFinite(point.x)
    && Number.isFinite(point.y)
    && Math.abs(point.x) <= 32768
    && Math.abs(point.y) <= 32768
    ? { x: point.x, y: point.y }
    : null;
}

export function normalizeSelectionOverlay(input) {
  if (!input || typeof input !== "object" || !selectionKinds.has(input.kind) || !Array.isArray(input.handles) || input.handles.length > 12) return null;
  const bounds = input.bounds == null
    ? null
    : { start: boundedPoint(input.bounds.start), end: boundedPoint(input.bounds.end) };
  if (bounds && (!bounds.start || !bounds.end)) return null;
  const handles = input.handles.map((handle) => ({
    role: handleRoles.has(handle?.role) ? handle.role : null,
    point: boundedPoint(handle?.point),
  }));
  if (handles.some((handle) => !handle.role || !handle.point)) return null;
  return { kind: input.kind, bounds, handles, movable: input.movable === true };
}

export function selectionOverlayToScreen(overlay, viewport) {
  const point = (value) => ({
    x: viewport.pan.x + value.x * viewport.zoom,
    y: viewport.pan.y + value.y * viewport.zoom,
  });
  return {
    ...overlay,
    bounds: overlay.bounds ? { start: point(overlay.bounds.start), end: point(overlay.bounds.end) } : null,
    handles: overlay.handles.map((handle) => ({ ...handle, point: point(handle.point), size: 10 })),
  };
}

export function selectionHandleAt(overlay, pointer, viewport) {
  if (!overlay || !pointer || !Number.isFinite(pointer.x) || !Number.isFinite(pointer.y)) return null;
  const screen = selectionOverlayToScreen(overlay, viewport);
  return screen.handles.find((handle) => {
    const radius = handle.size / 2 + 2;
    return Math.abs(handle.point.x - pointer.x) <= radius && Math.abs(handle.point.y - pointer.y) <= radius;
  }) ?? null;
}

export function selectionGuide(screenOverlay) {
  if (!screenOverlay) return null;
  if (screenOverlay.kind === "line") {
    const start = screenOverlay.handles.find((handle) => handle.role === "start")?.point;
    const end = screenOverlay.handles.find((handle) => handle.role === "end")?.point;
    const midpoint = screenOverlay.handles.find((handle) => handle.role === "curve")?.point;
    if (!start || !end || !midpoint) return null;
    return {
      kind: "quadratic",
      start,
      end,
      control: {
        x: 2 * midpoint.x - (start.x + end.x) / 2,
        y: 2 * midpoint.y - (start.y + end.y) / 2,
      },
      dash: [5, 4],
    };
  }
  if (!screenOverlay.bounds) return null;
  return {
    kind: "rectangle",
    start: screenOverlay.bounds.start,
    end: screenOverlay.bounds.end,
    dash: [5, 4],
  };
}

const defaultEffects = Object.freeze({
  shadow: Object.freeze({ enabled: false, rgb: Object.freeze([0, 0, 0]), blur: 6, x: 3, y: 3 }),
  outline: Object.freeze({ enabled: false, rgb: Object.freeze([0, 0, 0]), width: 2 }),
});

function effectRgb(value) {
  return Array.isArray(value)
    && value.length === 3
    && value.every((part) => Number.isInteger(part) && part >= 0 && part <= 255)
    ? [...value]
    : null;
}

export function normalizeEffects(input, fallback = defaultEffects) {
  const shadowRgb = effectRgb(input?.shadow?.rgb);
  const outlineRgb = effectRgb(input?.outline?.rgb);
  const valid = typeof input?.shadow?.enabled === "boolean"
    && shadowRgb
    && Number.isInteger(input.shadow.blur) && input.shadow.blur >= 0 && input.shadow.blur <= 32
    && Number.isInteger(input.shadow.x) && input.shadow.x >= -32 && input.shadow.x <= 32
    && Number.isInteger(input.shadow.y) && input.shadow.y >= -32 && input.shadow.y <= 32
    && typeof input?.outline?.enabled === "boolean"
    && outlineRgb
    && Number.isInteger(input.outline.width) && input.outline.width >= 1 && input.outline.width <= 16;
  if (!valid) return fallback === null ? null : normalizeEffects(fallback, null);
  return {
    shadow: { ...input.shadow, rgb: shadowRgb },
    outline: { ...input.outline, rgb: outlineRgb },
  };
}

export function normalizeEditor(input) {
  const value = input && typeof input === "object" ? input : {};
  const pan = value.pan && Number.isFinite(value.pan.x) && Number.isFinite(value.pan.y)
    ? { x: value.pan.x, y: value.pan.y }
    : { x: 0, y: 0 };
  const view = value.view && typeof value.view === "object"
    ? {
      ...value.view,
      effects: normalizeEffects(value.view.effects),
      outline_supported: value.view.outline_supported === true,
      selection_overlay: normalizeSelectionOverlay(value.view.selection_overlay),
    }
    : null;
  return {
    language: value.language === "th" ? "th" : "en",
    activeTool: TOOLS.has(value.activeTool) ? value.activeTool : "Select",
    color: /^#[0-9a-f]{6}$/i.test(value.color) ? value.color.toLowerCase() : "#ff0000",
    parameter: Number.isInteger(value.parameter) && value.parameter >= 1 && value.parameter <= 256 ? value.parameter : 5,
    opacity: Number.isInteger(value.opacity) && value.opacity >= 0 && value.opacity <= 100 ? value.opacity : 100,
    pattern: patterns.has(value.pattern) ? value.pattern : "Solid",
    font: fonts.has(value.font) ? value.font : "segoe_ui",
    textBackground: {
      enabled: value.textBackground?.enabled === true,
      color: /^#[0-9a-f]{6}$/i.test(value.textBackground?.color)
        ? value.textBackground.color.toLowerCase()
        : "#ffffff",
    },
    zoom: Number.isFinite(value.zoom) && value.zoom >= 0.05 && value.zoom <= 16 ? value.zoom : 1,
    pan,
    view,
  };
}

export function screenToDocument(point, bounds, viewport) {
  return {
    x: (point.x - bounds.left - viewport.pan.x) / viewport.zoom,
    y: (point.y - bounds.top - viewport.pan.y) / viewport.zoom,
  };
}

export function zoomAround(viewport, pointer, requestedZoom) {
  const zoom = Math.min(4, Math.max(0.25, requestedZoom));
  const documentPoint = {
    x: (pointer.x - viewport.pan.x) / viewport.zoom,
    y: (pointer.y - viewport.pan.y) / viewport.zoom,
  };
  return {
    zoom,
    pan: {
      x: pointer.x - documentPoint.x * zoom,
      y: pointer.y - documentPoint.y * zoom,
    },
  };
}

export function steppedZoom(current, direction) {
  if (!Number.isFinite(current) || ![-1, 1].includes(direction)) return 1;
  if (direction > 0) return ZOOM_STEPS.find((value) => value > current + 0.001) ?? ZOOM_STEPS.at(-1);
  return [...ZOOM_STEPS].reverse().find((value) => value < current - 0.001) ?? ZOOM_STEPS[0];
}

export function shouldDuplicateShortcut(event, selected) {
  return selected === true
    && event?.ctrlKey === true
    && event?.repeat !== true
    && event?.editing !== true
    && String(event?.key ?? "").toLowerCase() === "d";
}

export function contextMenuActions(selected) {
  return selected === true
    ? ["duplicate_selection", "layers", "delete"]
    : ["duplicate_area"];
}

export function clampContextMenuPosition(anchor, size, viewport, margin = 8) {
  const maxX = Math.max(margin, viewport.width - size.width - margin);
  const maxY = Math.max(margin, viewport.height - size.height - margin);
  return {
    x: Math.min(maxX, Math.max(margin, anchor.x)),
    y: Math.min(maxY, Math.max(margin, anchor.y)),
  };
}

export function moveMenuFocus(current, key, length) {
  if (!Number.isInteger(length) || length < 1) return -1;
  if (key === "Home") return 0;
  if (key === "End") return length - 1;
  if (key === "ArrowDown") return (current + 1 + length) % length;
  if (key === "ArrowUp") return (current - 1 + length) % length;
  return current;
}

export function isContextMenuKey(event) {
  return event?.key === "ContextMenu" || (event?.key === "F10" && event?.shiftKey === true);
}

export function createCommandQueue(execute) {
  if (typeof execute !== "function") throw new TypeError("Command queue requires an executor.");
  let tail = Promise.resolve();
  return (...args) => {
    const current = tail.then(() => execute(...args));
    tail = current.catch(() => {});
    return current;
  };
}

export function backendCommand(type, payload = {}) {
  if (type === "choose") {
    if (payload.tool === "DuplicateArea") throw new Error("Duplicate Area is a UI-only tool.");
    if (!BACKEND_TOOLS.has(payload.tool)) throw new Error("Unsupported editor tool.");
    return { type, tool: payload.tool };
  }
  if (["press", "move", "release"].includes(type)) {
    const point = payload.point;
    if (!point || !Number.isFinite(point.x) || !Number.isFinite(point.y)) throw new Error("Invalid pointer input.");
    if (type === "press") {
      if (!Number.isFinite(payload.tolerance) || payload.tolerance < 0.5 || payload.tolerance > 100) throw new Error("Invalid pointer tolerance.");
      return { type, point: { x: point.x, y: point.y }, tolerance: payload.tolerance };
    }
    return { type, point: { x: point.x, y: point.y } };
  }
  if (["confirm_crop", "cancel", "delete", "undo", "redo", "reset_curve"].includes(type)) return { type };
  if (type === "duplicate_selection") return { type };
  if (type === "duplicate_area") {
    const { start, end } = payload;
    if (![start, end].every((point) => point && Number.isFinite(point.x) && Number.isFinite(point.y))) {
      throw new Error("Invalid duplicate area.");
    }
    return {
      type,
      start: { x: start.x, y: start.y },
      end: { x: end.x, y: end.y },
    };
  }
  if (type === "add_text") {
    const point = payload.point;
    const text = typeof payload.text === "string" ? payload.text : "";
    if (!point || !Number.isFinite(point.x) || !Number.isFinite(point.y)) throw new Error("Invalid text position.");
    if (!text.trim() || [...text].length > 4096) throw new Error("Text must contain 1-4096 nonblank characters.");
    return { type, point: { x: point.x, y: point.y }, text, callout: payload.callout === true };
  }
  if (type === "set_color") return { type, rgb: payload.rgb };
  if (type === "set_parameter" || type === "set_opacity") return { type, value: payload.value };
  if (type === "set_pattern") return { type, pattern: payload.pattern };
  if (type === "set_font") {
    if (!fonts.has(payload.font)) throw new Error("Unsupported text font.");
    return { type, font: payload.font };
  }
  if (type === "set_text_background") {
    const background = payload.background;
    if (typeof background?.enabled !== "boolean" || !effectRgb(background.rgb)) {
      throw new Error("Invalid text background.");
    }
    return { type, background: { enabled: background.enabled, rgb: [...background.rgb] } };
  }
  if (type === "set_effects") {
    const effects = normalizeEffects(payload.effects, null);
    if (!effects) throw new Error("Invalid effect settings.");
    return { type, effects };
  }
  if (type === "reorder") return { type, order: payload.order };
  throw new Error("Unsupported editor command.");
}

export function hexToRgb(value) {
  const normalized = /^#[0-9a-f]{6}$/i.test(value) ? value.slice(1) : "ff0000";
  return [0, 2, 4].map((offset) => Number.parseInt(normalized.slice(offset, offset + 2), 16));
}

export function rgbToHex(rgb) {
  return `#${rgb.map((value) => Math.min(255, Math.max(0, value)).toString(16).padStart(2, "0")).join("")}`;
}

export function pngDownloadName(sourceName) {
  const name = typeof sourceName === "string" ? sourceName.trim() : "";
  if (!name) return "SnapZy-image.png";
  const base = name.replace(/\.[^.\\/]+$/, "");
  return `${base || "SnapZy-image"}.png`;
}
