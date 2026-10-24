const actions = new Set(["area", "window", "desktop", "scrolling", "video_monitor", "video_window", "open", "hotkeys"]);

export function normalize(input) {
  const value = input && typeof input === "object" ? input : {};
  return {
    language: value.language === "th" ? "th" : "en",
    delay: [0, 3, 5, 10].includes(value.delay) ? value.delay : 0,
    scroll: value.scroll === "manual" ? "manual" : "automatic",
    destination: value.destination === "window" ? "window" : "tab",
    audio: ["none", "system", "microphone", "both"].includes(value.audio) ? value.audio : "none",
  };
}

export function requestFor(state, action) {
  if (!actions.has(action)) throw new Error("Unsupported desktop command.");
  return { action, preferences: {
    language: state.language,
    delay_seconds: state.delay,
    scroll_automatic: state.scroll === "automatic",
    open_in_tab: state.destination === "tab",
    video_audio: state.audio,
  } };
}

export function afterAction(state, action, success) {
  return { ...state, delay: success && ["area", "window", "desktop", "scrolling"].includes(action) ? 0 : state.delay };
}
