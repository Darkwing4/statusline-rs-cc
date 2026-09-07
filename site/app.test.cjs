const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const app = require("./app.js");

const {
  cacheColdGradientStops,
  cacheTtlPreview,
  cacheTtlGradientStops,
  colorToRgb,
  configProblems,
  contextGradientStops,
  createSegment,
  cut,
  defaultLine,
  describeLineFill,
  displayWidth,
  generateRon,
  gradientRgb,
  indexAfterNeighbour,
  installCatalog,
  interpolateColorStops,
  isStandalone,
  layoutRows,
  nearestDropSlot,
  presets,
  previewGitBranch,
  previewSegment,
  resetState,
  resolveCacheTtlColor,
  resolveContextColor,
  runtimePreviewColumns,
  scenarios,
  segmentLabel,
  segmentToRon,
  shelfModules,
  state,
  wrapPreviewSegments
} = app;

const catalogPath = path.join(__dirname, "segment-catalog.json");
assert.ok(
  fs.existsSync(catalogPath),
  "segment-catalog.json is missing: run `cargo run -- --schema > site/segment-catalog.json`"
);
const catalog = JSON.parse(fs.readFileSync(catalogPath, "utf8"));
const modules = installCatalog(catalog);

const gradient = { kind: "Gradient" };
const named = { kind: "Named", code: 32 };

function pieces(text) {
  return [{ text, color: "#ffffff" }];
}

function entry(text, standalone = false) {
  return { pieces: pieces(text), standalone };
}

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

test("the default line only uses pieces the catalogue knows", () => {
  defaultLine.forEach((id) => assert.ok(modules.some((module) => module.id === id), id));
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

test("every piece draws something in at least one session", () => {
  modules.forEach((module) => {
    const segment = createSegment(module.id);
    const drawn = scenarios.some((scenario) => {
      const result = previewSegment(segment, scenario);
      return Array.isArray(result) && result.length > 0;
    });
    assert.ok(drawn, module.id);
  });
});

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
  assert.ok(ron.startsWith("(\n    separator: \" \",\n    separator_color: Named(90),\n    segments: [\n"));
  assert.ok(ron.includes("        Model(\n            color: Rgb(180, 142, 173),\n            prefix: \"\",\n            replacements: [],\n        ),"));
  assert.ok(ron.includes("            window: FiveHour,"));
  assert.ok(ron.includes("            window: SevenDay,"));
  assert.ok(ron.trimEnd().endsWith("    ],\n)"));
});

test("segment names become readable labels", () => {
  assert.equal(segmentLabel("PromptCacheTtl"), "Prompt cache TTL");
  assert.equal(segmentLabel("LlmInsight"), "LLM insight");
  assert.equal(segmentLabel("MyLastPrompt"), "My last prompt");
  assert.equal(segmentLabel("Cwd"), "Cwd");
});

test("model replacements are applied in order in the preview", () => {
  const model = createSegment("Model");
  model.config.replacements = [[" (1M context)", ""], ["Opus 5", "Opus"]];
  assert.equal(previewSegment(model, scenarios[0])[0].text, "Opus");
  model.config.replacements = [["Opus 5 (1M context)", ""]];
  assert.equal(previewSegment(model, scenarios[0]), null);
});

test("the fable window shows its markers only in a fable session", () => {
  const fable = createSegment("RateLimit:Fable");
  fable.config.style = "Percent";
  fable.config.active_marker = "*";
  fable.config.severity_markers = [["warning", "!"]];
  assert.equal(previewSegment(fable, scenarios[0]), null);
  const session = scenarios.find((scenario) => scenario.id === "fable");
  assert.equal(previewSegment(fable, session)[0].text, "1.5d 76%!*");
});

test("subagent stats follow the runtime format", () => {
  const stats = createSegment("SubagentStats");
  assert.equal(previewSegment(stats, scenarios.find((s) => s.id === "active"))[0].text, "agents 2/7 4m12s 1.2M");
  assert.equal(previewSegment(stats, scenarios.find((s) => s.id === "clean"))[0].text, "agents 7 42k");
  assert.equal(previewSegment(stats, scenarios.find((s) => s.id === "pressure"))[0].text, "agents 1/3! 1h02m");
  assert.equal(previewSegment(stats, scenarios.find((s) => s.id === "outside")), null);
});

test("a session notice shows the time left the way the runtime pads it", () => {
  const notice = createSegment("SessionNotice");
  assert.equal(previewSegment(notice, scenarios[0])[0].text, "📌 ✗ cargo test (exit 101) (9m55s)");
  notice.config.show_remaining = false;
  assert.equal(previewSegment(notice, scenarios[0])[0].text, "📌 ✗ cargo test (exit 101)");
});

test("text pieces are cut to max_chars with an ellipsis and zero means unlimited", () => {
  assert.equal(cut("абвгде", 4), "абв…");
  assert.equal(cut("абвгде", 6), "абвгде");
  assert.equal(cut("абвгде", 0), "абвгде");
  const prompt = createSegment("MyLastPrompt");
  prompt.config.max_chars = 10;
  assert.equal(previewSegment(prompt, scenarios[0])[0].text, "» rebase th…");
});

test("standalone pieces come from the config or from the segment itself", () => {
  assert.ok(isStandalone(createSegment("LlmAnswer")));
  assert.ok(isStandalone(createSegment("SessionNotice")));
  assert.ok(!isStandalone(createSegment("Reminder")));
  assert.ok(isStandalone(createSegment("Spacer")));
});

test("a spacer takes no columns", () => {
  const spacer = createSegment("Spacer");
  const [part] = previewSegment(spacer, scenarios[0]);
  assert.equal(displayWidth(part.text), 0);
});

test("standalone pieces get rows of their own after the main block", () => {
  const rows = layoutRows(
    [entry("one"), entry("below", true), entry("two"), entry("also below", true)],
    " ",
    80
  );
  assert.equal(rows.length, 3);
  assert.deepEqual(rows.map((row) => row.standalone), [false, true, true]);
  assert.deepEqual(rows[0].cells.map((cell) => cell.entry.pieces[0].text), ["one", "two"]);
  assert.equal(rows[1].cells[0].entry.pieces[0].text, "below");
  assert.equal(layoutRows([entry("below", true)], " ", 80)[0].cells.length, 0);
});

test("context gradient matches the runtime stops and truncation", () => {
  assert.deepEqual(gradientRgb(contextGradientStops, -1, "truncate"), [150, 150, 150]);
  assert.deepEqual(gradientRgb(contextGradientStops, 0, "truncate"), [150, 150, 150]);
  assert.deepEqual(gradientRgb(contextGradientStops, 10, "truncate"), [165, 157, 125]);
  assert.deepEqual(gradientRgb(contextGradientStops, 20, "truncate"), [180, 165, 100]);
  assert.deepEqual(gradientRgb(contextGradientStops, 25, "truncate"), [200, 112, 80]);
  assert.deepEqual(gradientRgb(contextGradientStops, 30, "truncate"), [220, 60, 60]);
  assert.deepEqual(gradientRgb(contextGradientStops, 100, "truncate"), [220, 60, 60]);
  assert.equal(resolveContextColor(gradient, 25), "#c87050");
});

test("cache gradient matches active and cold runtime modes", () => {
  assert.deepEqual(gradientRgb(cacheTtlGradientStops, 80, "truncate"), [215, 140, 70]);
  assert.deepEqual(gradientRgb(cacheTtlGradientStops, 95, "truncate"), [242, 75, 55]);
  assert.deepEqual(gradientRgb(cacheColdGradientStops, 57.5, "truncate"), [215, 140, 70]);
  assert.deepEqual(gradientRgb(cacheColdGradientStops, 87.5, "truncate"), [242, 75, 55]);
  assert.equal(resolveCacheTtlColor(gradient, { cold: false, percentage: 80 }), "#d78c46");
  assert.equal(resolveCacheTtlColor(gradient, { cold: true, percentage: 87.5 }), "#f24b37");
});

test("cache scenario derives text and burned percentage from runtime values", () => {
  const active = cacheTtlPreview({
    cacheTtlSeconds: 300,
    cacheRemainingSeconds: 272,
    context: 42
  });
  const clean = cacheTtlPreview({
    cacheTtlSeconds: 3600,
    cacheRemainingSeconds: 484,
    context: 12
  });
  const cold = cacheTtlPreview({
    cacheTtlSeconds: 300,
    cacheRemainingSeconds: 0,
    context: 88
  });

  assert.deepEqual(active, {
    cold: false,
    percentage: 28 / 300 * 100,
    text: "4m32s"
  });
  assert.deepEqual(clean, {
    cold: false,
    percentage: 3116 / 3600 * 100,
    text: "8m04s"
  });
  assert.deepEqual(cold, {
    cold: true,
    percentage: 88,
    text: "cold"
  });
  assert.equal(resolveCacheTtlColor(gradient, active), "#828072");
  assert.equal(resolveCacheTtlColor(gradient, clean), "#e0713f");
});

test("rate limit gradient uses runtime fallbacks for non RGB colors", () => {
  assert.deepEqual(colorToRgb(named, [60, 200, 60]), [60, 200, 60]);
  assert.equal(
    interpolateColorStops(
      colorToRgb(named, [60, 200, 60]),
      colorToRgb(named, [220, 200, 40]),
      colorToRgb(named, [220, 60, 60]),
      25,
      50
    ),
    "#8cc832"
  );
});

test("git state has the runtime space before the state marker", () => {
  const drawn = previewGitBranch(
    {
      color: named,
      state_color: { kind: "Named", code: 91 },
      show_worktree: false,
      show_state: true,
      show_ahead_behind: true
    },
    {
      git: true,
      worktree: false,
      branch: "main",
      state: "REBASE 2/5",
      ahead: 1,
      behind: 3
    }
  );
  assert.equal(drawn.map((piece) => piece.text).join(""), "main [REBASE 2/5](↑1 ↓3)");
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
  assert.equal(displayWidth("é"), 1);
  assert.equal(displayWidth("\u2060"), 0);
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

test("a piece lands after its nearest visual neighbour of the same kind", () => {
  const seg = (id, standalone = false) => ({ id, type: "Cwd", config: { standalone } });
  const segments = [seg("a"), seg("notice", true), seg("b"), seg("c")];
  const visible = ["a", "b", "c", "notice"];

  assert.equal(indexAfterNeighbour(segments, visible, 3, false), 4);
  assert.equal(indexAfterNeighbour(segments, visible, 0, false), 1);
  assert.equal(indexAfterNeighbour(segments, visible, -1, false), 0);
  assert.equal(indexAfterNeighbour(segments, visible, 2, true), 4);
  assert.equal(indexAfterNeighbour(segments, visible, 3, true), 2);
});
