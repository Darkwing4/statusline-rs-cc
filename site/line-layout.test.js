import assert from "node:assert/strict";
import test from "node:test";

import {
  describeLineFill,
  displayWidth,
  layoutRows,
  nearestDropSlot,
  runtimePreviewColumns,
  wrapPreviewSegments
} from "./line-layout.js";

function pieces(text) {
  return [{ text, color: "#ffffff" }];
}

function entry(text, standalone = false) {
  return { pieces: pieces(text), standalone };
}

test("standalone pieces break the line where they sit", () => {
  const rows = layoutRows(
    [entry("one"), entry("below", true), entry("two"), entry("three"), entry("also below", true)],
    " ",
    80
  );
  assert.deepEqual(rows.map((row) => row.standalone), [false, true, false, true]);
  assert.deepEqual(rows[0].cells.map((cell) => cell.entry.pieces[0].text), ["one"]);
  assert.equal(rows[1].cells[0].entry.pieces[0].text, "below");
  assert.deepEqual(rows[2].cells.map((cell) => cell.entry.pieces[0].text), ["two", "three"]);
  assert.equal(layoutRows([entry("below", true)], " ", 80)[0].cells.length, 0);
  assert.equal(layoutRows([entry("below", true), entry("after", false)], " ", 80).length, 3);
  assert.equal(layoutRows([entry("one"), entry("below", true), entry("also", true)], " ", 80).length, 3);
});

test("preview wrapping uses cols minus four and never leads with a separator", () => {
  const lines = wrapPreviewSegments([pieces("one"), pieces("two")], " | ", runtimePreviewColumns(12));
  assert.equal(runtimePreviewColumns(12), 8);
  assert.equal(lines.length, 2);
  assert.deepEqual(lines.map((line) => line.map((cell) => cell.separator)), [[false], [false]]);
  assert.deepEqual(
    wrapPreviewSegments([pieces("one"), pieces("two")], " | ", runtimePreviewColumns(13))
      .map((line) => line.map((cell) => cell.separator)),
    [[false, true]]
  );
});

test("preview width handles terminal graphemes", () => {
  assert.equal(displayWidth("a界🙂"), 5);
  assert.equal(displayWidth("👩‍💻"), 2);
  assert.equal(displayWidth("🇺🇸"), 2);
  assert.equal(displayWidth("1️⃣"), 2);
  assert.equal(displayWidth("é"), 1);
  assert.equal(displayWidth("⁠"), 0);
});

test("line fill reports the widest row and the row count", () => {
  assert.equal(describeLineFill([], " ", 92), "0 of 92 cols, 0 rows");
  assert.equal(describeLineFill([entry("one"), entry("two")], " | ", 92), "9 of 92 cols, 1 row");
  assert.equal(describeLineFill([entry("one"), entry("two")], " | ", 8), "3 of 8 cols, 2 rows");
  assert.equal(describeLineFill([entry("one"), entry("standalone", true)], " ", 92), "10 of 92 cols, 2 rows");
});

test("flag and keycap widths preserve wrapping boundaries", () => {
  assert.equal(wrapPreviewSegments([pieces("a"), pieces("🇺🇸")], " ", 4).length, 1);
  assert.equal(wrapPreviewSegments([pieces("a"), pieces("🇺🇸")], " ", 3).length, 2);
  assert.equal(wrapPreviewSegments([pieces("a"), pieces("1️⃣")], " ", 4).length, 1);
  assert.equal(wrapPreviewSegments([pieces("a"), pieces("1️⃣")], " ", 3).length, 2);
});

test("a drop lands on the nearest slot of the row under the cursor", () => {
  const box = (id, left, right, top, bottom) => ({ node: id, rect: { left, right, top, bottom } });
  const boxes = [box("a", 0, 40, 0, 20), box("b", 50, 90, 0, 20), box("c", 0, 40, 30, 50)];

  assert.deepEqual(nearestDropSlot(boxes, 48, 10), { node: "b", before: true });
  assert.deepEqual(nearestDropSlot(boxes, 45, 27), { node: "c", before: false });
  assert.deepEqual(nearestDropSlot(boxes, 200, 10), { node: "b", before: false });
  assert.deepEqual(nearestDropSlot(boxes, 200, 200), { node: "c", before: false });
  assert.equal(nearestDropSlot([], 10, 10), null);
});
