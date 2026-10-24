import test from "node:test";
import assert from "node:assert/strict";
import { normalize, requestFor, afterAction } from "../ui/state.js";

test("untrusted stored preferences return to valid defaults", () => {
  assert.deepEqual(normalize({ language: "xx", delay: 999, scroll: "bad", destination: "bad", audio: "bad" }), {
    language: "en", delay: 0, scroll: "automatic", destination: "tab", audio: "none",
  });
  assert.equal(normalize(null).language, "en");
});

test("a successful image capture consumes delay, video and failed actions retain it", () => {
  const state = normalize({ language: "th", delay: 5, destination: "window" });
  assert.equal(afterAction(state, "scrolling", true).delay, 0);
  for (const action of ["video_monitor", "open", "hotkeys"]) {
    assert.equal(afterAction(state, action, true).delay, 5);
  }
  assert.equal(afterAction(state, "area", false).delay, 5);
});

test("only explicit preferences cross the bridge", () => {
  const request = requestFor(normalize({ language: "th", delay: 10, scroll: "manual", destination: "window", audio: "system" }), "area");
  assert.deepEqual(request, { action: "area", preferences: {
    language: "th", delay_seconds: 10, scroll_automatic: false, open_in_tab: false, video_audio: "system",
  } });
  assert.throws(() => requestFor(normalize({}), "execute"));
});
