import test from "node:test";
import assert from "node:assert/strict";
import {
  commandForSession,
  normalizeWorkspace,
  removeWorkspaceTab,
  selectWorkspaceTab,
} from "../ui/editor-workspace.js";

const one = "0123456789abcdef0123456789abcdef";
const two = "fedcba9876543210fedcba9876543210";

test("workspace accepts only bounded backend sessions and the assigned active tab", () => {
  const workspace = normalizeWorkspace({
    tabs: [
      { session_id: one, title: "Area capture" },
      { session_id: two, title: "Window capture" },
      { session_id: "../invalid", title: "Private" },
    ],
    active: two,
  });
  assert.deepEqual(workspace, {
    tabs: [
      { sessionId: one, title: "Area capture" },
      { sessionId: two, title: "Window capture" },
    ],
    activeSessionId: two,
  });
});

test("tab selection and closing stay inside the current editor host", () => {
  let workspace = normalizeWorkspace({
    tabs: [{ session_id: one, title: "One" }, { session_id: two, title: "Two" }],
    active: one,
  });
  workspace = selectWorkspaceTab(workspace, two);
  assert.equal(workspace.activeSessionId, two);
  assert.throws(() => selectWorkspaceTab(workspace, "0".repeat(32)), /session/i);
  workspace = removeWorkspaceTab(workspace, two);
  assert.equal(workspace.activeSessionId, one);
  assert.deepEqual(workspace.tabs.map((tab) => tab.sessionId), [one]);
});

test("native editor commands always carry the active session identity", () => {
  assert.deepEqual(commandForSession(one, { command: { type: "undo" } }), {
    sessionId: one,
    command: { type: "undo" },
  });
  assert.throws(() => commandForSession(null, {}), /session/i);
});
