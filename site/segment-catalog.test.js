import assert from "node:assert/strict";
import test from "node:test";

import { catalog, modules } from "./catalog-from-disk.js";
import { segmentLabel } from "./segment-catalog.js";
import { createSegment, presets } from "./builder-state.js";

test("every rate limit window gets its own shelf piece", () => {
  const windows = catalog.segments
    .find((segment) => segment.name === "RateLimit")
    .fields.find((field) => field.name === "window").variants;
  windows.forEach((window) => {
    const module = modules.find((candidate) => candidate.id === `RateLimit:${window}`);
    assert.ok(module, window);
    assert.equal(createSegment(module.id).config.window, window);
  });
});

test("presets only name fields the catalogue declares", () => {
  Object.entries(presets).forEach(([type, preset]) => {
    const segment = catalog.segments.find((candidate) => candidate.name === type);
    assert.ok(segment, type);
    Object.keys(preset).forEach((key) => {
      assert.ok(segment.fields.some((field) => field.name === key), `${type}.${key}`);
    });
  });
});

test("segment names become readable labels", () => {
  assert.equal(segmentLabel("PromptCacheTtl"), "Prompt cache TTL");
  assert.equal(segmentLabel("LlmInsight"), "LLM insight");
  assert.equal(segmentLabel("MyLastPrompt"), "My last prompt");
  assert.equal(segmentLabel("Cwd"), "Cwd");
});
