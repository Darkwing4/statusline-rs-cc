"use strict";

const installerUrl = "https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.sh";
const catalogUrl = "segment-catalog.json";

const ansiColors = [
  [30, "Black", "#303642"],
  [31, "Red", "#d85c68"],
  [32, "Green", "#69c88d"],
  [33, "Yellow", "#d4b967"],
  [34, "Blue", "#668fd4"],
  [35, "Magenta", "#ad7ad0"],
  [36, "Cyan", "#58c4d1"],
  [37, "White", "#c9d1dc"],
  [90, "Bright black", "#66758a"],
  [91, "Bright red", "#ff7c88"],
  [92, "Bright green", "#78d49b"],
  [93, "Bright yellow", "#e4c879"],
  [94, "Bright blue", "#7fa8ef"],
  [95, "Bright magenta", "#c99af2"],
  [96, "Bright cyan", "#63d9e6"],
  [97, "Bright white", "#e8edf5"]
];

const contextGradientStops = [
  [0, [150, 150, 150]],
  [20, [180, 165, 100]],
  [30, [220, 60, 60]]
];

const cacheTtlGradientStops = [
  [0, [120, 120, 120]],
  [70, [200, 180, 80]],
  [90, [230, 100, 60]],
  [100, [255, 50, 50]]
];

const cacheColdGradientStops = [
  [0, [120, 120, 120]],
  [40, [200, 180, 80]],
  [75, [230, 100, 60]],
  [100, [255, 50, 50]]
];

const windowLabels = { FiveHour: "5h", SevenDay: "7d" };
const windowPrefixes = { FiveHour: "{t}h " };
const acronyms = { Ttl: "TTL", Llm: "LLM" };
const gradientFields = new Set(["ContextUsage.color", "PromptCacheTtl.color"]);
const alwaysStandalone = new Set(["LlmAnswer"]);
const repeatableSegments = new Set(["Spacer"]);

const presets = {
  Model: { color: rgbColor(180, 142, 173) },
  Effort: { color: namedColor(90) },
  ContextUsage: {
    color: gradientColor(),
    prefix_color: rgbColor(180, 142, 173),
    suffix_color: rgbColor(180, 142, 173)
  },
  PromptCacheTtl: { color: gradientColor(), prefix: "cache " },
  RateLimit: {
    style: "Bar",
    fill: "Remaining",
    color_mode: "Gradient",
    gradient_midpoint_percentage: 50,
    usage_ttl_seconds: 300,
    low_color: rgbColor(103, 175, 103),
    mid_color: rgbColor(195, 179, 100),
    high_color: rgbColor(220, 60, 60)
  },
  SubagentStats: {
    color: namedColor(90),
    active_color: rgbColor(150, 200, 100),
    stall_color: rgbColor(220, 60, 60),
    prefix: "agents ",
    stall_marker: "!",
    stall_seconds: 120,
    show_tokens: true
  },
  Reminder: { color: rgbColor(230, 180, 80), prefix: "⏰ ", separator: " · ", max_chars: 80 },
  SessionNotice: {
    color: namedColor(93),
    prefix: "📌 ",
    max_chars: 120,
    show_remaining: true,
    standalone: true
  },
  Cwd: { color: rgbColor(95, 175, 175) },
  GitBranch: {
    color: namedColor(32),
    state_color: namedColor(91),
    show_worktree: true,
    show_ahead_behind: true,
    show_state: true
  },
  GitDiff: {
    modified_color: namedColor(33),
    untracked_color: namedColor(32),
    deleted_color: namedColor(31)
  },
  GitError: { color: namedColor(91), text: "no git" },
  MyLastPrompt: { color: namedColor(90), prefix: "» ", max_chars: 48 },
  ClaudeResourceUsage: { color: namedColor(90), cpu_prefix: "CPU ", memory_prefix: "RSS " },
  UserIdleTime: { color: namedColor(90), prefix: "idle " },
  Spacer: { standalone: true },
  LlmAnswer: {
    color: namedColor(90),
    command: "codex",
    args: ["exec", "-s", "read-only", "-c", "approval_policy=never"],
    prompt: "One short tip for working with Claude Code.",
    ttl_seconds: 3600,
    max_chars: 120
  },
  LlmInsight: {
    color: rgbColor(150, 190, 150),
    prefix: "🎯 ",
    command: "codex",
    args: ["exec", "--skip-git-repo-check", "-s", "read-only", "-c", "approval_policy=never"],
    prompt: "One sentence: the user's goal and what is being done for it.",
    every_turns: 2,
    initial_scan_bytes: 262144,
    context_chars: 4000,
    max_chars: 128,
    standalone: true
  },
  Weather: { color: namedColor(90), format: "%c+%t", ttl_seconds: 1800, max_chars: 32 }
};

const defaultLine = [
  "Model",
  "Effort",
  "ContextUsage",
  "PromptCacheTtl",
  "RateLimit:FiveHour",
  "RateLimit:SevenDay",
  "SubagentStats",
  "Reminder",
  "SessionNotice",
  "Cwd",
  "GitBranch",
  "GitDiff",
  "GitError",
  "MyLastPrompt"
];

const scenarios = [
  {
    id: "active",
    label: "Active worktree",
    model: "Opus 5 (1M context)",
    effort: "high",
    context: 42,
    cacheTtlSeconds: 3600,
    cacheRemainingSeconds: 2720,
    rateLimits: {
      FiveHour: { used: 36, countdown: "3.1" },
      SevenDay: { used: 21, countdown: "4.8" }
    },
    cwd: "~/AI.Admin/statusline-rs",
    git: true,
    branch: "feature/builder-site",
    worktree: true,
    ahead: 2,
    behind: 0,
    state: "",
    diff: { modified: 2, untracked: 1, deleted: 0 },
    idleSeconds: 42,
    cpu: "1.10c",
    rss: "685",
    agents: { active: 2, total: 7, longestSeconds: 252, tokens: 1240000, stalled: false },
    reminders: ["standup 11:00"],
    notice: { text: "✗ cargo test (exit 101)", remainingSeconds: 595 },
    lastPrompt: "rebase the builder branch onto develop and check every segment",
    llmAnswer: "Keep the prompt short and let the model ask for what it needs.",
    llmInsight: "goal: ship the builder site with the next release; the catalogue is generated",
    weather: "⛅ +21°C"
  },
  {
    id: "fable",
    label: "Fable session",
    model: "Fable 5.1",
    effort: "max",
    context: 63,
    cacheTtlSeconds: 3600,
    cacheRemainingSeconds: 1210,
    rateLimits: {
      FiveHour: { used: 48, countdown: "2.2" },
      SevenDay: { used: 31, countdown: "3.9" },
      Fable: { used: 76, countdown: "1.5", severity: "warning", active: true }
    },
    cwd: "~/AI.Admin/statusline-rs",
    git: true,
    branch: "develop",
    worktree: false,
    ahead: 0,
    behind: 0,
    state: "",
    diff: { modified: 4, untracked: 0, deleted: 1 },
    idleSeconds: 9,
    cpu: "0.84c",
    rss: "912",
    agents: { active: 1, total: 2, longestSeconds: 61, tokens: 380000, stalled: false },
    reminders: [],
    notice: null,
    lastPrompt: "make the 5h window radial",
    llmAnswer: "Name the segment after what it shows, not after where it reads.",
    llmInsight: "goal: recolour the line; the 5h window is being switched to radial",
    weather: "🌦 +18°C"
  },
  {
    id: "pressure",
    label: "Rebase pressure",
    model: "Opus 5",
    effort: "max",
    context: 88,
    cacheTtlSeconds: 300,
    cacheRemainingSeconds: 0,
    rateLimits: {
      FiveHour: { used: 84, countdown: "0.6" },
      SevenDay: { used: 92, countdown: "0.9" }
    },
    cwd: "~/multicast/escape2",
    git: true,
    branch: "fix/input-buffer",
    worktree: false,
    ahead: 1,
    behind: 3,
    state: "REBASE 2/5",
    diff: { modified: 8, untracked: 2, deleted: 1 },
    idleSeconds: 367,
    cpu: "2.37c",
    rss: "1214",
    agents: { active: 1, total: 3, longestSeconds: 3720, tokens: 0, stalled: true },
    reminders: ["stand up", "drink water"],
    notice: { text: "do not push before the review", remainingSeconds: null },
    lastPrompt: "continue the rebase and fix the conflicts in the input buffer",
    llmAnswer: "Resolve the smallest conflict first.",
    llmInsight: "goal: finish the rebase; conflicts in the input buffer are being resolved",
    weather: "🌧 +12°C"
  },
  {
    id: "clean",
    label: "Clean branch",
    model: "Sonnet 5",
    effort: "medium",
    context: 12,
    cacheTtlSeconds: 3600,
    cacheRemainingSeconds: 484,
    rateLimits: {
      FiveHour: { used: 12, countdown: "4.4" },
      SevenDay: { used: 8, countdown: "6.3" }
    },
    cwd: "~/courses",
    git: true,
    branch: "main",
    worktree: false,
    ahead: 0,
    behind: 0,
    state: "",
    diff: { modified: 0, untracked: 0, deleted: 0 },
    idleSeconds: 7,
    cpu: "0.06c",
    rss: "224",
    agents: { active: 0, total: 7, longestSeconds: null, tokens: 42000, stalled: false },
    reminders: [],
    notice: null,
    lastPrompt: "summarise the lecture notes",
    llmAnswer: "Ask for an outline before the full text.",
    llmInsight: "goal: summarise the notes; an outline was just produced",
    weather: "☀️ +25°C"
  },
  {
    id: "outside",
    label: "Outside Git",
    model: "Haiku 4.5",
    effort: "low",
    context: 27,
    cacheTtlSeconds: 300,
    cacheRemainingSeconds: 0,
    rateLimits: {
      FiveHour: { used: 27, countdown: "2.8" },
      SevenDay: { used: 19, countdown: "5.1" }
    },
    cwd: "~/Downloads",
    git: false,
    branch: "",
    worktree: false,
    ahead: 0,
    behind: 0,
    state: "",
    diff: { modified: 0, untracked: 0, deleted: 0 },
    idleSeconds: 95,
    cpu: "0.22c",
    rss: "312",
    agents: { active: 0, total: 0, longestSeconds: null, tokens: 0, stalled: false },
    reminders: ["take the coffee"],
    notice: null,
    lastPrompt: "rename the downloaded files by date",
    llmAnswer: "Sort by modification time, then rename.",
    llmInsight: "goal: tidy the downloads; files are being renamed by date",
    weather: "🌫 +9°C"
  }
];

const LINE_SELECTION = "line";

let idSequence = 0;
let dragPayload = null;
let lastDownloadUrl = "";
let modules = [];
let catalogVersion = "";

const state = {
  separator: " ",
  separatorColor: namedColor(90),
  terminalWidth: 120,
  scenarioId: "active",
  segments: [],
  selectedId: LINE_SELECTION
};

const elements = {};

function namedColor(code) {
  return { kind: "Named", code };
}

function rgbColor(r, g, b) {
  return { kind: "Rgb", hex: rgbToHex(r, g, b) };
}

function gradientColor() {
  return { kind: "Gradient" };
}

function nextId() {
  idSequence += 1;
  if (globalThis.crypto && typeof globalThis.crypto.randomUUID === "function") {
    return `segment-${globalThis.crypto.randomUUID()}`;
  }
  return `segment-${Date.now()}-${idSequence}`;
}

function segmentLabel(name) {
  return name
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(" ")
    .map((word, index) => acronyms[word] || (index === 0 ? word : word.toLowerCase()))
    .join(" ");
}

function fieldLabel(name) {
  const text = name.replaceAll("_", " ");
  return text.charAt(0).toUpperCase() + text.slice(1);
}

function validateCatalog(catalog) {
  if (!catalog || typeof catalog.version !== "string" || !Array.isArray(catalog.segments)) {
    throw new Error("The segment catalogue has an unexpected shape.");
  }
  if (catalog.segments.length === 0) {
    throw new Error("The segment catalogue is empty.");
  }
  catalog.segments.forEach((segment) => {
    if (typeof segment.name !== "string" || typeof segment.pitch !== "string" || !Array.isArray(segment.fields)) {
      throw new Error("A catalogue entry is missing its name, pitch, or fields.");
    }
    segment.fields.forEach((field) => {
      if (typeof field.name !== "string" || typeof field.kind !== "string") {
        throw new Error(`${segment.name} has a field without a name or a kind.`);
      }
      if (field.kind === "enum" && (!Array.isArray(field.variants) || field.variants.length === 0)) {
        throw new Error(`${segment.name}.${field.name} is an enum without variants.`);
      }
    });
  });
  return catalog;
}

function buildModules(catalog) {
  return validateCatalog(catalog).segments.flatMap((segment) => {
    const base = { type: segment.name, description: segment.pitch, fields: segment.fields };
    if (segment.name === "RateLimit") {
      const window = segment.fields.find((field) => field.name === "window" && field.kind === "enum");
      if (!window) {
        throw new Error("RateLimit has no window enum to build the shelf from.");
      }
      return window.variants.map((variant) => ({
        ...base,
        id: `RateLimit:${variant}`,
        label: `Rate limit ${windowLabels[variant] || variant}`,
        overrides: { window: variant, prefix: windowPrefixes[variant] || "{t}d " },
        repeatable: false
      }));
    }
    return [{
      ...base,
      id: segment.name,
      label: segmentLabel(segment.name),
      overrides: {},
      repeatable: repeatableSegments.has(segment.name)
    }];
  });
}

function installCatalog(catalog) {
  modules = buildModules(catalog);
  catalogVersion = catalog.version;
  return modules;
}

function fieldDefault(field) {
  switch (field.kind) {
    case "color":
      return namedColor(90);
    case "text":
      return "";
    case "bool":
      return false;
    case "integer":
    case "float":
      return 0;
    case "list":
    case "pairs":
      return [];
    case "enum":
      return field.variants[0];
    default:
      throw new Error(`Unknown field kind: ${field.kind}`);
  }
}

function segmentDefaults(entry) {
  const preset = presets[entry.type] || {};
  const config = {};
  entry.fields.forEach((field) => {
    const value = Object.hasOwn(preset, field.name) ? preset[field.name] : fieldDefault(field);
    config[field.name] = structuredClone(value);
  });
  return { ...config, ...structuredClone(entry.overrides) };
}

function createSegment(moduleId) {
  const entry = moduleEntry(moduleId);
  return {
    id: nextId(),
    moduleId,
    type: entry.type,
    config: segmentDefaults(entry)
  };
}

function buildDefaultSegments() {
  return defaultLine.map(createSegment);
}

function resetState() {
  state.separator = " ";
  state.separatorColor = namedColor(90);
  state.terminalWidth = 120;
  state.scenarioId = "active";
  state.segments = buildDefaultSegments();
  state.selectedId = LINE_SELECTION;
}

function element(tagName, className = "", text = "") {
  const node = document.createElement(tagName);
  if (className) {
    node.className = className;
  }
  if (text) {
    node.textContent = text;
  }
  return node;
}

function moduleEntry(moduleId) {
  const entry = modules.find((candidate) => candidate.id === moduleId);
  if (!entry) {
    throw new Error(`Unknown module: ${moduleId}`);
  }
  return entry;
}

function segmentModule(segment) {
  return moduleEntry(segment.moduleId);
}

function selectedSegment() {
  return state.segments.find((segment) => segment.id === state.selectedId) || null;
}

function usedModuleIds() {
  return new Set(state.segments.map((segment) => segment.moduleId));
}

function scenario() {
  return scenarios.find((entry) => entry.id === state.scenarioId) || scenarios[0];
}

function isStandalone(segment) {
  return alwaysStandalone.has(segment.type) || segment.config.standalone === true;
}

function announce(message) {
  elements.liveRegion.textContent = "";
  requestAnimationFrame(() => {
    elements.liveRegion.textContent = message;
  });
}

function createControl(field, type, value, onChange) {
  const label = fieldLabel(field.name);
  switch (field.kind) {
    case "color":
      return createColorControl(label, value, onChange, gradientFields.has(`${type}.${field.name}`));
    case "bool":
      return createBooleanControl(label, value, onChange);
    case "enum":
      return createSelectControl(label, field.variants, value, onChange);
    case "integer":
      return createNumberControl(label, { min: 0, max: 1000000000, step: 1 }, value, onChange);
    case "float":
      return createNumberControl(label, floatRange(field.name), value, onChange);
    case "list":
      return createListControl(label, value, onChange);
    case "pairs":
      return createPairsControl(label, value, onChange);
    default:
      return createTextControl(label, value, onChange, field.name === "prompt" ? 2000 : 256);
  }
}

function floatRange(fieldName) {
  if (fieldName === "gradient_midpoint_percentage") {
    return { min: 0.1, max: 99.9, step: 0.1 };
  }
  return { min: 0, max: 1000000000, step: 0.1 };
}

function createTextControl(labelText, value, onChange, maxLength) {
  const label = element("label", "control-label");
  label.append(element("span", "", labelText));
  const input = element("input", "control-input");
  input.type = "text";
  input.value = value;
  input.maxLength = maxLength || 128;
  input.autocomplete = "off";
  input.spellcheck = false;
  input.addEventListener("input", () => onChange(input.value, false));
  label.append(input);
  return label;
}

function createNumberControl(labelText, range, value, onChange) {
  const label = element("label", "control-label");
  label.append(element("span", "", labelText));
  const input = element("input", "control-input");
  input.type = "number";
  input.value = String(value);
  input.min = String(range.min);
  input.max = String(range.max);
  input.step = String(range.step);
  input.addEventListener("input", () => {
    if (input.value === "" || !input.validity.valid) {
      return;
    }
    const number = Number(input.value);
    if (Number.isFinite(number)) {
      onChange(number, false);
    }
  });
  input.addEventListener("change", () => {
    let number = Number(input.value);
    if (!Number.isFinite(number)) {
      number = value;
    }
    number = Math.max(range.min, Math.min(range.max, number));
    if (range.step === 1) {
      number = Math.round(number);
    }
    input.value = String(number);
    onChange(number, false);
  });
  label.append(input);
  return label;
}

function createSelectControl(labelText, variants, value, onChange) {
  const label = element("label", "control-label");
  label.append(element("span", "", labelText));
  const select = element("select", "control-input select-control");
  variants.forEach((variant) => {
    const option = element("option", "", segmentLabel(variant));
    option.value = variant;
    select.append(option);
  });
  select.value = value;
  select.addEventListener("change", () => onChange(select.value, false));
  label.append(select);
  return label;
}

function createBooleanControl(labelText, value, onChange) {
  const label = element("label", "toggle-control");
  label.append(element("span", "", labelText));
  const input = element("input");
  input.type = "checkbox";
  input.checked = Boolean(value);
  input.addEventListener("change", () => onChange(input.checked, true));
  label.append(input);
  return label;
}

function createListControl(labelText, value, onChange) {
  const items = value.map((item) => String(item));
  const wrapper = element("div", "list-control");
  wrapper.append(element("span", "list-control-label", labelText));
  const rows = element("div", "list-rows");

  items.forEach((item, index) => {
    const row = element("div", "list-row");
    const input = element("input", "control-input");
    input.type = "text";
    input.value = item;
    input.maxLength = 256;
    input.autocomplete = "off";
    input.spellcheck = false;
    input.setAttribute("aria-label", `${labelText} ${index + 1}`);
    input.addEventListener("input", () => {
      items[index] = input.value;
      onChange(items.slice(), false);
    });
    row.append(input, createRemoveButton(`${labelText} ${index + 1}`, () => {
      onChange(items.filter((_, at) => at !== index), true);
    }));
    rows.append(row);
  });

  wrapper.append(rows, createAddButton(labelText, () => onChange([...items, ""], true)));
  return wrapper;
}

function createPairsControl(labelText, value, onChange) {
  const pairs = value.map(([from, to]) => [String(from), String(to)]);
  const wrapper = element("div", "list-control");
  wrapper.append(element("span", "list-control-label", labelText));
  const rows = element("div", "list-rows");

  pairs.forEach((pair, index) => {
    const row = element("div", "list-row list-row-pair");
    ["from", "to"].forEach((side, position) => {
      const input = element("input", "control-input");
      input.type = "text";
      input.value = pair[position];
      input.maxLength = 256;
      input.autocomplete = "off";
      input.spellcheck = false;
      input.placeholder = side;
      input.setAttribute("aria-label", `${labelText} ${index + 1} ${side}`);
      input.addEventListener("input", () => {
        pairs[index][position] = input.value;
        onChange(pairs.map((entry) => entry.slice()), false);
      });
      row.append(input);
    });
    row.append(createRemoveButton(`${labelText} ${index + 1}`, () => {
      onChange(pairs.filter((_, at) => at !== index), true);
    }));
    rows.append(row);
  });

  wrapper.append(rows, createAddButton(labelText, () => onChange([...pairs, ["", ""]], true)));
  return wrapper;
}

function createRemoveButton(what, onClick) {
  const button = element("button", "list-remove", "×");
  button.type = "button";
  button.setAttribute("aria-label", `Remove ${what}`);
  button.addEventListener("click", onClick);
  return button;
}

function createAddButton(what, onClick) {
  const button = element("button", "list-add", "+ add");
  button.type = "button";
  button.setAttribute("aria-label", `Add ${what}`);
  button.addEventListener("click", onClick);
  return button;
}

function createColorControl(labelText, value, onChange, allowGradient) {
  const wrapper = element("div", "color-control");
  const heading = element("div", "color-control-label");
  heading.append(element("span", "", labelText));
  const swatch = element("span", "color-swatch");
  swatch.setAttribute("aria-hidden", "true");
  swatch.style.setProperty("--swatch", colorSwatch(value));
  heading.append(swatch);

  const editor = element("div", "color-editor");
  const kindSelect = element("select", "color-kind");
  const colorKinds = [
    ["Named", "ANSI"],
    ["Rgb", "RGB"]
  ];
  if (allowGradient) {
    colorKinds.push(["Gradient", "Gradient"]);
  }
  colorKinds.forEach(([kind, name]) => {
    const option = element("option", "", name);
    option.value = kind;
    kindSelect.append(option);
  });
  kindSelect.value = value.kind;
  kindSelect.setAttribute("aria-label", `${labelText} color type`);
  kindSelect.addEventListener("change", () => {
    let next;
    if (kindSelect.value === "Named") {
      next = namedColor(90);
    } else if (kindSelect.value === "Rgb") {
      const [r, g, b] = colorToRgb(value, [183, 165, 255]);
      next = rgbColor(r, g, b);
    } else {
      next = gradientColor();
    }
    onChange(next, true);
  });
  editor.append(kindSelect);

  if (value.kind === "Named") {
    const select = element("select", "ansi-select");
    select.setAttribute("aria-label", `${labelText} ANSI color`);
    ansiColors.forEach(([code, name]) => {
      const option = element("option", "", `${code} · ${name}`);
      option.value = String(code);
      select.append(option);
    });
    select.value = String(value.code);
    select.addEventListener("change", () => {
      const next = namedColor(Number(select.value));
      swatch.style.setProperty("--swatch", colorSwatch(next));
      onChange(next, false);
    });
    editor.append(select);
  } else if (value.kind === "Rgb") {
    const input = element("input", "rgb-input");
    input.type = "color";
    input.value = value.hex;
    input.setAttribute("aria-label", `${labelText} RGB color`);
    input.addEventListener("input", () => {
      const next = { kind: "Rgb", hex: input.value };
      swatch.style.setProperty("--swatch", colorSwatch(next));
      onChange(next, false);
    });
    editor.append(input);
  } else {
    editor.append(element("div", "gradient-readout", "follows the value"));
  }

  wrapper.append(heading, editor);
  return wrapper;
}

function configMutated() {
  invalidateSession();
  renderDerived();
}

function invalidateSession() {
  elements.sessionResult.hidden = true;
  elements.sessionError.hidden = true;
  elements.sessionError.textContent = "";
  elements.commandOutput.value = "";
}

function runtimePreviewColumns(columns) {
  return Math.max(0, Math.floor(columns) - 4);
}

function wrapPreviewSegments(segments, separator, maxColumns) {
  if (segments.length === 0) {
    return [];
  }

  const lines = [];
  const separatorWidth = displayWidth(separator);
  let currentLine = [];
  let currentWidth = 0;

  segments.forEach((pieces) => {
    const segmentWidth = pieces.reduce((sum, value) => sum + displayWidth(value.text), 0);
    if (currentLine.length === 0) {
      currentLine.push({ pieces, separator: false });
      currentWidth = segmentWidth;
      return;
    }

    const projectedWidth = currentWidth + separatorWidth + segmentWidth;
    if (projectedWidth > maxColumns) {
      lines.push(currentLine);
      currentLine = [{ pieces, separator: false }];
      currentWidth = segmentWidth;
      return;
    }

    currentLine.push({ pieces, separator: true });
    currentWidth = projectedWidth;
  });

  if (currentLine.length > 0) {
    lines.push(currentLine);
  }
  return lines;
}

function layoutRows(entries, separator, maxColumns) {
  const main = entries.filter((entry) => !entry.standalone);
  const tail = entries.filter((entry) => entry.standalone);
  let cursor = 0;
  const rows = wrapPreviewSegments(main.map((entry) => entry.pieces), separator, maxColumns)
    .map((line) => ({
      standalone: false,
      cells: line.map((cell) => {
        const entry = main[cursor];
        cursor += 1;
        return { entry, separator: cell.separator };
      })
    }));

  if (rows.length === 0 && tail.length > 0) {
    rows.push({ standalone: false, cells: [] });
  }

  tail.forEach((entry) => {
    rows.push({ standalone: true, cells: [{ entry, separator: false }] });
  });

  return rows;
}

function displayWidth(value) {
  const segments = typeof Intl.Segmenter === "function"
    ? Array.from(new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(value), (entry) => entry.segment)
    : Array.from(value);

  return segments.reduce((width, grapheme) => {
    if (/^[\u0000-\u001f\u007f-\u009f]*$/u.test(grapheme)) {
      return width;
    }
    if (/\p{Extended_Pictographic}/u.test(grapheme)) {
      return width + 2;
    }
    if (/^\p{Regional_Indicator}{2}$/u.test(grapheme) || /\u20e3/u.test(grapheme)) {
      return width + 2;
    }
    const codePoint = grapheme.codePointAt(0);
    if (isWideCodePoint(codePoint)) {
      return width + 2;
    }
    if (/^[\p{Mark}\p{Format}]+$/u.test(grapheme)) {
      return width;
    }
    return width + 1;
  }, 0);
}

function isWideCodePoint(codePoint) {
  return codePoint >= 0x1100 && (
    codePoint <= 0x115f
    || codePoint === 0x2329
    || codePoint === 0x232a
    || codePoint >= 0x2e80 && codePoint <= 0xa4cf && codePoint !== 0x303f
    || codePoint >= 0xac00 && codePoint <= 0xd7a3
    || codePoint >= 0xf900 && codePoint <= 0xfaff
    || codePoint >= 0xfe10 && codePoint <= 0xfe19
    || codePoint >= 0xfe30 && codePoint <= 0xfe6f
    || codePoint >= 0xff00 && codePoint <= 0xff60
    || codePoint >= 0xffe0 && codePoint <= 0xffe6
    || codePoint >= 0x1b000 && codePoint <= 0x1b2ff
    || codePoint >= 0x20000 && codePoint <= 0x3fffd
  );
}

function cut(text, maxChars) {
  const characters = Array.from(text);
  if (maxChars === 0 || characters.length <= maxChars) {
    return text;
  }
  return `${characters.slice(0, Math.max(0, maxChars - 1)).join("")}…`;
}

function previewSegment(segment, activeScenario) {
  const config = segment.config;
  switch (segment.type) {
    case "Model":
      return previewModel(config, activeScenario);
    case "Effort":
      return [piece(`${config.prefix}${activeScenario.effort}`, resolveColor(config.color, 45))];
    case "ContextUsage":
      return [
        piece(config.prefix, resolveColor(config.prefix_color, activeScenario.context)),
        piece(`${Math.round(activeScenario.context)}%`, resolveContextColor(config.color, activeScenario.context)),
        piece(config.suffix, resolveColor(config.suffix_color, activeScenario.context))
      ];
    case "PromptCacheTtl": {
      const cacheView = cacheTtlPreview(activeScenario);
      return [
        piece(
          `${config.prefix}${cacheView.text}`,
          resolveCacheTtlColor(config.color, cacheView)
        )
      ];
    }
    case "ClaudeResourceUsage":
      return [
        piece(
          `${config.cpu_prefix}${activeScenario.cpu} ${config.memory_prefix}${activeScenario.rss} MiB`,
          resolveColor(config.color, 46)
        )
      ];
    case "Cwd":
      return [piece(activeScenario.cwd, resolveColor(config.color, 45))];
    case "GitBranch":
      return previewGitBranch(config, activeScenario);
    case "GitDiff":
      return previewGitDiff(config, activeScenario);
    case "GitError":
      return activeScenario.git ? null : [piece(config.text, resolveColor(config.color, 90))];
    case "UserIdleTime":
      if (activeScenario.idleSeconds < config.threshold_seconds) {
        return null;
      }
      return [
        piece(
          `${config.prefix}${formatDuration(activeScenario.idleSeconds)}`,
          resolveColor(config.color, 48)
        )
      ];
    case "RateLimit":
      return previewRateLimit(config, activeScenario);
    case "SubagentStats":
      return previewSubagentStats(config, activeScenario);
    case "Reminder":
      return previewText(config, cut(activeScenario.reminders.join(config.separator), config.max_chars));
    case "SessionNotice":
      return previewSessionNotice(config, activeScenario);
    case "MyLastPrompt":
      return previewText(config, cut(activeScenario.lastPrompt, config.max_chars));
    case "LlmAnswer":
      return previewText(config, cut(activeScenario.llmAnswer, config.max_chars));
    case "LlmInsight":
      return previewText(config, cut(activeScenario.llmInsight, config.max_chars));
    case "Weather":
      return previewText(config, cut(activeScenario.weather, config.max_chars));
    case "Spacer":
      return [piece("\u2060", "transparent")];
    default:
      return null;
  }
}

function previewText(config, text) {
  if (!text) {
    return null;
  }
  return [piece(`${config.prefix}${text}`, resolveColor(config.color, 50))];
}

function previewModel(config, activeScenario) {
  const name = (config.replacements || []).reduce(
    (current, [from, to]) => (from ? current.replaceAll(from, to) : current),
    activeScenario.model
  );
  if (!name) {
    return null;
  }
  return [piece(`${config.prefix}${name}`, resolveColor(config.color, 40))];
}

function previewSessionNotice(config, activeScenario) {
  const notice = activeScenario.notice;
  if (!notice) {
    return null;
  }
  const text = cut(notice.text, config.max_chars);
  if (!text) {
    return null;
  }
  let body = `${config.prefix}${text}`;
  if (config.show_remaining && notice.remainingSeconds !== null) {
    body += ` (${formatDurationPadded(notice.remainingSeconds)})`;
  }
  return [piece(body, resolveColor(config.color, 50))];
}

function previewSubagentStats(config, activeScenario) {
  const stats = activeScenario.agents;
  if (!stats || stats.total === 0) {
    return null;
  }
  let text = stats.active > 0
    ? `${config.prefix}${stats.active}/${stats.total}`
    : `${config.prefix}${stats.total}`;
  if (stats.stalled) {
    text += config.stall_marker;
  }
  if (stats.active > 0 && stats.longestSeconds !== null) {
    text += ` ${formatDurationPadded(stats.longestSeconds)}`;
  }
  if (config.show_tokens && stats.tokens > 0) {
    text += ` ${formatTokens(stats.tokens)}`;
  }
  let color = config.color;
  if (stats.stalled) {
    color = config.stall_color;
  } else if (stats.active > 0) {
    color = config.active_color;
  }
  return [piece(text, resolveColor(color, 50))];
}

function formatTokens(tokens) {
  if (tokens < 1000) {
    return String(tokens);
  }
  if (tokens < 1000000) {
    const thousands = tokens / 1000;
    return thousands < 10 ? `${thousands.toFixed(1)}k` : `${Math.round(thousands)}k`;
  }
  return `${(tokens / 1000000).toFixed(1)}M`;
}

function previewGitBranch(config, activeScenario) {
  if (!activeScenario.git) {
    return null;
  }
  let branch = "";
  if (config.show_worktree && activeScenario.worktree) {
    branch += "⑂";
  }
  branch += activeScenario.branch;
  const pieces = [piece(branch, resolveColor(config.color, 35))];

  if (config.show_state && activeScenario.state) {
    pieces.push(piece(` [${activeScenario.state}]`, resolveColor(config.state_color, 90)));
  }

  if (config.show_ahead_behind) {
    const movements = [];
    if (activeScenario.ahead > 0) {
      movements.push(`↑${activeScenario.ahead}`);
    }
    if (activeScenario.behind > 0) {
      movements.push(`↓${activeScenario.behind}`);
    }
    if (movements.length > 0) {
      pieces.push(piece(`(${movements.join(" ")})`, resolveColor(config.color, 35)));
    }
  }

  return pieces;
}

function previewGitDiff(config, activeScenario) {
  if (!activeScenario.git) {
    return null;
  }
  const pieces = [];
  if (activeScenario.diff.modified > 0) {
    pieces.push(piece(`~${activeScenario.diff.modified}`, resolveColor(config.modified_color, 55)));
  }
  if (activeScenario.diff.untracked > 0) {
    if (pieces.length > 0) {
      pieces.push(piece(" ", resolveColor(state.separatorColor, 50)));
    }
    pieces.push(piece(`+${activeScenario.diff.untracked}`, resolveColor(config.untracked_color, 35)));
  }
  if (activeScenario.diff.deleted > 0) {
    if (pieces.length > 0) {
      pieces.push(piece(" ", resolveColor(state.separatorColor, 50)));
    }
    pieces.push(piece(`-${activeScenario.diff.deleted}`, resolveColor(config.deleted_color, 90)));
  }
  return pieces.length > 0 ? pieces : null;
}

function previewRateLimit(config, activeScenario) {
  const data = activeScenario.rateLimits[config.window];
  if (!data) {
    return null;
  }
  const used = Math.max(0, Math.min(100, data.used));
  const glyphValue = config.fill === "Remaining" ? 100 - used : used;
  const prefix = config.prefix.replaceAll("{t}", data.countdown);
  const rounded = Math.round(used);
  let text;

  switch (config.style) {
    case "Percent":
      text = `${prefix}${rounded}%`;
      break;
    case "Bar":
      text = `${prefix}${barGlyph(glyphValue)}`;
      break;
    case "BarPercent":
      text = `${prefix}${barGlyph(glyphValue)} ${rounded}%`;
      break;
    case "Radial":
      text = `${prefix}${radialGlyph(glyphValue)}`;
      break;
    case "RadialPercent":
      text = `${prefix}${radialGlyph(glyphValue)} ${rounded}%`;
      break;
    default:
      text = `${prefix}${rounded}%`;
  }

  text += rateLimitMarkers(config, data);

  let color;
  if (config.color_mode === "Steps") {
    color = used < 50
      ? resolveColor(config.low_color, used)
      : used <= 80
        ? resolveColor(config.mid_color, used)
        : resolveColor(config.high_color, used);
  } else {
    color = interpolateColorStops(
      colorToRgb(config.low_color, [60, 200, 60]),
      colorToRgb(config.mid_color, [220, 200, 40]),
      colorToRgb(config.high_color, [220, 60, 60]),
      used,
      config.gradient_midpoint_percentage
    );
  }
  return [piece(text, color)];
}

function rateLimitMarkers(config, data) {
  const severity = (config.severity_markers || []).find(([name]) => name && name === data.severity);
  const active = data.active ? config.active_marker || "" : "";
  return `${severity ? severity[1] : ""}${active}`;
}

function piece(text, color) {
  return { text, color };
}

function barGlyph(percentage) {
  const glyphs = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
  const index = Math.max(0, Math.min(7, Math.floor(percentage * 8 / 100)));
  return glyphs[index];
}

function radialGlyph(percentage) {
  const glyphs = ["○", "◔", "◑", "◕", "●"];
  const index = Math.max(0, Math.min(4, Math.floor(percentage * 5 / 100)));
  return glyphs[index];
}

function formatDuration(totalSeconds) {
  const seconds = Math.max(0, Math.floor(totalSeconds));
  if (seconds >= 3600) {
    return `${Math.floor(seconds / 3600)}h${Math.floor(seconds % 3600 / 60)}m`;
  }
  if (seconds >= 60) {
    return `${Math.floor(seconds / 60)}m${seconds % 60}s`;
  }
  return `${seconds}s`;
}

function formatDurationPadded(totalSeconds) {
  const seconds = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor(seconds % 3600 / 60);
  const remainder = seconds % 60;
  if (hours > 0) {
    return `${hours}h${String(minutes).padStart(2, "0")}m`;
  }
  if (minutes > 0) {
    return `${minutes}m${String(remainder).padStart(2, "0")}s`;
  }
  return `${remainder}s`;
}

function cacheTtlPreview(activeScenario) {
  const ttlSeconds = Math.max(1, Math.floor(activeScenario.cacheTtlSeconds));
  const remainingSeconds = Math.floor(activeScenario.cacheRemainingSeconds);
  if (remainingSeconds <= 0) {
    return {
      cold: true,
      percentage: activeScenario.context,
      text: "cold"
    };
  }

  return {
    cold: false,
    percentage: ((ttlSeconds - remainingSeconds) / ttlSeconds) * 100,
    text: formatDurationPadded(remainingSeconds)
  };
}

function colorSwatch(color) {
  if (color.kind === "Gradient") {
    return "linear-gradient(90deg, #78d49b, #e4c879, #ff7c88)";
  }
  return resolveColor(color, 50);
}

function resolveContextColor(color, percentage) {
  if (color.kind !== "Gradient") {
    return resolveColor(color, percentage);
  }
  return rgbArrayToHex(gradientRgb(contextGradientStops, percentage, "truncate"));
}

function resolveCacheTtlColor(color, cacheView) {
  if (color.kind !== "Gradient") {
    return resolveColor(color, cacheView.percentage);
  }
  const stops = cacheView.cold ? cacheColdGradientStops : cacheTtlGradientStops;
  return rgbArrayToHex(gradientRgb(stops, cacheView.percentage, "truncate"));
}

function resolveColor(color, percentage) {
  if (color.kind === "Rgb") {
    return color.hex;
  }
  if (color.kind === "Named") {
    const entry = ansiColors.find(([code]) => code === color.code);
    return entry ? entry[2] : "#a6b1c2";
  }
  return interpolateColorStops(
    [60, 200, 60],
    [220, 200, 40],
    [220, 60, 60],
    percentage,
    50
  );
}

function colorToRgb(color, fallback) {
  if (color.kind === "Rgb") {
    return hexToRgb(color.hex);
  }
  return fallback;
}

function interpolateColorStops(low, middle, high, percentage, midpoint) {
  const stops = [
    [0, low],
    [midpoint, middle],
    [100, high]
  ];
  return rgbArrayToHex(gradientRgb(stops, percentage, "nearest"));
}

function gradientRgb(stops, percentage, quantization) {
  if (Number.isNaN(percentage) || stops.length === 0) {
    return [0, 0, 0];
  }

  const [firstPosition, firstColor] = stops[0];
  if (percentage <= firstPosition) {
    return [...firstColor];
  }

  for (let index = 0; index < stops.length - 1; index += 1) {
    const [startPosition, startColor] = stops[index];
    const [endPosition, endColor] = stops[index + 1];
    if (percentage > endPosition) {
      continue;
    }
    const span = endPosition - startPosition;
    if (span <= 0) {
      return [...endColor];
    }
    const amount = Math.max(0, Math.min(1, (percentage - startPosition) / span));
    return startColor.map((channel, channelIndex) => {
      const value = channel + (endColor[channelIndex] - channel) * amount;
      const bounded = Math.max(0, Math.min(255, value));
      return quantization === "truncate" ? Math.trunc(bounded) : Math.round(bounded);
    });
  }

  return [...stops[stops.length - 1][1]];
}

function rgbToHex(r, g, b) {
  return rgbArrayToHex([r, g, b]);
}

function rgbArrayToHex(channels) {
  return `#${channels
    .map((channel) => Math.max(0, Math.min(255, channel)).toString(16).padStart(2, "0"))
    .join("")}`;
}

function hexToRgb(hex) {
  const normalized = hex.replace("#", "");
  return [
    Number.parseInt(normalized.slice(0, 2), 16),
    Number.parseInt(normalized.slice(2, 4), 16),
    Number.parseInt(normalized.slice(4, 6), 16)
  ];
}

function configProblems() {
  const problems = [];
  state.segments.forEach((segment) => {
    const entry = segmentModule(segment);
    entry.fields
      .filter((field) => field.kind === "pairs")
      .forEach((field) => {
        if (segment.config[field.name].some(([from]) => from === "")) {
          problems.push(`${entry.label}: every ${fieldLabel(field.name).toLowerCase()} row needs a left side.`);
        }
      });
  });
  return problems;
}

function renderRon() {
  const problems = configProblems();
  elements.ronProblems.hidden = problems.length === 0;
  elements.ronProblems.textContent = problems.join(" ");
  elements.ronOutput.textContent = generateRon();
}

function generateRon() {
  const lines = [
    "(",
    `    separator: ${ronString(state.separator)},`,
    `    separator_color: ${colorToRon(state.separatorColor)},`,
    "    segments: ["
  ];
  state.segments.forEach((segment) => {
    lines.push(...segmentToRon(segment));
  });
  lines.push("    ],", ")");
  return `${lines.join("\n")}\n`;
}

function segmentToRon(segment) {
  const entry = segmentModule(segment);
  const fields = entry.fields.map((field) => [field.name, valueToRon(field, segment.config[field.name])]);

  return [
    `        ${segment.type}(`,
    ...fields.map(([key, value]) => `            ${key}: ${value},`),
    "        ),"
  ];
}

function valueToRon(field, value) {
  switch (field.kind) {
    case "color":
      return colorToRon(value);
    case "text":
      return ronString(value);
    case "bool":
      return value ? "true" : "false";
    case "integer":
      return String(Math.max(0, Math.round(Number(value) || 0)));
    case "float":
      return ronFloat(value);
    case "enum":
      return value;
    case "list":
      return `[${value.map(ronString).join(", ")}]`;
    case "pairs":
      return `[${value.map(([from, to]) => `(${ronString(from)}, ${ronString(to)})`).join(", ")}]`;
    default:
      throw new Error(`Cannot serialize field kind: ${field.kind}`);
  }
}

function colorToRon(color) {
  if (color.kind === "Named") {
    return `Named(${color.code})`;
  }
  if (color.kind === "Rgb") {
    const [r, g, b] = hexToRgb(color.hex);
    return `Rgb(${r}, ${g}, ${b})`;
  }
  return "Gradient";
}

function ronFloat(value) {
  const number = Number(value);
  return Number.isInteger(number) ? number.toFixed(1) : String(number);
}

function ronString(value) {
  const escaped = String(value)
    .replaceAll("\\", "\\\\")
    .replaceAll("\"", "\\\"")
    .replaceAll("\b", "\\b")
    .replaceAll("\f", "\\f")
    .replaceAll("\n", "\\n")
    .replaceAll("\r", "\\r")
    .replaceAll("\t", "\\t");
  return `"${escaped}"`;
}

function downloadRon() {
  if (lastDownloadUrl) {
    URL.revokeObjectURL(lastDownloadUrl);
  }
  lastDownloadUrl = URL.createObjectURL(
    new Blob([generateRon()], { type: "text/plain;charset=utf-8" })
  );
  const link = element("a");
  link.href = lastDownloadUrl;
  link.download = "config.ron";
  document.body.append(link);
  link.click();
  link.remove();
  announce("config.ron downloaded.");
}

async function createInstallCommand() {
  const ron = generateRon();
  elements.copyCommandButton.disabled = true;
  elements.sessionResult.hidden = true;
  elements.sessionError.hidden = true;

  try {
    const problems = configProblems();
    if (problems.length > 0) {
      throw new Error(problems.join(" "));
    }
    if (typeof CompressionStream !== "function") {
      throw new Error("This browser cannot compress the config. Download the RON instead.");
    }

    const code = await encodeConfigCode(ron);
    if (ron !== generateRon()) {
      throw new Error("The configuration changed while the command was being built. Build it again.");
    }

    elements.configSizeOutput.textContent = describeConfigSize(ron, code);
    elements.commandOutput.value = installCommand(code);
    elements.sessionResult.hidden = false;
    announce("Install command created.");
  } catch (error) {
    console.error("Could not build the install command.", error);
    elements.sessionError.textContent = error instanceof Error
      ? error.message
      : "Could not build the install command.";
    elements.sessionError.hidden = false;
    announce("Install command could not be created.");
  } finally {
    elements.copyCommandButton.disabled = false;
  }
}

async function encodeConfigCode(ron) {
  const compressed = new Blob([ron])
    .stream()
    .pipeThrough(new CompressionStream("gzip"));
  const bytes = new Uint8Array(await new Response(compressed).arrayBuffer());
  return base64Url(bytes);
}

function base64Url(bytes) {
  let binary = "";
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary)
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replaceAll("=", "");
}

function installCommand(code) {
  return `curl -fsSL ${installerUrl} | STATUSLINE_INSTALL_CONFIG=${code} sh`;
}

function describeConfigSize(ron, code) {
  const ronBytes = new TextEncoder().encode(ron).length;
  return `${ronBytes} B RON · ${code.length} char code`;
}

async function copyInstallCommand() {
  const command = elements.commandOutput.value;
  if (!command) {
    return;
  }

  if (navigator.clipboard && globalThis.isSecureContext) {
    try {
      await navigator.clipboard.writeText(command);
      setCopyButtonState();
      return;
    } catch (error) {
      console.error("Clipboard API failed.", error);
    }
  }

  elements.commandOutput.focus();
  elements.commandOutput.select();
  document.execCommand("copy");
  setCopyButtonState();
}

function setCopyButtonState() {
  elements.copyCommandButton.textContent = "Copied";
  announce("Install command copied.");
  elements.copyCommandButton.addEventListener(
    "blur",
    () => {
      elements.copyCommandButton.textContent = "Copy";
    },
    { once: true }
  );
}

function initializeElements() {
  const ids = [
    "catalogVersion",
    "scenarioSelect",
    "terminalWidth",
    "terminalWidthValue",
    "lineDimensions",
    "lineCanvas",
    "lineScreen",
    "hiddenPieces",
    "shelf",
    "inspectorKind",
    "inspectorTitle",
    "inspectorDescription",
    "removeSelectedButton",
    "inspectorControls",
    "resetButton",
    "ronButton",
    "installButton",
    "ronDialog",
    "ronProblems",
    "ronOutput",
    "downloadButton",
    "installDialog",
    "commandOutput",
    "copyCommandButton",
    "configSizeOutput",
    "sessionError",
    "sessionResult",
    "liveRegion"
  ];
  ids.forEach((id) => {
    elements[id] = document.getElementById(id);
  });
}

function initializeScenarioOptions() {
  elements.scenarioSelect.replaceChildren();
  scenarios.forEach((entry) => {
    const option = element("option", "", entry.label);
    option.value = entry.id;
    elements.scenarioSelect.append(option);
  });
  elements.scenarioSelect.value = state.scenarioId;
}

function bindStaticEvents() {
  elements.scenarioSelect.addEventListener("change", () => {
    state.scenarioId = elements.scenarioSelect.value;
    renderLine();
  });
  elements.terminalWidth.addEventListener("input", () => {
    state.terminalWidth = Number(elements.terminalWidth.value);
    renderLine();
  });
  elements.resetButton.addEventListener("click", () => {
    resetState();
    elements.scenarioSelect.value = state.scenarioId;
    elements.terminalWidth.value = String(state.terminalWidth);
    invalidateSession();
    renderAll();
    announce("Line reset to defaults.");
  });
  elements.ronButton.addEventListener("click", () => {
    renderRon();
    elements.ronDialog.showModal();
  });
  elements.downloadButton.addEventListener("click", downloadRon);
  elements.installButton.addEventListener("click", openInstallSheet);
  elements.copyCommandButton.addEventListener("click", copyInstallCommand);
  elements.removeSelectedButton.addEventListener("click", () => {
    const segment = selectedSegment();
    if (segment) {
      removeSegment(segment.id);
    }
  });

  elements.lineCanvas.addEventListener("click", (event) => {
    if (!event.target.closest(".piece, .sep")) {
      selectTarget(LINE_SELECTION);
    }
  });
  elements.lineScreen.addEventListener("dragover", handleLineDragOver);
  elements.lineScreen.addEventListener("dragleave", (event) => {
    if (!elements.lineScreen.contains(event.relatedTarget)) {
      clearDropIndicators();
    }
  });
  elements.lineScreen.addEventListener("drop", handleLineDrop);
  document.addEventListener("keydown", handleSelectionKey);

  elements.shelf.addEventListener("dragover", handleShelfDragOver);
  elements.shelf.addEventListener("dragleave", (event) => {
    if (!elements.shelf.contains(event.relatedTarget)) {
      elements.shelf.classList.remove("is-dropzone");
    }
  });
  elements.shelf.addEventListener("drop", handleShelfDrop);
}

function renderAll() {
  const previous = capturePositions();
  renderShelf();
  renderLine();
  renderInspector();
  playFlip(previous);
}

function renderDerived() {
  renderLine();
}

function renderLine() {
  const activeScenario = scenario();
  const drawn = state.segments.map((segment) => ({
    segment,
    pieces: previewSegment(segment, activeScenario),
    standalone: isStandalone(segment)
  }));
  const rendered = drawn.filter((entry) => entry.pieces && entry.pieces.length > 0);
  const maxColumns = runtimePreviewColumns(state.terminalWidth);

  renderHiddenPieces(drawn.filter((entry) => !entry.pieces || entry.pieces.length === 0));

  elements.lineCanvas.replaceChildren();
  elements.lineCanvas.style.maxWidth = `${maxColumns}ch`;

  if (rendered.length === 0) {
    elements.lineCanvas.append(
      element("span", "line-empty", "Nothing on the line yet — drag a piece up from below.")
    );
  } else {
    const rows = layoutRows(rendered, state.separator, maxColumns);
    let lastRow = null;
    rows.forEach((row) => {
      const rowNode = element("span", "line-row");
      if (row.standalone) {
        rowNode.classList.add("is-standalone");
      }
      row.cells.forEach((cell) => {
        if (cell.separator) {
          rowNode.append(createSeparatorNode());
        }
        rowNode.append(createPieceNode(cell.entry.segment, cell.entry.pieces));
      });
      elements.lineCanvas.append(rowNode);
      lastRow = rowNode;
    });
    lastRow.append(element("span", "line-cursor"));
  }

  elements.terminalWidthValue.textContent = String(state.terminalWidth);
  elements.lineDimensions.textContent = describeLineFill(rendered, state.separator, maxColumns);
}

function createPieceNode(segment, pieces) {
  const entry = segmentModule(segment);
  const node = element("button", "piece");
  node.type = "button";
  node.draggable = true;
  node.dataset.segmentId = segment.id;
  node.dataset.moduleId = segment.moduleId;
  node.title = entry.label;
  node.setAttribute("aria-label", `${entry.label} on the line`);
  if (segment.id === state.selectedId) {
    node.classList.add("is-selected");
  }
  if (pieces.every((part) => displayWidth(part.text) === 0)) {
    node.classList.add("is-blank");
  }

  pieces.forEach((part) => {
    const span = element("span", "", part.text);
    span.style.color = part.color;
    node.append(span);
  });

  node.addEventListener("click", () => selectTarget(segment.id));
  node.addEventListener("dragstart", (event) => {
    dragPayload = { kind: "line", id: segment.id };
    node.classList.add("is-dragging");
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", entry.label);
  });
  node.addEventListener("dragend", () => {
    dragPayload = null;
    node.classList.remove("is-dragging");
    clearDropIndicators();
  });

  return node;
}

function describeLineFill(entries, separator, maxColumns) {
  const rows = layoutRows(entries, separator, maxColumns);
  const separatorWidth = displayWidth(separator);
  const widest = rows.reduce((widestSoFar, row) => {
    const width = row.cells.reduce(
      (sum, cell) =>
        sum +
        (cell.separator ? separatorWidth : 0) +
        cell.entry.pieces.reduce((total, part) => total + displayWidth(part.text), 0),
      0
    );
    return Math.max(widestSoFar, width);
  }, 0);

  const count = rows.length === 1 ? "1 row" : `${rows.length} rows`;
  return `${widest} of ${maxColumns} cols, ${count}`;
}

function renderHiddenPieces(entries) {
  elements.hiddenPieces.replaceChildren();
  if (entries.length === 0) {
    return;
  }

  elements.hiddenPieces.append(element("span", "hidden-pieces-label", "Silent in this session:"));
  entries.forEach(({ segment }) => {
    const entry = segmentModule(segment);
    const node = element("button", "ghost-piece", entry.label);
    node.type = "button";
    node.draggable = true;
    node.dataset.segmentId = segment.id;
    node.dataset.moduleId = segment.moduleId;
    node.title = entry.description;
    if (segment.id === state.selectedId) {
      node.classList.add("is-selected");
    }
    node.addEventListener("click", () => selectTarget(segment.id));
    node.addEventListener("dragstart", (event) => {
      dragPayload = { kind: "line", id: segment.id };
      node.classList.add("is-dragging");
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", entry.label);
    });
    node.addEventListener("dragend", () => {
      dragPayload = null;
      node.classList.remove("is-dragging");
      clearDropIndicators();
    });
    elements.hiddenPieces.append(node);
  });
}

function createSeparatorNode() {
  const node = element("button", "sep", state.separator);
  node.type = "button";
  node.title = "Separator";
  node.setAttribute("aria-label", "Separator settings");
  node.style.color = resolveColor(state.separatorColor, 50);
  if (state.selectedId === LINE_SELECTION) {
    node.classList.add("is-selected");
  }
  node.addEventListener("click", () => selectTarget(LINE_SELECTION));
  return node;
}

function shelfModules() {
  const used = usedModuleIds();
  return modules.filter((entry) => entry.repeatable || !used.has(entry.id));
}

function renderShelf() {
  const available = shelfModules();
  elements.shelf.replaceChildren();

  if (available.length === 0) {
    elements.shelf.append(element("p", "shelf-empty", "Every piece is on the line."));
    return;
  }

  available.forEach((entry) => {
    const node = element("button", "shelf-piece");
    node.type = "button";
    node.draggable = true;
    node.dataset.moduleId = entry.id;
    node.title = entry.description;
    node.append(
      element("span", "shelf-piece-name", entry.label),
      element("span", "shelf-piece-pitch", entry.description)
    );
    node.addEventListener("click", () => addSegment(entry.id));
    node.addEventListener("dragstart", (event) => {
      dragPayload = { kind: "shelf", moduleId: entry.id };
      node.classList.add("is-dragging");
      event.dataTransfer.effectAllowed = "copy";
      event.dataTransfer.setData("text/plain", entry.label);
    });
    node.addEventListener("dragend", () => {
      dragPayload = null;
      node.classList.remove("is-dragging");
      clearDropIndicators();
    });
    elements.shelf.append(node);
  });
}

function addSegment(moduleId, index = state.segments.length) {
  const entry = moduleEntry(moduleId);
  if (!entry.repeatable && usedModuleIds().has(moduleId)) {
    return;
  }
  const segment = createSegment(moduleId);
  const target = Math.max(0, Math.min(index, state.segments.length));
  state.segments.splice(target, 0, segment);
  state.selectedId = segment.id;
  invalidateSession();
  renderAll();
  announce(`${entry.label} put on the line at position ${target + 1}.`);
  focusPiece(segment.id);
}

function removeSegment(id) {
  const index = state.segments.findIndex((segment) => segment.id === id);
  if (index < 0) {
    return;
  }

  const [removed] = state.segments.splice(index, 1);
  if (state.selectedId === id) {
    state.selectedId = LINE_SELECTION;
  }
  invalidateSession();
  renderAll();
  announce(`${moduleEntry(removed.moduleId).label} taken off the line.`);
}

function moveSegment(id, delta) {
  const from = state.segments.findIndex((segment) => segment.id === id);
  const to = from + delta;
  if (from < 0 || to < 0 || to >= state.segments.length) {
    return;
  }
  const [segment] = state.segments.splice(from, 1);
  state.segments.splice(to, 0, segment);
  invalidateSession();
  renderLine();
  announce(`${segmentModule(segment).label} moved to position ${to + 1}.`);
  focusPiece(id);
}

function moveSegmentAlongLine(id, direction) {
  const visibleIds = [...elements.lineCanvas.querySelectorAll(".piece")].map(
    (node) => node.dataset.segmentId
  );
  const position = visibleIds.indexOf(id);
  if (position < 0) {
    moveSegment(id, direction);
    return;
  }
  if (position + direction < 0 || position + direction >= visibleIds.length) {
    return;
  }
  const others = visibleIds.filter((visibleId) => visibleId !== id);
  const leftPosition = direction > 0 ? position : position - 2;
  const segment = state.segments.find((candidate) => candidate.id === id);
  moveSegmentToIndex(id, indexAfterNeighbour(state.segments, others, leftPosition, isStandalone(segment)));
}

function indexAfterNeighbour(segments, visibleIds, leftPosition, standalone) {
  for (let position = leftPosition; position >= 0; position -= 1) {
    const neighbour = segments.find((segment) => segment.id === visibleIds[position]);
    if (neighbour && isStandalone(neighbour) === standalone) {
      return segments.indexOf(neighbour) + 1;
    }
  }
  return 0;
}

function moveSegmentToIndex(id, requestedIndex) {
  const from = state.segments.findIndex((segment) => segment.id === id);
  if (from < 0) {
    return;
  }
  let to = Math.max(0, Math.min(requestedIndex, state.segments.length));
  const [segment] = state.segments.splice(from, 1);
  if (from < to) {
    to -= 1;
  }
  if (from === to) {
    state.segments.splice(to, 0, segment);
    renderLine();
    return;
  }
  state.segments.splice(to, 0, segment);
  state.selectedId = id;
  invalidateSession();
  renderLine();
  renderInspector();
  announce(`${segmentModule(segment).label} moved to position ${to + 1}.`);
  focusPiece(id);
}

function focusPiece(id) {
  requestAnimationFrame(() => {
    const node = elements.lineCanvas.querySelector(`[data-segment-id="${CSS.escape(id)}"]`);
    if (node) {
      node.focus();
    }
  });
}

function selectTarget(target) {
  if (state.selectedId === target) {
    return;
  }
  state.selectedId = target;
  document.querySelectorAll(".piece, .ghost-piece").forEach((node) => {
    node.classList.toggle("is-selected", node.dataset.segmentId === target);
  });
  elements.lineCanvas.querySelectorAll(".sep").forEach((node) => {
    node.classList.toggle("is-selected", target === LINE_SELECTION);
  });
  renderInspector();
}

function renderInspector() {
  elements.inspectorControls.replaceChildren();
  const segment = selectedSegment();

  if (!segment) {
    elements.inspectorKind.textContent = "Separator";
    elements.inspectorTitle.textContent = "Whole line";
    elements.inspectorDescription.textContent = "What sits between every piece.";
    elements.removeSelectedButton.hidden = true;
    elements.inspectorControls.append(
      createTextControl("Separator", state.separator, (value) => {
        state.separator = value;
        configMutated();
      }, 12),
      createColorControl(
        "Separator color",
        state.separatorColor,
        (value, rerender) => {
          state.separatorColor = value;
          configMutated();
          if (rerender) {
            renderInspector();
          }
        },
        false
      )
    );
    return;
  }

  const entry = segmentModule(segment);
  elements.inspectorKind.textContent = entry.type;
  elements.inspectorTitle.textContent = entry.label;
  elements.inspectorDescription.textContent = entry.description;
  elements.removeSelectedButton.hidden = false;

  entry.fields.forEach((field) => {
    const control = createControl(field, entry.type, segment.config[field.name], (value, rerender) => {
      segment.config[field.name] = value;
      configMutated();
      if (rerender) {
        renderInspector();
      }
    });
    elements.inspectorControls.append(control);
  });
}

function handleSelectionKey(event) {
  if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) {
    return;
  }
  const target = event.target instanceof Element ? event.target : null;
  if (!target || target.closest("input, textarea, select, dialog, [contenteditable]")) {
    return;
  }
  const segment = selectedSegment();
  if (!segment) {
    return;
  }

  if (event.key === "ArrowLeft") {
    event.preventDefault();
    moveSegmentAlongLine(segment.id, -1);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    moveSegmentAlongLine(segment.id, 1);
  } else if (event.key === "Delete" || event.key === "Backspace") {
    event.preventDefault();
    removeSegment(segment.id);
  }
}

function handleLineDragOver(event) {
  if (!dragPayload) {
    return;
  }
  event.preventDefault();
  event.dataTransfer.dropEffect = dragPayload.kind === "shelf" ? "copy" : "move";
  clearDropIndicators();
  elements.lineCanvas.classList.add("is-dropzone");
  const slot = lineDropSlot(event);
  if (!slot) {
    return;
  }
  slot.node.classList.add(slot.before ? "drop-before" : "drop-after");
}

function visiblePieceNodes() {
  const draggedId = dragPayload && dragPayload.kind === "line" ? dragPayload.id : null;
  return [...elements.lineCanvas.querySelectorAll(".piece")].filter(
    (node) => node.dataset.segmentId !== draggedId
  );
}

function lineDropSlot(event) {
  const boxes = visiblePieceNodes().map((node) => ({ node, rect: node.getBoundingClientRect() }));
  return nearestDropSlot(boxes, event.clientX, event.clientY);
}

function nearestDropSlot(boxes, x, y) {
  if (boxes.length === 0) {
    return null;
  }
  const rowDistance = (box) => {
    if (y < box.rect.top) {
      return box.rect.top - y;
    }
    if (y > box.rect.bottom) {
      return y - box.rect.bottom;
    }
    return 0;
  };
  const closestRow = Math.min(...boxes.map(rowDistance));
  const row = boxes.filter((box) => rowDistance(box) === closestRow);
  const centre = (box) => box.rect.left + (box.rect.right - box.rect.left) / 2;
  const nearest = row.reduce((best, box) =>
    Math.abs(centre(box) - x) < Math.abs(centre(best) - x) ? box : best
  );
  return { node: nearest.node, before: x < centre(nearest) };
}

function lineDropIndex(event, standalone) {
  const slot = lineDropSlot(event);
  if (!slot) {
    return state.segments.length;
  }
  const visibleIds = visiblePieceNodes().map((node) => node.dataset.segmentId);
  const slotPosition = visibleIds.indexOf(slot.node.dataset.segmentId);
  const leftPosition = slot.before ? slotPosition - 1 : slotPosition;
  return indexAfterNeighbour(state.segments, visibleIds, leftPosition, standalone);
}

function handleLineDrop(event) {
  if (!dragPayload) {
    return;
  }
  event.preventDefault();
  const payload = dragPayload;
  const dropped =
    payload.kind === "shelf"
      ? createSegment(payload.moduleId)
      : state.segments.find((segment) => segment.id === payload.id);
  const index = lineDropIndex(event, isStandalone(dropped));
  dragPayload = null;
  clearDropIndicators();

  if (payload.kind === "shelf") {
    addSegment(payload.moduleId, index);
  } else {
    moveSegmentToIndex(payload.id, index);
  }
}

function handleShelfDragOver(event) {
  if (!dragPayload || dragPayload.kind !== "line") {
    return;
  }
  event.preventDefault();
  event.dataTransfer.dropEffect = "move";
  elements.shelf.classList.add("is-dropzone");
}

function handleShelfDrop(event) {
  if (!dragPayload || dragPayload.kind !== "line") {
    return;
  }
  event.preventDefault();
  const payload = dragPayload;
  dragPayload = null;
  clearDropIndicators();
  removeSegment(payload.id);
}

function clearDropIndicators() {
  elements.lineCanvas.classList.remove("is-dropzone");
  elements.shelf.classList.remove("is-dropzone");
  elements.lineCanvas.querySelectorAll(".drop-before, .drop-after").forEach((node) => {
    node.classList.remove("drop-before", "drop-after");
  });
}

function capturePositions() {
  const positions = new Map();
  if (!elements.lineCanvas) {
    return positions;
  }
  document.querySelectorAll(".piece, .ghost-piece, .shelf-piece").forEach((node) => {
    positions.set(node.dataset.moduleId, node.getBoundingClientRect());
  });
  return positions;
}

function playFlip(positions) {
  if (positions.size === 0 || motionIsReduced()) {
    return;
  }
  document.querySelectorAll(".piece, .ghost-piece, .shelf-piece").forEach((node) => {
    const before = positions.get(node.dataset.moduleId);
    if (!before) {
      return;
    }
    const after = node.getBoundingClientRect();
    const dx = before.left - after.left;
    const dy = before.top - after.top;
    if (Math.abs(dx) < 1 && Math.abs(dy) < 1) {
      return;
    }
    node.animate(
      [{ transform: `translate(${dx}px, ${dy}px)` }, { transform: "none" }],
      { duration: 260, easing: "cubic-bezier(0.2, 0.85, 0.25, 1)" }
    );
  });
}

function motionIsReduced() {
  return globalThis.matchMedia
    ? globalThis.matchMedia("(prefers-reduced-motion: reduce)").matches
    : false;
}

async function openInstallSheet() {
  elements.installDialog.showModal();
  await createInstallCommand();
}

async function loadCatalog() {
  const response = await fetch(catalogUrl, { cache: "no-cache" });
  if (!response.ok) {
    throw new Error(`${catalogUrl}: HTTP ${response.status}`);
  }
  return validateCatalog(await response.json());
}

function showCatalogError(error) {
  const message = error instanceof Error ? error.message : String(error);
  elements.shelf.replaceChildren(
    element(
      "p",
      "shelf-empty",
      `The segment catalogue could not be loaded (${message}). Generate it with: cargo run -- --schema > site/segment-catalog.json`
    )
  );
  elements.installButton.disabled = true;
  elements.ronButton.disabled = true;
  elements.resetButton.disabled = true;
}

async function initialize() {
  initializeElements();
  initializeScenarioOptions();
  bindStaticEvents();
  elements.terminalWidth.value = String(state.terminalWidth);

  let catalog;
  try {
    catalog = await loadCatalog();
  } catch (error) {
    console.error("Could not load the segment catalogue.", error);
    showCatalogError(error);
    return;
  }

  installCatalog(catalog);
  elements.catalogVersion.textContent = `v${catalogVersion}`;
  resetState();
  renderAll();
}

if (typeof document !== "undefined") {
  initialize();
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = {
    buildModules,
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
  };
}
