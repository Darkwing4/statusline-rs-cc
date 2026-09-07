import assert from "node:assert/strict";
import test from "node:test";

import { modules } from "./catalog-from-disk.js";
import { createSegment, resetState, state } from "./builder-state.js";
import { configProblems, generateRon, segmentToRon } from "./ron-config.js";

test("every piece serialises every field the catalogue declares", () => {
  modules.forEach((module) => {
    const lines = segmentToRon(createSegment(module.id));
    assert.equal(lines[0], `        ${module.type}(`);
    module.fields.forEach((field) => {
      assert.ok(lines.some((line) => line.startsWith(`            ${field.name}: `)), `${module.id}.${field.name}`);
    });
  });
});

test("lists and pairs serialise as ron arrays", () => {
  const model = createSegment("Model");
  model.config.replacements = [["Opus 5 (1M context)", "Opus"], ["Sonnet 5", "Sonnet"]];
  assert.ok(segmentToRon(model).includes('            replacements: [("Opus 5 (1M context)", "Opus"), ("Sonnet 5", "Sonnet")],'));

  const answer = createSegment("LlmAnswer");
  answer.config.args = ["exec", "-s", "read-only"];
  assert.ok(segmentToRon(answer).includes('            args: ["exec", "-s", "read-only"],'));
});

test("a pair without a left side is reported instead of silently dropped", () => {
  resetState();
  const model = state.segments.find((segment) => segment.type === "Model");
  model.config.replacements = [["", "Opus"]];
  assert.deepEqual(configProblems(), ["Model: every replacements row needs a left side."]);
  model.config.replacements = [];
  assert.deepEqual(configProblems(), []);
});

test("the default line renders the default config shape", () => {
  resetState();
  const ron = generateRon();
  assert.ok(ron.startsWith("(\n    separator: \" \",\n    separator_color: Rgb(120, 125, 140),\n    segments: [\n"));
  assert.ok(ron.includes("        Model(\n            color: Rgb(180, 142, 173),\n            prefix: \"\",\n            replacements: [],\n        ),"));
  assert.ok(ron.includes("            window: FiveHour,"));
  assert.ok(ron.includes("            window: SevenDay,"));
  assert.ok(ron.trimEnd().endsWith("    ],\n)"));
});
