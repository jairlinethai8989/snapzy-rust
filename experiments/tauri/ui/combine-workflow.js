export function selectedSessionIds(candidates) {
  const ids = candidates.filter((candidate) => candidate.checked).map((candidate) => candidate.session_id);
  if (new Set(ids).size !== ids.length) throw new Error("Choose different image tabs.");
  return ids;
}

export function moveCandidate(candidates, sessionId, direction) {
  const result = candidates.map((candidate) => ({ ...candidate }));
  const index = result.findIndex((candidate) => candidate.session_id === sessionId);
  const next = index + direction;
  if (index < 0 || next < 0 || next >= result.length) return result;
  [result[index], result[next]] = [result[next], result[index]];
  return result;
}

export function combinedBounds(candidates, layout, gap) {
  const selected = candidates.filter((candidate) => candidate.checked);
  selectedSessionIds(candidates);
  if (selected.length < 2 || selected.length > 16) throw new Error("Choose 2-16 images.");
  if (!Number.isInteger(gap) || gap < 0 || gap > 500) throw new Error("Choose a gap from 0 to 500 px.");
  if (!["vertical", "horizontal", "free"].includes(layout)) throw new Error("Choose a layout.");
  let x = 0;
  let y = 0;
  let width = 0;
  let height = 0;
  for (const [index, candidate] of selected.entries()) {
    if (!Number.isInteger(candidate.width) || !Number.isInteger(candidate.height) || candidate.width < 1 || candidate.height < 1) {
      throw new Error("An image has invalid dimensions.");
    }
    if (layout === "free") x = y = index * 24;
    width = Math.max(width, x + candidate.width);
    height = Math.max(height, y + candidate.height);
    if (layout === "vertical") y += candidate.height + gap;
    if (layout === "horizontal") x += candidate.width + gap;
  }
  return { width, height };
}
