import "./style.css";
import logo from "../../../assets/snapzy-rust.png";
import { createIcons, Scan, AppWindow, ScanLine, Video, ImagePlus, Ellipsis, ChevronDown, Timer, ListFilter, PanelsTopLeft, Monitor, Keyboard, Info, X, Check, Circle } from "lucide";
import { normalize, requestFor, afterAction } from "./state.js";
import { translate } from "./i18n.js";

const icons = { Scan, AppWindow, ScanLine, Video, ImagePlus, Ellipsis, ChevronDown, Timer, ListFilter, PanelsTopLeft, Monitor, Keyboard, Info, X, Check, Circle };
const $ = (selector) => document.querySelector(selector);
const storageKey = "snapzy-tauri-trial-preferences-v1";
let state;
try { state = normalize(JSON.parse(localStorage.getItem(storageKey))); } catch { state = normalize({}); }
let working = false;
let source = "video_monitor";
let menuOwner = null;
let tooltipTimer;
let tooltipOwner;
const t = (key) => translate(state.language, key);
const native = Boolean(window.__TAURI__?.core?.invoke);
if (!native) document.documentElement.classList.add("browser");
$("#logo").src = logo;
$("#about-logo").src = logo;

function refreshIcons() {
  createIcons({ icons, attrs: { "stroke-width": 1.65, "aria-hidden": "true" } });
}

function save() {
  try { localStorage.setItem(storageKey, JSON.stringify(state)); } catch { /* Storage is optional. */ }
}

function render() {
  document.documentElement.lang = state.language;
  document.querySelectorAll("[data-text]").forEach((element) => { element.textContent = t(element.dataset.text); });
  document.querySelectorAll("[data-tooltip]").forEach((element) => { element.setAttribute("aria-label", t(element.dataset.tooltip)); });
  $("#language").textContent = state.language.toUpperCase();
  $("#status").textContent = t(working ? "working" : "ready");
  $("#delay-value").textContent = state.delay ? t("seconds").replace("{n}", state.delay) : t("none");
  $("[data-menu='delay']").classList.toggle("active", state.delay > 0);
  $("#scroll-value").textContent = t(state.scroll);
  $("#destination-value").textContent = t(state.destination === "tab" ? "tab" : "newWindow");
  $("#audio").value = state.audio;
  document.body.classList.toggle("busy", working);
  document.querySelectorAll("main button").forEach((button) => { button.disabled = working; });
}

function notice(message) {
  $("#notice-text").textContent = message;
  $("#notice").hidden = false;
}

function dismissTooltip() {
  clearTimeout(tooltipTimer);
  $("#tooltip").hidden = true;
  tooltipOwner?.removeAttribute("aria-describedby");
  tooltipOwner = null;
}

function placePopup(element, anchor, gap = 6) {
  const bounds = anchor.getBoundingClientRect();
  element.hidden = false;
  const size = element.getBoundingClientRect();
  const left = Math.min(Math.max(8, bounds.left), Math.max(8, innerWidth - size.width - 8));
  const below = bounds.bottom + gap;
  const top = below + size.height <= innerHeight - 8 ? below : Math.max(8, bounds.top - size.height - gap);
  element.style.left = `${left}px`;
  element.style.top = `${top}px`;
}

function closeMenu(restoreFocus = false) {
  $("#popup").hidden = true;
  if (menuOwner) {
    menuOwner.setAttribute("aria-expanded", "false");
    if (restoreFocus) menuOwner.focus();
  }
  menuOwner = null;
}

function item(label, icon, action, selected) {
  const button = document.createElement("button");
  button.className = `menu-item${selected ? " selected" : ""}`;
  button.type = "button";
  button.setAttribute("role", selected === undefined ? "menuitem" : "menuitemradio");
  if (selected !== undefined) button.setAttribute("aria-checked", String(selected));
  const glyph = document.createElement("i");
  glyph.dataset.lucide = icon;
  const text = document.createElement("span");
  text.className = "menu-label";
  text.textContent = label;
  button.append(glyph, text);
  if (selected) {
    const check = document.createElement("i");
    check.dataset.lucide = "check";
    button.append(check);
  }
  button.addEventListener("click", () => { closeMenu(true); action(); });
  return button;
}

function showMenu(owner) {
  if (menuOwner === owner) return closeMenu(true);
  closeMenu();
  dismissTooltip();
  const popup = $("#popup");
  popup.replaceChildren();
  const choose = (key, value) => { state = normalize({ ...state, [key]: value }); save(); render(); };
  const kind = owner.dataset.menu;
  if (kind === "delay") {
    for (const delay of [0, 3, 5, 10]) popup.append(item(delay ? t("seconds").replace("{n}", delay) : t("none"), "timer", () => choose("delay", delay), state.delay === delay));
  } else if (kind === "scroll") {
    for (const mode of ["automatic", "manual"]) popup.append(item(t(mode), "list-filter", () => choose("scroll", mode), state.scroll === mode));
  } else if (kind === "destination") {
    for (const destination of ["tab", "window"]) popup.append(item(t(destination === "tab" ? "tab" : "newWindow"), "panels-top-left", () => choose("destination", destination), state.destination === destination));
  } else if (kind === "window") {
    popup.append(item(t("window"), "app-window", () => perform("window")), item(t("fullscreen"), "monitor", () => perform("desktop")));
  } else if (kind === "more") {
    popup.append(item(t("fullscreen"), "monitor", () => perform("desktop")), item(t("hotkeys"), "keyboard", () => perform("hotkeys")), item(t("about"), "info", () => $("#about-dialog").showModal()));
  }
  menuOwner = owner;
  owner.setAttribute("aria-expanded", "true");
  refreshIcons();
  placePopup(popup, owner);
  popup.querySelector("button")?.focus();
}

async function perform(action) {
  if (working) return;
  closeMenu();
  dismissTooltip();
  $("#notice").hidden = true;
  if (!native) { notice(t("desktopOnly")); return; }
  working = true;
  render();
  try {
    if (action === "open") await window.__TAURI__.core.invoke("editor_open_image_window");
    else await window.__TAURI__.core.invoke("desktop_action", { request: requestFor(state, action) });
    state = afterAction(state, action, true);
    save();
  } catch (error) {
    notice(`${t("failed")} ${String(error)}`);
  } finally {
    working = false;
    render();
  }
}

document.addEventListener("click", (event) => {
  const button = event.target.closest("button");
  if (button?.disabled) return;
  if (button?.dataset.menu) return showMenu(button);
  if (button?.dataset.action) return void perform(button.dataset.action);
  if (button?.dataset.close) document.getElementById(button.dataset.close).close();
  if (!event.target.closest("#popup")) closeMenu();
});

$("#language").addEventListener("click", () => {
  state = { ...state, language: state.language === "en" ? "th" : "en" };
  dismissTooltip(); save(); render();
});
$("#dismiss").addEventListener("click", () => { $("#notice").hidden = true; });
$("#video").addEventListener("click", () => { closeMenu(); dismissTooltip(); $("#video-dialog").showModal(); });
document.querySelectorAll("[data-source]").forEach((button) => {
  button.addEventListener("click", () => {
    source = button.dataset.source;
    document.querySelectorAll("[data-source]").forEach((option) => {
      const selected = source === option.dataset.source;
      option.classList.toggle("selected", selected);
      option.setAttribute("aria-pressed", String(selected));
    });
  });
});
$("#video-form").addEventListener("submit", (event) => {
  event.preventDefault();
  state = normalize({ ...state, audio: $("#audio").value });
  save();
  $("#video-dialog").close();
  void perform(source);
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") { closeMenu(true); dismissTooltip(); $("#notice").hidden = true; }
  if (menuOwner && ["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
    event.preventDefault();
    const buttons = [...$("#popup").querySelectorAll("button")];
    let index = buttons.indexOf(document.activeElement);
    if (event.key === "Home") index = 0;
    else if (event.key === "End") index = buttons.length - 1;
    else index = (index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
    buttons[index]?.focus();
  }
  if (event.key === "Tab" && menuOwner) closeMenu();
});

function scheduleTooltip(event) {
  dismissTooltip();
  const target = event.target.closest("[data-tooltip]");
  if (!target || menuOwner || working) return;
  tooltipTimer = setTimeout(() => {
    const tooltip = $("#tooltip");
    tooltip.textContent = t(target.dataset.tooltip);
    tooltipOwner = target;
    target.setAttribute("aria-describedby", "tooltip");
    placePopup(tooltip, target);
  }, 550);
}
document.addEventListener("pointerover", scheduleTooltip);
document.addEventListener("pointerout", dismissTooltip);
document.addEventListener("focusin", scheduleTooltip);
document.addEventListener("focusout", dismissTooltip);
document.addEventListener("pointerdown", dismissTooltip);
window.addEventListener("blur", () => { closeMenu(); dismissTooltip(); });

refreshIcons();
render();
if (native && window.__TAURI__?.event?.listen) {
  void window.__TAURI__.event.listen("capture-result-error", (event) => notice(String(event.payload)));
}
