import "./editor.css";
import logo from "../../../assets/snapzy-rust.png";
import packageMetadata from "../package.json";
import {
  createIcons, X, Plus, Ellipsis, ScanText, CircleHelp, Copy, Download, MousePointer2, Crop, SquareDashedMousePointer,
  PenLine, Highlighter, Minus, MoveUpRight, Square, Circle, ChevronDown, Droplets,
  Type, MessageSquare, Sparkles, Layers3, Trash2, Undo2, Redo2, Check, Maximize, ImagePlus,
  FolderOpen, ArrowUpToLine, ArrowUp, ArrowDown, ArrowDownToLine, Folder, Save, Images, ZoomIn, ZoomOut,
} from "lucide";
import {
  EDITOR_FONTS, EDITOR_PALETTE, EDITOR_TOOL_GROUPS, backendCommand, clampContextMenuPosition, contextMenuActions, createCommandQueue, hexToRgb, isContextMenuKey,
  moveMenuFocus, normalizeEditor, normalizeEffects, rgbToHex, screenToDocument, selectionGuide, selectionHandleAt, selectionOverlayToScreen, shouldDuplicateShortcut, steppedZoom, zoomAround, ZOOM_STEPS, pngDownloadName,
} from "./editor-state.js";
import {
  commandForSession, normalizeWorkspace, removeWorkspaceTab, selectWorkspaceTab,
} from "./editor-workspace.js";
import { editorText, toolTextKey } from "./editor-i18n.js";

const icons = { X, Plus, Ellipsis, ScanText, CircleHelp, Copy, Download, MousePointer2, Crop, SquareDashedMousePointer, PenLine, Highlighter, Minus, MoveUpRight, Square, Circle, ChevronDown, Droplets, Type, MessageSquare, Sparkles, Layers3, Trash2, Undo2, Redo2, Check, Maximize, ImagePlus, FolderOpen, ArrowUpToLine, ArrowUp, ArrowDown, ArrowDownToLine, Folder, Save, Images, ZoomIn, ZoomOut };
const $ = (selector) => document.querySelector(selector);
const native = Boolean(window.__TAURI__?.core?.invoke);
const invoke = (command, args = {}) => window.__TAURI__.core.invoke(command, args);
const storageKey = "snapzy-tauri-trial-preferences-v1";
let language = "en";
try { language = JSON.parse(localStorage.getItem(storageKey))?.language === "th" ? "th" : "en"; } catch { /* Optional storage. */ }
let state = normalizeEditor({ language });
let workspace = normalizeWorkspace({});
let loadedSessionId = null;
let previewImage = null;
let previewUrl = null;
let busy = false;
let menuOwner = null;
let tooltipOwner = null;
let tooltipTimer = 0;
let pointer = null;
let draft = null;
let mockObjects = [];
let moveFrame = 0;
let queuedMove = null;
let spaceDown = false;
let textPoint = null;
let workspaceRefreshPending = false;
let stopWorkspaceListener = null;
const sessionViewports = new Map();
const recentColorKey = "snapzy-editor-recent-colors-v1";
let recentColors = [];
let customColorAction = null;
try {
  recentColors = JSON.parse(localStorage.getItem(recentColorKey) || "[]")
    .filter((color) => /^#[0-9a-f]{6}$/i.test(color))
    .map((color) => color.toLowerCase())
    .filter((color, index, all) => all.indexOf(color) === index)
    .slice(0, 6);
} catch { /* Optional storage. */ }

const t = (key) => editorText(state.language, key);

function activeTab() {
  return workspace.tabs.find((tab) => tab.sessionId === workspace.activeSessionId) ?? null;
}

function refreshIcons(root = document) {
  createIcons({ icons, attrs: { "stroke-width": 1.65, "aria-hidden": "true" }, nameAttr: "data-lucide", root });
}

function renderLanguage() {
  document.documentElement.lang = state.language;
  document.querySelectorAll("[data-i18n]").forEach((element) => { element.textContent = t(element.dataset.i18n); });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((element) => { element.placeholder = t(element.dataset.i18nPlaceholder); });
  document.querySelectorAll("[data-tooltip]").forEach((element) => { element.setAttribute("aria-label", t(element.dataset.tooltip)); });
  $("#editor-language").textContent = state.language.toUpperCase();
  document.title = `SnapZy ${packageMetadata.version} | ${t("imageEditor")}`;
  if ($("#text-dialog").open) renderTextDialogTitle();
  buildHelp();
  renderTabs();
}

function renderTabs() {
  const container = $("#document-tabs");
  container.replaceChildren();
  for (const tab of workspace.tabs) {
    const item = document.createElement("div");
    item.className = `document-tab${tab.sessionId === workspace.activeSessionId ? " active" : ""}`;
    item.role = "tab";
    item.tabIndex = tab.sessionId === workspace.activeSessionId ? 0 : -1;
    item.setAttribute("aria-selected", String(tab.sessionId === workspace.activeSessionId));
    item.dataset.sessionId = tab.sessionId;
    item.innerHTML = '<span class="unsaved-dot" aria-hidden="true"></span><span class="tab-name"></span><button class="tab-close" type="button"><i data-lucide="x"></i></button>';
    item.querySelector(".tab-name").textContent = tab.title;
    const close = item.querySelector(".tab-close");
    close.setAttribute("aria-label", t("closeDocument"));
    close.dataset.tooltip = "closeDocument";
    item.addEventListener("click", (event) => {
      if (!event.target.closest(".tab-close")) void activateSession(tab.sessionId);
    });
    close.addEventListener("click", (event) => {
      event.stopPropagation();
      void closeSession(tab.sessionId);
    });
    container.append(item);
  }
  refreshIcons(container);
  const tab = activeTab();
  $("#document-title").textContent = tab?.title ?? t("imageEditor");
}

function renderControls() {
  document.querySelectorAll("[data-tool]").forEach((button) => button.classList.toggle("selected", button.dataset.tool === state.activeTool));
  setColorButton($("#tool-color"), state.color);
  $("#tool-size").value = state.parameter;
  $("#line-pattern .line-sample").dataset.pattern = state.pattern;
  $("#line-pattern").setAttribute("aria-label", `${t("lineStyle")}: ${t(state.pattern.toLowerCase())}`);
  $("#tool-opacity").value = state.opacity;
  $("#opacity-value").textContent = `${state.opacity}%`;
  $("#zoom-value").textContent = `${Math.round(state.zoom * 100)}%`;
  const view = state.view;
  $("#document-size").textContent = view ? `${view.width} × ${view.height} px` : t("noImage");
  $("#empty-state").hidden = Boolean(view);
  $("#copy-button").disabled = !view || busy;
  $("#save-png").disabled = !view || busy;
  $("#undo-button").disabled = !view?.can_undo || busy;
  $("#redo-button").disabled = !view?.can_redo || busy;
  $("#delete-button").disabled = !view?.can_delete || busy;
  $("#duplicate-button").disabled = !view?.selected || busy;
  $("#effects-button").disabled = !view || busy;
  $("#confirm-crop").hidden = !view?.crop_ready;
  $("#zoom-out").disabled = !view || busy || state.zoom <= ZOOM_STEPS[0];
  $("#zoom-in").disabled = !view || busy || state.zoom >= ZOOM_STEPS.at(-1);
  $("#zoom-value").disabled = !view || busy;
  $("#fit-view").disabled = !view || busy;
  const textActive = ["Text", "Callout"].includes(state.activeTool) || view?.outline_supported === true;
  $("#font-control").hidden = !textActive;
  $("#text-background-control").hidden = !textActive;
  $("#text-font").value = state.font;
  $("#text-font").style.fontFamily = EDITOR_FONTS.find((font) => font.id === state.font)?.family ?? "Segoe UI";
  $("#text-background-enabled").checked = state.textBackground.enabled;
  setColorButton($("#text-background-color"), state.textBackground.color);
  $("#text-background-color").disabled = busy;
  $("#editor-status").textContent = t(busy ? "working" : "ready");
}

function setColorButton(button, color) {
  const normalized = /^#[0-9a-f]{6}$/i.test(color) ? color.toLowerCase() : "#000000";
  button.dataset.color = normalized;
  button.querySelector(".color-swatch")?.style.setProperty("--swatch", normalized);
}

function rememberColor(color) {
  recentColors = [color.toLowerCase(), ...recentColors.filter((item) => item !== color.toLowerCase())].slice(0, 6);
  try { localStorage.setItem(recentColorKey, JSON.stringify(recentColors)); } catch { /* Optional storage. */ }
}

function setBusy(value) {
  busy = value;
  renderControls();
  if (!value && workspaceRefreshPending) setTimeout(() => void refreshWorkspaceEvent(), 0);
}

async function refreshWorkspaceEvent() {
  if (!native) return;
  if (busy) {
    workspaceRefreshPending = true;
    return;
  }
  workspaceRefreshPending = false;
  try {
    await reloadWorkspace();
    if (workspace.activeSessionId) await activateSession(workspace.activeSessionId);
  } catch (error) {
    notice(`${t("openFailed")} ${String(error)}`);
  }
}

function notice(message) {
  $("#editor-notice-text").textContent = message;
  $("#editor-notice").hidden = false;
}

function clearNotice() { $("#editor-notice").hidden = true; }

function placeFloating(element, anchor, gap = 6) {
  const bounds = anchor.getBoundingClientRect();
  element.hidden = false;
  const size = element.getBoundingClientRect();
  const left = Math.min(Math.max(8, bounds.left), Math.max(8, innerWidth - size.width - 8));
  const top = bounds.bottom + gap + size.height <= innerHeight ? bounds.bottom + gap : Math.max(8, bounds.top - size.height - gap);
  element.style.left = `${left}px`;
  element.style.top = `${top}px`;
}

function dismissTooltip() {
  clearTimeout(tooltipTimer);
  $("#editor-tooltip").hidden = true;
  tooltipOwner?.removeAttribute("aria-describedby");
  tooltipOwner = null;
}

function closeMenu(restore = false) {
  const popup = $("#editor-popup");
  popup.hidden = true;
  popup.classList.remove("palette-popup", "pattern-popup");
  if (menuOwner) {
    menuOwner.setAttribute("aria-expanded", "false");
    if (restore) menuOwner.focus();
  }
  menuOwner = null;
}

function applyPickedColor(target, color) {
  rememberColor(color);
  if (target === "tool") {
    state = { ...state, color };
    renderControls();
    renderCanvas();
    void applyCommand(backendCommand("set_color", { rgb: hexToRgb(color) }), false);
  } else if (target === "background") {
    state = { ...state, textBackground: { ...state.textBackground, color } };
    renderControls();
    void applyCommand(backendCommand("set_text_background", {
      background: { enabled: state.textBackground.enabled, rgb: hexToRgb(color) },
    }), false);
  } else {
    setColorButton($(`#${target}-color`), color);
  }
}

function paletteGrid(colors, current, target) {
  const grid = document.createElement("div");
  grid.className = "palette-grid";
  for (const color of colors) {
    const button = document.createElement("button");
    button.type = "button";
    button.role = "menuitem";
    button.className = `palette-swatch${color === current ? " selected" : ""}`;
    button.style.setProperty("--swatch", color);
    button.setAttribute("aria-label", color);
    button.addEventListener("click", () => {
      closeMenu(true);
      applyPickedColor(target, color);
    });
    grid.append(button);
  }
  return grid;
}

function showPalette(owner) {
  if (menuOwner === owner) return closeMenu(true);
  closeMenu();
  dismissTooltip();
  const popup = $("#editor-popup");
  const modal = owner.closest("dialog[open]");
  (modal ?? document.body).append(popup, $("#custom-color-input"));
  const target = owner.dataset.colorTarget;
  const current = owner.dataset.color || "#000000";
  popup.replaceChildren();
  popup.classList.add("palette-popup");
  const title = document.createElement("div");
  title.className = "palette-heading";
  title.textContent = t("paletteColors");
  popup.append(title, paletteGrid(EDITOR_PALETTE, current, target));
  if (recentColors.length) {
    const recent = document.createElement("div");
    recent.className = "palette-recent";
    const heading = document.createElement("div");
    heading.className = "palette-heading";
    heading.textContent = t("recentColors");
    recent.append(heading, paletteGrid(recentColors, current, target));
    popup.append(recent);
  }
  const custom = document.createElement("button");
  custom.type = "button";
  custom.role = "menuitem";
  custom.className = "palette-custom";
  custom.innerHTML = '<span class="palette-custom-mark" aria-hidden="true"></span><span></span>';
  custom.querySelector("span:last-child").textContent = t("customColor");
  custom.addEventListener("click", () => {
    customColorAction = (color) => applyPickedColor(target, color);
    const input = $("#custom-color-input");
    input.value = current;
    input.click();
  });
  popup.append(custom);
  menuOwner = owner;
  owner.setAttribute("aria-expanded", "true");
  placeFloating(popup, owner);
  popup.querySelector("button")?.focus();
}

function popupItem(label, icon, action, selected = false, actionId = "") {
  const button = document.createElement("button");
  button.type = "button";
  button.role = "menuitem";
  button.classList.toggle("selected", selected);
  button.classList.toggle("danger", actionId === "delete");
  if (actionId === "layers") button.setAttribute("aria-haspopup", "menu");
  button.innerHTML = `<i data-lucide="${icon}"></i><span></span>`;
  if (actionId) button.dataset.contextAction = actionId;
  button.querySelector("span").textContent = label;
  button.addEventListener("click", () => { closeMenu(true); action(); });
  return button;
}

function placeMenuAt(anchor) {
  const popup = $("#editor-popup");
  popup.hidden = false;
  const bounds = popup.getBoundingClientRect();
  const position = clampContextMenuPosition(
    anchor,
    { width: bounds.width, height: bounds.height },
    { width: innerWidth, height: innerHeight },
  );
  popup.style.left = `${position.x}px`;
  popup.style.top = `${position.y}px`;
}

function showContextMenu(anchor, selected) {
  closeMenu();
  dismissTooltip();
  const popup = $("#editor-popup");
  document.body.append(popup);
  popup.replaceChildren();
  for (const action of contextMenuActions(selected)) {
    if (action === "duplicate_selection") {
      popup.append(popupItem(t("duplicateObject"), "copy", () => applyCommand(backendCommand("duplicate_selection")), false, action));
    } else if (action === "layers") {
      popup.append(popupItem(t("layers"), "layers-3", () => showMenu($("#layers-button")), false, action));
    } else if (action === "delete") {
      popup.append(popupItem(t("delete"), "trash-2", () => applyCommand(backendCommand("delete")), false, action));
    } else if (action === "duplicate_area") {
      popup.append(popupItem(t("duplicateArea"), "square-dashed-mouse-pointer", () => chooseTool("DuplicateArea"), false, action));
    }
  }
  menuOwner = $("#editor-canvas");
  menuOwner.setAttribute("aria-expanded", "true");
  refreshIcons(popup);
  placeMenuAt(anchor);
  popup.querySelector("button")?.focus();
}

function showMenu(owner) {
  if (menuOwner === owner) return closeMenu(true);
  closeMenu();
  dismissTooltip();
  const popup = $("#editor-popup");
  document.body.append(popup);
  popup.replaceChildren();
  const kind = owner.dataset.menu;
  if (kind === "outline-shape") {
    popup.append(
      popupItem(t("rectangle"), "square", () => chooseTool("Rectangle"), state.activeTool === "Rectangle"),
      popupItem(t("ellipse"), "circle", () => chooseTool("Ellipse"), state.activeTool === "Ellipse"),
    );
  } else if (kind === "filled-shape") {
    popup.append(
      popupItem(t("filledRectangle"), "square", () => chooseTool("FilledRectangle"), state.activeTool === "FilledRectangle"),
      popupItem(t("filledEllipse"), "circle", () => chooseTool("FilledEllipse"), state.activeTool === "FilledEllipse"),
    );
  } else if (kind === "layers") {
    for (const [key, icon, order] of [["bringFront", "arrow-up-to-line", "front"], ["bringForward", "arrow-up", "forward"], ["sendBackward", "arrow-down", "backward"], ["sendBack", "arrow-down-to-line", "back"]]) {
      popup.append(popupItem(t(key), icon, () => applyCommand(backendCommand("reorder", { order }))));
    }
  } else if (kind === "zoom") {
    for (const zoom of ZOOM_STEPS) {
      popup.append(popupItem(`${Math.round(zoom * 100)}%`, "zoom-in", () => setZoomAtCenter(zoom), Math.abs(state.zoom - zoom) < 0.001));
    }
  } else if (kind === "pattern") {
    popup.classList.add("pattern-popup");
    for (const pattern of ["Solid", "Dashed", "Dotted"]) {
      const button = document.createElement("button");
      button.type = "button";
      button.role = "menuitem";
      button.className = `pattern-item${state.pattern === pattern ? " selected" : ""}`;
      button.setAttribute("aria-label", t(pattern.toLowerCase()));
      button.innerHTML = `<span class="line-sample" data-pattern="${pattern}"></span>`;
      button.addEventListener("click", () => {
        closeMenu(true);
        state = { ...state, pattern };
        renderControls();
        void applyCommand(backendCommand("set_pattern", { pattern }), false);
      });
      popup.append(button);
    }
  } else {
    popup.append(
      popupItem(t("effects"), "sparkles", openEffects),
      popupItem(t("openImage"), "folder-open", () => $("#open-image").click()),
      popupItem(t("addImage"), "image-plus", () => notice(t("notAvailable"))),
      popupItem(t("combine"), "images", () => notice(t("notAvailable"))),
      popupItem(t("openProject"), "folder", () => notice(t("notAvailable"))),
      popupItem(t("saveProject"), "save", () => notice(t("notAvailable"))),
    );
  }
  menuOwner = owner;
  owner.setAttribute("aria-expanded", "true");
  refreshIcons(popup);
  placeFloating(popup, owner);
  popup.querySelector("button")?.focus();
}

async function applyCommand(command, refresh = true, allowWhileBusy = false) {
  if (!state.view || (busy && !allowWhileBusy)) return;
  if (!native) {
    mockApply(command);
    renderControls();
    renderCanvas();
    return;
  }
  setBusy(true);
  clearNotice();
  try {
    const view = await invoke("editor_apply", commandForSession(workspace.activeSessionId, { command }));
    syncView(view);
    if (refresh) await refreshPreview();
    else renderCanvas();
  } catch (error) {
    notice(`${t("commandFailed")} ${String(error)}`);
  } finally {
    setBusy(false);
  }
}

const applyPointerCommand = createCommandQueue(
  (command, refresh = false) => applyCommand(command, refresh, true),
);

function mockApply(command) {
  if (command.type === "choose") state = { ...state, activeTool: command.tool };
  if (command.type === "set_color") state = { ...state, color: rgbToHex(command.rgb) };
  if (command.type === "set_parameter") state = { ...state, parameter: command.value };
  if (command.type === "set_opacity") state = { ...state, opacity: command.value };
  if (command.type === "set_pattern") state = { ...state, pattern: command.pattern };
  if (command.type === "set_font") state = { ...state, font: command.font };
  if (command.type === "set_text_background") state = {
    ...state,
    textBackground: { enabled: command.background.enabled, color: rgbToHex(command.background.rgb) },
  };
  if (command.type === "set_effects" && state.view) state = { ...state, view: { ...state.view, effects: command.effects, can_undo: true } };
  if (command.type === "add_text") mockObjects.push({
    tool: command.callout ? "Callout" : "Text",
    points: [command.point],
    text: command.text,
    color: state.color,
    parameter: Math.max(8, state.parameter),
    opacity: state.opacity,
    pattern: "Solid",
    font: state.font,
    background: state.textBackground,
  });
  if (command.type === "add_text") state = {
    ...state,
    activeTool: "Select",
    view: state.view ? { ...state.view, outline_supported: true } : state.view,
  };
  if (command.type === "duplicate_selection" && mockObjects.length) {
    const source = mockObjects.at(-1);
    mockObjects.push({ ...source, points: source.points.map((point) => ({ x: point.x + 16, y: point.y + 16 })) });
    state = { ...state, activeTool: "Select" };
  }
  if (command.type === "duplicate_area") {
    mockObjects.push({ tool: "DuplicateAreaResult", points: [command.start, command.end] });
    state = { ...state, activeTool: "Select", view: state.view ? { ...state.view, outline_supported: false } : state.view };
  }
  if (command.type === "undo" && mockObjects.length) mockObjects.pop();
  if (command.type === "delete" && mockObjects.length) mockObjects.pop();
  if (state.view) state.view = { ...state.view, selected: mockObjects.length > 0, can_undo: mockObjects.length > 0, can_redo: false, can_delete: mockObjects.length > 0 };
}

function syncView(view) {
  state = normalizeEditor({
    ...state,
    activeTool: view.tool,
    color: rgbToHex(view.color),
    parameter: view.parameter,
    opacity: view.opacity,
    pattern: view.pattern,
    font: view.font,
    textBackground: {
      enabled: view.text_background?.enabled === true,
      color: rgbToHex(view.text_background?.rgb ?? [255, 255, 255]),
    },
    view,
  });
  renderControls();
}

async function reloadWorkspace(preferredSessionId = null) {
  if (!native) return;
  const snapshot = await invoke("editor_tabs");
  workspace = normalizeWorkspace(snapshot);
  if (preferredSessionId && workspace.tabs.some((tab) => tab.sessionId === preferredSessionId)) {
    workspace = selectWorkspaceTab(workspace, preferredSessionId);
  }
  renderTabs();
}

async function activateSession(sessionId) {
  if (!native || busy || sessionId === loadedSessionId && state.view) return;
  rememberViewport();
  setBusy(true);
  clearNotice();
  try {
    const view = await invoke("editor_activate", { sessionId });
    workspace = selectWorkspaceTab(workspace, sessionId);
    loadedSessionId = sessionId;
    pointer = null;
    draft = null;
    syncView(view);
    renderTabs();
    await refreshPreview();
    requestAnimationFrame(() => restoreViewport(sessionId));
  } catch (error) {
    notice(`${t("openFailed")} ${String(error)}`);
  } finally {
    setBusy(false);
  }
}

async function closeSession(sessionId) {
  if (!native || busy) return;
  setBusy(true);
  clearNotice();
  try {
    const snapshot = await invoke("editor_close", { sessionId });
    sessionViewports.delete(sessionId);
    workspace = normalizeWorkspace(snapshot);
    renderTabs();
    if (!workspace.activeSessionId) {
      workspace = removeWorkspaceTab(workspace, sessionId);
      if (previewUrl) URL.revokeObjectURL(previewUrl);
      previewUrl = null;
      previewImage = null;
      loadedSessionId = null;
      state = normalizeEditor({ language: state.language });
      renderControls();
      renderCanvas();
      await window.__TAURI__?.window?.getCurrentWindow?.().close();
      return;
    }
    const view = await invoke("editor_activate", { sessionId: workspace.activeSessionId });
    loadedSessionId = workspace.activeSessionId;
    syncView(view);
    await refreshPreview();
    requestAnimationFrame(() => restoreViewport(workspace.activeSessionId));
  } catch (error) {
    notice(`${t("commandFailed")} ${String(error)}`);
  } finally {
    setBusy(false);
  }
}

async function bootstrapWorkspace() {
  if (!native) return;
  try {
    await reloadWorkspace();
    if (workspace.activeSessionId) {
      const view = await invoke("editor_activate", { sessionId: workspace.activeSessionId });
      loadedSessionId = workspace.activeSessionId;
      syncView(view);
      await refreshPreview();
      requestAnimationFrame(fitView);
    }
  } catch (error) {
    notice(`${t("openFailed")} ${String(error)}`);
  }
}

async function chooseTool(tool) {
  closeMenu();
  state = { ...state, activeTool: tool };
  if (!native && ["Text", "Callout"].includes(tool) && state.parameter < 8) state = { ...state, parameter: 24 };
  renderControls();
  if (native && state.view && tool !== "DuplicateArea") await applyCommand(backendCommand("choose", { tool }), false);
}

function renderTextDialogTitle() {
  $("#text-dialog-title").textContent = t($("#text-dialog").dataset.kind === "Callout" ? "addCalloutTitle" : "addTextTitle");
}

function setEffectFields() {
  const effects = normalizeEffects(state.view?.effects);
  $("#shadow-enabled").checked = effects.shadow.enabled;
  setColorButton($("#shadow-color"), rgbToHex(effects.shadow.rgb));
  $("#shadow-blur").value = effects.shadow.blur;
  $("#shadow-x").value = effects.shadow.x;
  $("#shadow-y").value = effects.shadow.y;
  $("#outline-enabled").checked = effects.outline.enabled;
  setColorButton($("#outline-color"), rgbToHex(effects.outline.rgb));
  $("#outline-width").value = effects.outline.width;
  const supported = state.view?.outline_supported === true;
  $("#outline-section").classList.toggle("disabled", !supported);
  $("#outline-unavailable").hidden = supported;
  $("#outline-section").querySelectorAll("input").forEach((input) => { input.disabled = !supported; });
  $("#outline-color").disabled = !supported;
}

function openEffects() {
  if (!state.view) return;
  setEffectFields();
  $("#effects-dialog").showModal();
}

function effectInteger(selector, minimum, maximum) {
  const input = $(selector);
  const value = Number(input.value);
  if (!Number.isInteger(value) || value < minimum || value > maximum) {
    input.focus();
    throw new Error(t("commandFailed"));
  }
  return value;
}

async function applyEffects() {
  try {
    const current = normalizeEffects(state.view?.effects);
    const effects = {
      shadow: {
        enabled: $("#shadow-enabled").checked,
        rgb: hexToRgb($("#shadow-color").dataset.color),
        blur: effectInteger("#shadow-blur", 0, 32),
        x: effectInteger("#shadow-x", -32, 32),
        y: effectInteger("#shadow-y", -32, 32),
      },
      outline: state.view?.outline_supported ? {
        enabled: $("#outline-enabled").checked,
        rgb: hexToRgb($("#outline-color").dataset.color),
        width: effectInteger("#outline-width", 1, 16),
      } : current.outline,
    };
    const command = backendCommand("set_effects", { effects });
    $("#effects-dialog").close();
    await applyCommand(command);
  } catch (error) {
    notice(String(error));
  }
}

function openTextEntry(point, tool) {
  textPoint = point;
  const dialog = $("#text-dialog");
  dialog.dataset.kind = tool;
  $("#text-value").value = "";
  renderTextDialogTitle();
  dialog.showModal();
  requestAnimationFrame(() => $("#text-value").focus());
}

async function applyTextEntry() {
  if (!textPoint) return;
  const dialog = $("#text-dialog");
  let command;
  try {
    command = backendCommand("add_text", {
      point: textPoint,
      text: $("#text-value").value,
      callout: dialog.dataset.kind === "Callout",
    });
  } catch (error) {
    notice(String(error));
    $("#text-value").focus();
    return;
  }
  dialog.close();
  textPoint = null;
  await applyCommand(command);
}

async function openImage(file) {
  if (!file) return;
  setBusy(true);
  clearNotice();
  try {
    if (native) {
      const bytes = [...new Uint8Array(await file.arrayBuffer())];
      const opened = await invoke("editor_open", { bytes, name: file.name });
      await reloadWorkspace(opened.session_id);
      loadedSessionId = opened.session_id;
      syncView(opened.view);
      await refreshPreview();
    } else {
      previewImage = await createImageBitmap(file);
      mockObjects = [];
      syncView({ width: previewImage.width, height: previewImage.height, revision: 0, tool: "Select", color: [255, 0, 0], parameter: 5, opacity: 100, pattern: "Solid", selected: false, can_undo: false, can_redo: false, can_delete: false, crop_ready: false });
      notice(t("browserMode"));
    }
    if (!native) $("#document-title").textContent = file.name;
    requestAnimationFrame(fitView);
  } catch (error) {
    notice(`${t("openFailed")} ${String(error)}`);
  } finally {
    setBusy(false);
    renderCanvas();
  }
}

async function refreshPreview() {
  const response = await invoke("editor_preview", commandForSession(workspace.activeSessionId));
  const bytes = response instanceof Uint8Array ? response : new Uint8Array(response);
  if (previewUrl) URL.revokeObjectURL(previewUrl);
  previewUrl = URL.createObjectURL(new Blob([bytes], { type: "image/png" }));
  const image = new Image();
  image.src = previewUrl;
  await image.decode();
  previewImage = image;
  renderCanvas();
}

function canvasBlob(canvas) {
  return new Promise((resolve, reject) => canvas.toBlob((blob) => {
    if (blob) resolve(blob); else reject(new Error("PNG encoding failed."));
  }, "image/png"));
}

async function exportedPng() {
  if (!state.view || !previewImage) throw new Error(t("noImage"));
  if (native) {
    const response = await invoke("editor_preview", commandForSession(workspace.activeSessionId));
    const bytes = response instanceof Uint8Array ? response : new Uint8Array(response);
    return new Blob([bytes], { type: "image/png" });
  }
  const canvas = document.createElement("canvas");
  canvas.width = state.view.width;
  canvas.height = state.view.height;
  const context = canvas.getContext("2d");
  context.drawImage(previewImage, 0, 0, state.view.width, state.view.height);
  for (const object of mockObjects) drawDraft(context, object);
  return canvasBlob(canvas);
}

async function savePng() {
  if (!state.view || busy) return;
  setBusy(true);
  clearNotice();
  try {
    const blob = await exportedPng();
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = pngDownloadName($("#document-title").textContent);
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    notice(t("saved"));
  } catch (error) {
    notice(`${t("saveFailed")} ${String(error)}`);
  } finally {
    setBusy(false);
  }
}

async function copyPng() {
  if (!state.view || busy) return;
  setBusy(true);
  clearNotice();
  try {
    const blob = await exportedPng();
    if (!navigator.clipboard?.write || typeof ClipboardItem === "undefined") throw new Error(t("clipboardUnavailable"));
    await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
    notice(t("copied"));
  } catch (error) {
    notice(`${t("copyFailed")} ${String(error)}`);
  } finally {
    setBusy(false);
  }
}

function fitView() {
  if (!state.view) return;
  const stage = $("#canvas-stage").getBoundingClientRect();
  const margin = 24;
  const zoom = Math.min(1, (stage.width - margin * 2) / state.view.width, (stage.height - margin * 2) / state.view.height);
  state = { ...state, zoom: Math.max(0.05, zoom), pan: { x: (stage.width - state.view.width * zoom) / 2, y: (stage.height - state.view.height * zoom) / 2 } };
  renderControls();
  renderCanvas();
  rememberViewport();
}

function rememberViewport() {
  if (loadedSessionId && state.view) {
    sessionViewports.set(loadedSessionId, { zoom: state.zoom, pan: { ...state.pan } });
  }
}

function restoreViewport(sessionId) {
  const viewport = sessionViewports.get(sessionId);
  if (!viewport) return fitView();
  state = { ...state, zoom: viewport.zoom, pan: { ...viewport.pan } };
  renderControls();
  renderCanvas();
}

function setZoomAtCenter(zoom) {
  if (!state.view) return;
  const stage = $("#canvas-stage").getBoundingClientRect();
  const next = zoomAround(state, { x: stage.width / 2, y: stage.height / 2 }, zoom);
  state = { ...state, ...next };
  rememberViewport();
  renderControls();
  renderCanvas();
}

function renderCanvas() {
  const canvas = $("#editor-canvas");
  const rect = canvas.getBoundingClientRect();
  const ratio = devicePixelRatio || 1;
  const width = Math.max(1, Math.round(rect.width * ratio));
  const height = Math.max(1, Math.round(rect.height * ratio));
  if (canvas.width !== width || canvas.height !== height) { canvas.width = width; canvas.height = height; }
  const context = canvas.getContext("2d");
  context.setTransform(ratio, 0, 0, ratio, 0, 0);
  context.clearRect(0, 0, rect.width, rect.height);
  if (!state.view || !previewImage) return;
  context.save();
  context.translate(state.pan.x, state.pan.y);
  context.scale(state.zoom, state.zoom);
  context.imageSmoothingEnabled = state.zoom < 4;
  context.drawImage(previewImage, 0, 0, state.view.width, state.view.height);
  for (const object of mockObjects) drawDraft(context, object);
  if (draft) drawDraft(context, draft);
  context.restore();
  drawSelectionOverlay(context);
}

function drawSelectionOverlay(context) {
  const overlay = state.view?.selection_overlay;
  if (!overlay) return;
  const screen = selectionOverlayToScreen(overlay, state);
  const forcedColors = matchMedia("(forced-colors: active)").matches;
  const accent = forcedColors ? "#000000" : "#086de5";
  const surface = forcedColors ? "#ffffff" : "#ffffff";
  context.save();
  context.globalAlpha = 1;
  context.lineWidth = 1.5;
  context.strokeStyle = accent;
  context.fillStyle = surface;
  const guide = selectionGuide(screen);
  if (guide) {
    context.setLineDash(guide.dash);
    if (guide.kind === "rectangle") {
      const x = Math.min(guide.start.x, guide.end.x);
      const y = Math.min(guide.start.y, guide.end.y);
      const width = Math.abs(guide.end.x - guide.start.x);
      const height = Math.abs(guide.end.y - guide.start.y);
      context.strokeRect(x, y, width, height);
    } else {
      context.beginPath();
      context.moveTo(guide.start.x, guide.start.y);
      context.quadraticCurveTo(guide.control.x, guide.control.y, guide.end.x, guide.end.y);
      context.stroke();
    }
    context.setLineDash([]);
  }
  for (const handle of screen.handles) {
    const { x, y } = handle.point;
    const radius = handle.size / 2;
    if (["curve", "tail"].includes(handle.role)) {
      context.beginPath();
      context.fillStyle = accent;
      context.arc(x, y, radius, 0, Math.PI * 2);
      context.fill();
      context.strokeStyle = surface;
      context.lineWidth = 2;
      context.stroke();
    } else {
      context.fillStyle = surface;
      context.strokeStyle = accent;
      context.lineWidth = 1.5;
      context.fillRect(x - radius, y - radius, handle.size, handle.size);
      context.strokeRect(x - radius, y - radius, handle.size, handle.size);
    }
  }
  context.restore();
}

function drawDraft(context, item) {
  const points = item.points;
  if (!points?.length) return;
  context.save();
  context.strokeStyle = item.color ?? state.color;
  context.fillStyle = item.color ?? state.color;
  context.lineWidth = item.parameter ?? state.parameter;
  context.globalAlpha = (item.opacity ?? state.opacity) / 100;
  context.lineCap = "round";
  context.lineJoin = "round";
  context.setLineDash(item.pattern === "Dashed" ? [10, 7] : item.pattern === "Dotted" ? [2, 6] : []);
  const [a, b = a] = [points[0], points.at(-1)];
  if (["Pen", "Highlighter"].includes(item.tool)) {
    context.beginPath(); context.moveTo(a.x, a.y); for (const point of points.slice(1)) context.lineTo(point.x, point.y); context.stroke();
  } else if (["Rectangle", "FilledRectangle", "Crop", "Blur", "DuplicateArea"].includes(item.tool)) {
    const x = Math.min(a.x, b.x); const y = Math.min(a.y, b.y); const w = Math.abs(b.x - a.x); const h = Math.abs(b.y - a.y);
    if (item.tool === "FilledRectangle") context.fillRect(x, y, w, h); else { if (["Crop", "DuplicateArea"].includes(item.tool)) context.setLineDash([6, 5]); context.strokeRect(x, y, w, h); }
  } else if (["Ellipse", "FilledEllipse"].includes(item.tool)) {
    context.beginPath(); context.ellipse((a.x + b.x) / 2, (a.y + b.y) / 2, Math.abs(b.x - a.x) / 2, Math.abs(b.y - a.y) / 2, 0, 0, Math.PI * 2); item.tool === "FilledEllipse" ? context.fill() : context.stroke();
  } else if (item.tool === "Step") {
    context.beginPath(); context.arc(a.x, a.y, 18, 0, Math.PI * 2); context.fill(); context.globalAlpha = 1; context.fillStyle = "white"; context.textAlign = "center"; context.textBaseline = "middle"; context.font = "600 16px Segoe UI"; context.fillText("1", a.x, a.y);
  } else if (["Text", "Callout"].includes(item.tool)) {
    const size = Math.max(8, item.parameter ?? 24);
    const lines = String(item.text ?? "").split("\n");
    const font = EDITOR_FONTS.find((candidate) => candidate.id === item.font) ?? EDITOR_FONTS[0];
    context.font = `${size}px ${font.family}`;
    context.textBaseline = "top";
    const width = Math.max(...lines.map((line) => context.measureText(line || " ").width), size) + 20;
    const height = lines.length * size * 1.35 + 16;
    if (item.background?.enabled) {
      context.fillStyle = item.background.color;
      context.fillRect(a.x - 10, a.y - 8, width, height);
      context.fillStyle = item.color ?? state.color;
    }
    if (item.tool === "Callout") {
      context.strokeRect(a.x - 10, a.y - 8, width, height);
      context.beginPath();
      context.moveTo(a.x + 14, a.y + height - 8);
      context.lineTo(a.x + 8, a.y + height + 12);
      context.lineTo(a.x + 32, a.y + height - 8);
      context.stroke();
    }
    context.fillStyle = item.color ?? state.color;
    lines.forEach((line, index) => context.fillText(line, a.x, a.y + index * size * 1.35));
  } else {
    context.beginPath(); context.moveTo(a.x, a.y); context.lineTo(b.x, b.y); context.stroke();
    if (item.tool === "Arrow") { const angle = Math.atan2(b.y - a.y, b.x - a.x); context.beginPath(); context.moveTo(b.x, b.y); context.lineTo(b.x - 14 * Math.cos(angle - .45), b.y - 14 * Math.sin(angle - .45)); context.moveTo(b.x, b.y); context.lineTo(b.x - 14 * Math.cos(angle + .45), b.y - 14 * Math.sin(angle + .45)); context.stroke(); }
  }
  context.restore();
}

function canvasPoint(event) {
  const bounds = $("#editor-canvas").getBoundingClientRect();
  return screenToDocument({ x: event.clientX, y: event.clientY }, bounds, state);
}

function queueMove(point) {
  queuedMove = point;
  if (moveFrame) return;
  moveFrame = requestAnimationFrame(async () => {
    moveFrame = 0;
    const current = queuedMove;
    queuedMove = null;
    if (native && current && pointer?.mode === "draw") await applyPointerCommand(backendCommand("move", { point: current }));
  });
}

function buildHelp() {
  const grid = $("#help-grid");
  grid.replaceChildren();
  const iconNames = { Select: "mouse-pointer-2", Crop: "crop", DuplicateArea: "square-dashed-mouse-pointer", Pen: "pen-line", Highlighter: "highlighter", Line: "minus", Arrow: "move-up-right", Rectangle: "square", Ellipse: "circle", FilledRectangle: "square", FilledEllipse: "circle", Blur: "droplets", Step: "circle", Text: "type", Callout: "message-square", DuplicateObject: "copy" };
  const tools = [...EDITOR_TOOL_GROUPS.flatMap((group) => group.tools), "DuplicateObject"];
  for (const tool of tools) {
    const textKey = toolTextKey[tool];
    const item = document.createElement("div");
    item.className = "help-item";
    item.innerHTML = `<i data-lucide="${iconNames[tool]}"></i><div><strong></strong><span></span></div>`;
    item.querySelector("strong").textContent = t(textKey);
    const helpKey = `${textKey}Help`;
    item.querySelector("span").textContent = t(helpKey) === helpKey ? t(textKey) : t(helpKey);
    grid.append(item);
  }
  refreshIcons(grid);
}

document.addEventListener("click", (event) => {
  const button = event.target.closest("button");
  if (!button || button.disabled) {
    if (!event.target.closest("#editor-popup")) closeMenu();
    return;
  }
  if (button.dataset.colorTarget) return showPalette(button);
  if (button.dataset.menu) return showMenu(button);
  if (button.dataset.tool) return void chooseTool(button.dataset.tool);
  if (button.dataset.command) return void applyCommand(backendCommand(button.dataset.command));
  if (button.dataset.closeDialog) document.getElementById(button.dataset.closeDialog).close();
  if (!event.target.closest("#editor-popup")) closeMenu();
});

$("#open-image").addEventListener("change", (event) => { void openImage(event.target.files[0]); event.target.value = ""; });
$("#empty-open").addEventListener("click", () => $("#open-image").click());
$(".new-tab").addEventListener("click", () => $("#open-image").click());
$("#editor-language").addEventListener("click", () => {
  state = { ...state, language: state.language === "en" ? "th" : "en" };
  try { const saved = JSON.parse(localStorage.getItem(storageKey) || "{}"); localStorage.setItem(storageKey, JSON.stringify({ ...saved, language: state.language })); } catch { /* Optional storage. */ }
  dismissTooltip(); renderLanguage(); renderControls();
});
$("#help-button").addEventListener("click", () => $("#help-dialog").showModal());
$("#ocr-button").addEventListener("click", () => notice(t("notAvailable")));
$("#effects-button").addEventListener("click", openEffects);
$("#effects-apply").addEventListener("click", () => void applyEffects());
$("#text-apply").addEventListener("click", () => void applyTextEntry());
$("#text-dialog").addEventListener("close", () => { textPoint = null; });
$("#copy-button").addEventListener("click", () => void copyPng());
$("#save-png").addEventListener("click", () => void savePng());
$("#notice-close").addEventListener("click", clearNotice);
$("#fit-view").addEventListener("click", fitView);
$("#zoom-out").addEventListener("click", () => setZoomAtCenter(steppedZoom(state.zoom, -1)));
$("#zoom-in").addEventListener("click", () => setZoomAtCenter(steppedZoom(state.zoom, 1)));
$("#tool-size").addEventListener("change", (event) => { const value = Math.min(40, Math.max(1, Number(event.target.value) || 5)); state = { ...state, parameter: value }; void applyCommand(backendCommand("set_parameter", { value }), false); });
$("#text-font").addEventListener("change", (event) => {
  state = { ...state, font: event.target.value };
  renderControls();
  void applyCommand(backendCommand("set_font", { font: event.target.value }));
});
$("#text-background-enabled").addEventListener("change", (event) => {
  state = { ...state, textBackground: { ...state.textBackground, enabled: event.target.checked } };
  renderControls();
  void applyCommand(backendCommand("set_text_background", {
    background: { enabled: event.target.checked, rgb: hexToRgb(state.textBackground.color) },
  }));
});
$("#custom-color-input").addEventListener("change", (event) => {
  const action = customColorAction;
  customColorAction = null;
  closeMenu();
  action?.(event.target.value.toLowerCase());
});
$("#tool-opacity").addEventListener("input", (event) => { state = { ...state, opacity: Number(event.target.value) }; renderControls(); });
$("#tool-opacity").addEventListener("change", (event) => void applyCommand(backendCommand("set_opacity", { value: Number(event.target.value) }), false));
$("#confirm-crop").addEventListener("click", () => void applyCommand(backendCommand("confirm_crop")));

$("#editor-canvas").addEventListener("pointerdown", (event) => {
  if (!state.view || busy) return;
  if (event.button === 2 || ![0, 1].includes(event.button)) return;
  if (["Text", "Callout"].includes(state.activeTool) && event.button === 0 && !spaceDown) {
    openTextEntry(canvasPoint(event), state.activeTool);
    return;
  }
  const pan = event.button === 1 || spaceDown;
  pointer = { id: event.pointerId, mode: pan ? "pan" : "draw", last: { x: event.clientX, y: event.clientY } };
  event.currentTarget.setPointerCapture(event.pointerId);
  if (!pan) {
    const point = canvasPoint(event);
    draft = state.activeTool === "Select" ? null : { tool: state.activeTool, points: [point], color: state.color, parameter: state.parameter, opacity: state.activeTool === "Highlighter" ? 40 : state.opacity, pattern: state.pattern };
    if (state.activeTool !== "DuplicateArea") void applyPointerCommand(backendCommand("press", { point, tolerance: 5 / state.zoom }));
  }
});
$("#editor-canvas").addEventListener("contextmenu", async (event) => {
  event.preventDefault();
  if (!state.view || busy) return closeMenu();
  const anchor = { x: event.clientX, y: event.clientY };
  if (native) {
    if (state.activeTool !== "Select") await applyCommand(backendCommand("choose", { tool: "Select" }), false);
    const point = canvasPoint(event);
    await applyPointerCommand(backendCommand("press", { point, tolerance: 5 / state.zoom }));
    await applyPointerCommand(backendCommand("release", { point }), true);
  }
  showContextMenu(anchor, state.view?.selected === true);
});
$("#editor-canvas").addEventListener("dblclick", (event) => {
  if (!state.view || busy || event.button !== 0) return;
  const bounds = event.currentTarget.getBoundingClientRect();
  const pointerPosition = { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
  if (selectionHandleAt(state.view.selection_overlay, pointerPosition, state)?.role !== "curve") return;
  event.preventDefault();
  void applyCommand(backendCommand("reset_curve"));
});
$("#editor-canvas").addEventListener("pointermove", (event) => {
  if (!pointer || pointer.id !== event.pointerId) return;
  if (pointer.mode === "pan") {
    state = { ...state, pan: { x: state.pan.x + event.clientX - pointer.last.x, y: state.pan.y + event.clientY - pointer.last.y } };
    pointer.last = { x: event.clientX, y: event.clientY };
  } else {
    const point = canvasPoint(event);
    if (draft) {
      if (["Pen", "Highlighter"].includes(draft.tool)) draft.points.push(point); else draft.points[1] = point;
    }
    if (draft?.tool !== "DuplicateArea") queueMove(point);
  }
  renderCanvas();
});
$("#editor-canvas").addEventListener("pointerup", async (event) => {
  if (!pointer || pointer.id !== event.pointerId) return;
  event.currentTarget.releasePointerCapture(event.pointerId);
  const drawing = pointer.mode === "draw";
  pointer = null;
  if (drawing) {
    const point = canvasPoint(event);
    const duplicateStart = draft?.tool === "DuplicateArea" ? draft.points[0] : null;
    if (!native && draft && draft.tool !== "DuplicateArea" && draft.points.length > 1) { mockObjects.push(draft); if (state.view) state.view = { ...state.view, can_undo: true, can_delete: true }; }
    draft = null;
    if (duplicateStart) await applyCommand(backendCommand("duplicate_area", { start: duplicateStart, end: point }));
    else await applyPointerCommand(backendCommand("release", { point }), true);
  } else rememberViewport();
  renderCanvas();
});
$("#editor-canvas").addEventListener("wheel", (event) => {
  if (!state.view || !event.ctrlKey) return;
  event.preventDefault();
  const bounds = event.currentTarget.getBoundingClientRect();
  const pointerPosition = { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
  const next = zoomAround(state, pointerPosition, state.zoom * (event.deltaY < 0 ? 1.12 : .89));
  state = { ...state, ...next }; rememberViewport(); renderControls(); renderCanvas();
}, { passive: false });

document.addEventListener("keydown", (event) => {
  const editing = event.target instanceof Element && Boolean(event.target.closest("input, textarea, select, [contenteditable='true']"));
  if (menuOwner && ["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
    const items = [...$("#editor-popup").querySelectorAll("[role='menuitem']:not(:disabled)")];
    const current = Math.max(0, items.indexOf(document.activeElement));
    const next = moveMenuFocus(current, event.key, items.length);
    if (next >= 0) items[next].focus();
    event.preventDefault();
    return;
  }
  if (isContextMenuKey(event) && !editing && state.view) {
    const bounds = $("#editor-canvas").getBoundingClientRect();
    showContextMenu({ x: bounds.left + bounds.width / 2, y: bounds.top + bounds.height / 2 }, state.view.selected === true);
    event.preventDefault();
    return;
  }
  if (event.code === "Space" && !event.repeat) { spaceDown = true; event.preventDefault(); }
  if (event.key === "Escape") { if (menuOwner) closeMenu(true); else if (document.querySelector("dialog[open]")) document.querySelector("dialog[open]").close(); else { pointer = null; draft = null; void applyCommand(backendCommand("cancel"), false); renderCanvas(); } }
  if (event.key === "Delete") void applyCommand(backendCommand("delete"));
  if (event.ctrlKey && event.key.toLowerCase() === "z") { event.preventDefault(); void applyCommand(backendCommand(event.shiftKey ? "redo" : "undo")); }
  if (event.ctrlKey && event.key.toLowerCase() === "y") { event.preventDefault(); void applyCommand(backendCommand("redo")); }
  if (event.ctrlKey && event.key.toLowerCase() === "o") { event.preventDefault(); $("#open-image").click(); }
  if (event.ctrlKey && ["+", "="].includes(event.key)) { event.preventDefault(); setZoomAtCenter(steppedZoom(state.zoom, 1)); }
  if (event.ctrlKey && event.key === "-") { event.preventDefault(); setZoomAtCenter(steppedZoom(state.zoom, -1)); }
  if (event.ctrlKey && event.key === "0") { event.preventDefault(); fitView(); }
  if (shouldDuplicateShortcut({ key: event.key, ctrlKey: event.ctrlKey, repeat: event.repeat, editing }, state.view?.selected)) { event.preventDefault(); void applyCommand(backendCommand("duplicate_selection")); }
  if (event.key === "Enter" && state.view?.crop_ready) void applyCommand(backendCommand("confirm_crop"));
});
document.addEventListener("keyup", (event) => { if (event.code === "Space") spaceDown = false; });

function scheduleTooltip(event) {
  dismissTooltip();
  const target = event.target.closest("[data-tooltip]");
  if (!target || menuOwner || busy) return;
  tooltipTimer = setTimeout(() => {
    const tooltip = $("#editor-tooltip");
    tooltip.textContent = t(target.dataset.tooltip);
    tooltipOwner = target;
    target.setAttribute("aria-describedby", "editor-tooltip");
    placeFloating(tooltip, target);
  }, 550);
}
document.addEventListener("pointerover", scheduleTooltip);
document.addEventListener("pointerout", dismissTooltip);
document.addEventListener("focusin", scheduleTooltip);
document.addEventListener("focusout", dismissTooltip);
window.addEventListener("resize", renderCanvas);
window.addEventListener("blur", () => { spaceDown = false; closeMenu(); dismissTooltip(); });
window.addEventListener("beforeunload", () => {
  if (previewUrl) URL.revokeObjectURL(previewUrl);
  if (typeof stopWorkspaceListener === "function") stopWorkspaceListener();
});

for (const font of EDITOR_FONTS) {
  const option = document.createElement("option");
  option.value = font.id;
  option.textContent = font.label;
  option.style.fontFamily = font.family;
  $("#text-font").append(option);
}
refreshIcons();
$("#editor-logo").src = logo;
renderLanguage();
renderControls();
renderCanvas();
if (native) {
  void (async () => {
    try {
      await invoke("editor_window_mode");
      if (window.__TAURI__?.event?.listen) {
        stopWorkspaceListener = await window.__TAURI__.event.listen(
          "editor-sessions-changed",
          () => void refreshWorkspaceEvent(),
        );
      }
      await bootstrapWorkspace();
    } catch (error) {
      notice(String(error));
    }
  })();
}
