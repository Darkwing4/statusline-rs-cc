import assert from "node:assert/strict";
import test from "node:test";

import { modules } from "./catalog-from-disk.js";
import { scenarios } from "./preview-sessions.js";
import { createSegment } from "./builder-state.js";
import { cut, displayWidth } from "./line-layout.js";
import { previewGitBranch, previewSegment } from "./segment-preview.js";

const named = { kind: "Named", code: 32 };

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

test("model replacements are applied in order in the preview", () => {
  const model = createSegment("Model");
  model.config.replacements = [[" (1M context)", ""], ["Opus 5", "Opus"]];
  assert.equal(previewSegment(model, scenarios[0])[0].text, "Opus");
  model.config.replacements = [["Opus 5 (1M context)", ""]];
  assert.equal(previewSegment(model, scenarios[0]), null);
});

test("the fable window shows its markers only in a fable session", () => {
  const fable = createSegment("RateLimit:Fable");
  fable.config.prefix = "{t}d ";
  fable.config.active_marker = "*";
  fable.config.severity_markers = [["warning", "!"]];
  assert.equal(previewSegment(fable, scenarios[0]), null);
  const session = scenarios.find((scenario) => scenario.id === "fable");
  assert.equal(previewSegment(fable, session)[0].text, "1.5d 76%!*");
});

test("subagent stats follow the runtime format", () => {
  const stats = createSegment("SubagentStats");
  assert.equal(previewSegment(stats, scenarios.find((s) => s.id === "active"))[0].text, "Sub-agents:2/7 4m12s 1.2M");
  assert.equal(previewSegment(stats, scenarios.find((s) => s.id === "clean"))[0].text, "Sub-agents:7 42k");
  assert.equal(previewSegment(stats, scenarios.find((s) => s.id === "pressure"))[0].text, "Sub-agents:1/3! 1h02m");
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

test("a spacer takes no columns", () => {
  const spacer = createSegment("Spacer");
  const [part] = previewSegment(spacer, scenarios[0]);
  assert.equal(displayWidth(part.text), 0);
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
