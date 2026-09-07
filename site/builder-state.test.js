import assert from "node:assert/strict";
import test from "node:test";

import { catalog, modules } from "./catalog-from-disk.js";
import { createSegment, defaultLine, isStandalone, resetState, shelfModules, state } from "./builder-state.js";

test("every catalogue segment is on the shelf with its pitch", () => {
  resetState();
  state.segments = [];
  const shelf = shelfModules();
  catalog.segments.forEach((segment) => {
    const entries = shelf.filter((module) => module.type === segment.name);
    assert.ok(entries.length > 0, segment.name);
    entries.forEach((module) => assert.equal(module.description, segment.pitch));
  });
});

test("the default line only uses pieces the catalogue knows", () => {
  defaultLine.forEach((entry) => {
    const id = typeof entry === "string" ? entry : entry.module;
    assert.ok(modules.some((module) => module.id === id), id);
    if (typeof entry !== "string") {
      const fields = new Set(modules.find((module) => module.id === id).fields.map((field) => field.name));
      Object.keys(entry.config).forEach((name) => assert.ok(fields.has(name), `${id}.${name}`));
    }
  });
  resetState();
  const insights = state.segments.filter((segment) => segment.type === "LlmInsight");
  assert.equal(insights.length, 2);
  assert.notEqual(insights[0].config.prompt, insights[1].config.prompt);
  assert.ok(insights.every((segment) => segment.config.standalone));
});

test("standalone pieces come from the config or from the segment itself", () => {
  assert.ok(isStandalone(createSegment("LlmAnswer")));
  assert.ok(isStandalone(createSegment("SessionNotice")));
  assert.ok(!isStandalone(createSegment("Reminder")));
  assert.ok(isStandalone(createSegment("Spacer")));
});
