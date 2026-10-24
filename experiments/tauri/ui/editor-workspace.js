const sessionPattern = /^[0-9a-f]{32}$/;

function validSessionId(value) {
  return typeof value === "string" && sessionPattern.test(value);
}

export function normalizeWorkspace(input) {
  const source = input && typeof input === "object" ? input : {};
  const seen = new Set();
  const tabs = Array.isArray(source.tabs) ? source.tabs.flatMap((tab) => {
    if (!tab || !validSessionId(tab.session_id) || seen.has(tab.session_id)) return [];
    seen.add(tab.session_id);
    const title = typeof tab.title === "string" && tab.title.trim()
      ? tab.title.replace(/[\u0000-\u001f\u007f]/g, "").slice(0, 120)
      : "Untitled";
    return [{ sessionId: tab.session_id, title }];
  }) : [];
  const requested = validSessionId(source.active) ? source.active : null;
  return {
    tabs,
    activeSessionId: tabs.some((tab) => tab.sessionId === requested)
      ? requested
      : (tabs.at(-1)?.sessionId ?? null),
  };
}

export function selectWorkspaceTab(workspace, sessionId) {
  if (!validSessionId(sessionId) || !workspace.tabs.some((tab) => tab.sessionId === sessionId)) {
    throw new Error("The editor session is unavailable.");
  }
  return { ...workspace, activeSessionId: sessionId };
}

export function removeWorkspaceTab(workspace, sessionId) {
  const index = workspace.tabs.findIndex((tab) => tab.sessionId === sessionId);
  if (index < 0) return workspace;
  const tabs = workspace.tabs.filter((tab) => tab.sessionId !== sessionId);
  const activeSessionId = workspace.activeSessionId === sessionId
    ? (tabs[index]?.sessionId ?? tabs.at(-1)?.sessionId ?? null)
    : workspace.activeSessionId;
  return { tabs, activeSessionId };
}

export function commandForSession(sessionId, fields = {}) {
  if (!validSessionId(sessionId)) throw new Error("The editor session is unavailable.");
  return { sessionId, ...fields };
}
