import test from "node:test";
import assert from "node:assert/strict";
import { combinedBounds, selectedSessionIds, moveCandidate } from "../ui/combine-workflow.js";

const candidates = [
  { session_id: "a", width: 100, height: 80, checked: true },
  { session_id: "b", width: 60, height: 40, checked: true },
  { session_id: "c", width: 30, height: 20, checked: false },
];

test("combine request preserves checkbox order and vertical dimensions", () => {
  assert.deepEqual(selectedSessionIds(candidates), ["a", "b"]);
  assert.deepEqual(combinedBounds(candidates, "vertical", 16), { width: 100, height: 136 });
  assert.deepEqual(combinedBounds(candidates, "horizontal", 16), { width: 176, height: 80 });
  assert.deepEqual(combinedBounds(candidates, "free", 16), { width: 100, height: 80 });
});

test("reordering candidates changes source order without changing the originals", () => {
  const reordered = moveCandidate(candidates, "b", -1);
  assert.deepEqual(selectedSessionIds(reordered), ["b", "a"]);
  assert.deepEqual(selectedSessionIds(candidates), ["a", "b"]);
});

test("combine validation requires two distinct checked images and a bounded gap", () => {
  assert.throws(() => combinedBounds(candidates.slice(0, 1), "vertical", 16), /2-16/);
  assert.throws(() => combinedBounds(candidates, "vertical", 501), /gap/i);
  assert.throws(() => selectedSessionIds([candidates[0], candidates[0]]), /different/i);
});
