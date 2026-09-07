import {
  colorToRgb,
  interpolateColorStops,
  neutralGrey,
  resolveCacheTtlColor,
  resolveColor,
  resolveContextColor
} from "./color.js";
import { cut } from "./line-layout.js";

export function previewSegment(segment, session, separatorColor = neutralGrey()) {
  const config = segment.config;
  switch (segment.type) {
    case "Model":
      return previewModel(config, session);
    case "Effort":
      return [piece(`${config.prefix}${session.effort}`, resolveColor(config.color, 45))];
    case "ContextUsage":
      return [
        piece(config.prefix, resolveColor(config.prefix_color, session.context)),
        piece(`${Math.round(session.context)}%`, resolveContextColor(config.color, session.context)),
        piece(config.suffix, resolveColor(config.suffix_color, session.context))
      ];
    case "PromptCacheTtl": {
      const cacheView = cacheTtlPreview(session);
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
          `${config.cpu_prefix}${session.cpu} ${config.memory_prefix}${session.rss} MiB`,
          resolveColor(config.color, 46)
        )
      ];
    case "Cwd":
      return [piece(session.cwd, resolveColor(config.color, 45))];
    case "GitBranch":
      return previewGitBranch(config, session);
    case "GitDiff":
      return previewGitDiff(config, session, separatorColor);
    case "GitError":
      return session.git ? null : [piece(config.text, resolveColor(config.color, 90))];
    case "UserIdleTime":
      if (session.idleSeconds < config.threshold_seconds) {
        return null;
      }
      return [
        piece(
          `${config.prefix}${formatDuration(session.idleSeconds)}`,
          resolveColor(config.color, 48)
        )
      ];
    case "RateLimit":
      return previewRateLimit(config, session);
    case "SubagentStats":
      return previewSubagentStats(config, session);
    case "Reminder":
      return previewText(config, cut(session.reminders.join(config.separator), config.max_chars));
    case "SessionNotice":
      return previewSessionNotice(config, session);
    case "MyLastPrompt":
      return previewText(config, cut(session.lastPrompt, config.max_chars));
    case "LlmAnswer":
      return previewText(config, cut(session.llmAnswer, config.max_chars));
    case "LlmInsight":
      return previewText(config, cut(session.llmInsight, config.max_chars));
    case "Weather":
      return previewText(config, cut(session.weather, config.max_chars));
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

function previewModel(config, session) {
  const name = (config.replacements || []).reduce(
    (current, [from, to]) => (from ? current.replaceAll(from, to) : current),
    session.model
  );
  if (!name) {
    return null;
  }
  return [piece(`${config.prefix}${name}`, resolveColor(config.color, 40))];
}

function previewSessionNotice(config, session) {
  const notice = session.notice;
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

function previewSubagentStats(config, session) {
  const stats = session.agents;
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

export function previewGitBranch(config, session) {
  if (!session.git) {
    return null;
  }
  let branch = "";
  if (config.show_worktree && session.worktree) {
    branch += "⑂";
  }
  branch += session.branch;
  const pieces = [piece(branch, resolveColor(config.color, 35))];

  if (config.show_state && session.state) {
    pieces.push(piece(` [${session.state}]`, resolveColor(config.state_color, 90)));
  }

  if (config.show_ahead_behind) {
    const movements = [];
    if (session.ahead > 0) {
      movements.push(`↑${session.ahead}`);
    }
    if (session.behind > 0) {
      movements.push(`↓${session.behind}`);
    }
    if (movements.length > 0) {
      pieces.push(piece(`(${movements.join(" ")})`, resolveColor(config.color, 35)));
    }
  }

  return pieces;
}

function previewGitDiff(config, session, separatorColor) {
  if (!session.git) {
    return null;
  }
  const pieces = [];
  if (session.diff.modified > 0) {
    pieces.push(piece(`~${session.diff.modified}`, resolveColor(config.modified_color, 55)));
  }
  if (session.diff.untracked > 0) {
    if (pieces.length > 0) {
      pieces.push(piece(" ", resolveColor(separatorColor, 50)));
    }
    pieces.push(piece(`+${session.diff.untracked}`, resolveColor(config.untracked_color, 35)));
  }
  if (session.diff.deleted > 0) {
    if (pieces.length > 0) {
      pieces.push(piece(" ", resolveColor(separatorColor, 50)));
    }
    pieces.push(piece(`-${session.diff.deleted}`, resolveColor(config.deleted_color, 90)));
  }
  return pieces.length > 0 ? pieces : null;
}

function previewRateLimit(config, session) {
  const data = session.rateLimits[config.window];
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

export function cacheTtlPreview(session) {
  const ttlSeconds = Math.max(1, Math.floor(session.cacheTtlSeconds));
  const remainingSeconds = Math.floor(session.cacheRemainingSeconds);
  if (remainingSeconds <= 0) {
    return {
      cold: true,
      percentage: session.context,
      text: "cold"
    };
  }

  return {
    cold: false,
    percentage: ((ttlSeconds - remainingSeconds) / ttlSeconds) * 100,
    text: formatDurationPadded(remainingSeconds)
  };
}
