import { gradientColor, namedColor, neutralGrey, rgbColor } from "./color.js";
import { buildModules } from "./segment-catalog.js";
import { scenarios } from "./preview-sessions.js";

export const LINE_SELECTION = "line";

export const presets = {
  Model: { color: rgbColor(180, 142, 173) },
  Effort: { color: neutralGrey() },
  ContextUsage: {
    color: gradientColor(),
    prefix_color: rgbColor(180, 142, 173),
    suffix_color: rgbColor(180, 142, 173)
  },
  PromptCacheTtl: { color: gradientColor(), prefix: "cache " },
  RateLimit: {
    style: "Percent",
    fill: "Remaining",
    color_mode: "Gradient",
    gradient_midpoint_percentage: 50,
    usage_ttl_seconds: 300,
    low_color: rgbColor(103, 175, 103),
    mid_color: rgbColor(195, 179, 100),
    high_color: rgbColor(220, 60, 60)
  },
  SubagentStats: {
    color: neutralGrey(),
    active_color: rgbColor(150, 200, 100),
    stall_color: rgbColor(220, 60, 60),
    prefix: "Sub-agents:",
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
  MyLastPrompt: { color: neutralGrey(), prefix: "» ", max_chars: 48, standalone: true },
  ClaudeResourceUsage: { color: neutralGrey(), cpu_prefix: "CPU ", memory_prefix: "RSS " },
  UserIdleTime: { color: neutralGrey(), prefix: "idle " },
  Spacer: { standalone: true },
  LlmAnswer: {
    color: neutralGrey(),
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
    prompt: "In one sentence: what the user's goal is and what is being done for it right now. Only the sentence, no quotes or explanations.",
    every_turns: 2,
    scan_whole_session: true,
    initial_scan_bytes: 262144,
    context_chars: 12000,
    max_chars: 128,
    standalone: true
  },
  Weather: { color: neutralGrey(), format: "%c+%t", ttl_seconds: 1800, max_chars: 32 }
};

export const defaultLine = [
  "Model",
  "RateLimit:Fable",
  "ContextUsage",
  "Effort",
  "PromptCacheTtl",
  "RateLimit:FiveHour",
  "RateLimit:SevenDay",
  "SubagentStats",
  "Cwd",
  "GitBranch",
  "GitDiff",
  "MyLastPrompt",
  "GitError",
  "LlmInsight",
  {
    module: "LlmInsight",
    config: {
      color: rgbColor(170, 160, 120),
      prefix: "💡 ",
      prompt: "In one sentence: what should be added to the user's last prompt to make the task more precise. Do not suggest what is already done. Only the suggestion, no quotes or explanations."
    }
  }
];

const alwaysStandalone = new Set(["LlmAnswer"]);

export const state = {
  separator: " ",
  separatorColor: neutralGrey(),
  terminalWidth: 120,
  scenarioId: "active",
  segments: [],
  selectedId: LINE_SELECTION
};

let idSequence = 0;
let modules = [];
let catalogVersion = "";

export function installCatalog(catalog) {
  modules = buildModules(catalog);
  catalogVersion = catalog.version;
  return modules;
}

export function installedCatalogVersion() {
  return catalogVersion;
}

export function moduleEntry(moduleId) {
  const entry = modules.find((candidate) => candidate.id === moduleId);
  if (!entry) {
    throw new Error(`Unknown module: ${moduleId}`);
  }
  return entry;
}

export function segmentModule(segment) {
  return moduleEntry(segment.moduleId);
}

export function selectedSegment() {
  return state.segments.find((segment) => segment.id === state.selectedId) || null;
}

export function usedModuleIds() {
  return new Set(state.segments.map((segment) => segment.moduleId));
}

export function activeScenario() {
  return scenarios.find((entry) => entry.id === state.scenarioId) || scenarios[0];
}

export function isStandalone(segment) {
  return alwaysStandalone.has(segment.type) || segment.config.standalone === true;
}

export function shelfModules() {
  const used = usedModuleIds();
  return modules.filter((entry) => entry.repeatable || !used.has(entry.id));
}

export function createSegment(moduleId) {
  const entry = moduleEntry(moduleId);
  return {
    id: nextId(),
    moduleId,
    type: entry.type,
    config: segmentDefaults(entry)
  };
}

export function resetState() {
  state.separator = " ";
  state.separatorColor = neutralGrey();
  state.terminalWidth = 120;
  state.scenarioId = "active";
  state.segments = buildDefaultSegments();
  state.selectedId = LINE_SELECTION;
}

function nextId() {
  idSequence += 1;
  if (globalThis.crypto && typeof globalThis.crypto.randomUUID === "function") {
    return `segment-${globalThis.crypto.randomUUID()}`;
  }
  return `segment-${Date.now()}-${idSequence}`;
}

function fieldDefault(field) {
  switch (field.kind) {
    case "color":
      return neutralGrey();
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

function buildDefaultSegments() {
  return defaultLine.map((entry) => {
    if (typeof entry === "string") {
      return createSegment(entry);
    }
    const segment = createSegment(entry.module);
    Object.assign(segment.config, structuredClone(entry.config));
    return segment;
  });
}
