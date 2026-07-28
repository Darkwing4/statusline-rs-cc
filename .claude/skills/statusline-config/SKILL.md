---
name: statusline-config
description: Edit the statusline RON config (config/default.ron or config/local.ron), then rebuild and install the binary so Claude Code picks up the change on the next render. Use when the user wants to add/remove/reorder segments, recolour them, change separator, switch RateLimit style, show Linux Claude CPU/RAM usage, toggle git options, or otherwise tweak the statusline appearance.
---

# statusline-config

Skill for editing the statusline-rs config in this repo and reinstalling the binary so the change takes effect.

## When to use

Trigger on requests like:

- "change the branch colour", "recolour the bar", "make 5h radial"
- "add IdleTime", "drop CacheTtl", "reorder the segments"
- "show Claude CPU and RAM usage"
- "change the separator", "use truecolor instead of ansi"
- "make local config like default but without 7d"
- "rebuild / reinstall the statusline"

## Files

- `config/default.ron` — public config, committed to git, embedded into the prebuilt release binary at build time.
- `config/local.ron` — personal override, **gitignored**. If present, `install-local.sh` builds with this file instead of `default.ron`.
- `build.rs` — picks the config path from `STATUSLINE_CONFIG` env var (relative to manifest dir) or falls back to `config/default.ron`, then writes it to `$OUT_DIR/embedded_config.ron`.
- `src/config.rs` — `RootConfig` + `SegmentSpec` enum (the schema for the RON file).
- `install-local.sh` — `cargo build --release` + copy binary to `$HOME/.claude/bin/statusline`. Auto-selects `config/local.ron` if present.

**Default target:** edit `config/local.ron` for personal tweaks (so `default.ron` stays the published baseline). Edit `default.ron` only when the user explicitly says it's "for the repo" / "for everyone" / "to commit".

If `local.ron` doesn't exist and the user wants a personal tweak, copy `default.ron` to `local.ron` first, then edit.

## Schema

Top level:

```ron
(
    separator: " ",
    separator_color: <Color>,
    segments: [ <SegmentSpec>, ... ],
)
```

### Color

- `Named(code)` — ANSI 30–37 (fg) or 90–97 (bright fg). Common: 31 red, 32 green, 33 yellow, 90 bright-black/grey, 91 bright-red.
- `Rgb(r, g, b)` — truecolor, 0–255 each.
- `Gradient` — only meaningful on `Context`, `CacheTtl`, and `RateLimit` with `color_mode: Gradient`. On other segments treat as plain (no effect).

### Segments

Every segment is a tagged tuple. All fields are required (no defaults in Deserialize) — do not omit fields.

```ron
Context(
    color: Gradient,
    prefix: "",
    prefix_color: Rgb(180, 142, 173),
    suffix: "",
    suffix_color: Rgb(180, 142, 173),
)

CacheTtl(
    color: Gradient,
    prefix: "cache ",
)

ClaudeResourceUsage(
    color: Named(90),
    cpu_prefix: "CPU ",
    memory_prefix: "RAM ",
)

Cwd(
    color: Rgb(95, 175, 175),
)

GitBranch(
    color: Named(32),
    state_color: Named(91),
    show_worktree: true,
    show_ahead_behind: true,
    show_state: true,
)

GitDiff(
    modified_color: Named(33),
    untracked_color: Named(32),
    deleted_color: Named(31),
)

GitError(
    color: Named(91),
    prefix: "git: ",
)

IdleTime(
    color: Named(90),
    prefix: "idle ",
    threshold_seconds: 0,
)

RateLimit(
    window: FiveHour,        // FiveHour | SevenDay
    style: Bar,              // Percent | Bar | BarPercent | Radial | RadialPercent
    fill: Remaining,         // Used | Remaining
    color_mode: Gradient,    // Steps | Gradient
    prefix: "5h ",           // a "{t}" token is replaced by time left until reset, in the window's unit (h for FiveHour, d for SevenDay), 1 decimal — e.g. "{t}d " shows 6.9d..0.5d..0.0d; falls back to nominal 5.0/7.0 if resets_at is missing
    low_color: Rgb(103, 175, 103),
    mid_color: Rgb(195, 179, 100),
    high_color: Rgb(220, 60, 60),
)
```

Segment order in the vec controls render order. Empty `segments: []` renders nothing.

## Workflow

1. Confirm target file (default `config/local.ron`; switch to `default.ron` only if the user signals "for the repo").
2. Read the target file; if it doesn't exist and the target is `local.ron`, copy `default.ron` first.
3. Make the edit. Keep RON formatting consistent with the rest of the file (4-space indent, trailing commas, tagged variants like `Named(32)` / `Rgb(r,g,b)`).
4. Run `./install-local.sh` from the project root. This rebuilds and copies the binary to `~/.claude/bin/statusline`.
5. Report what changed in one line.

Do not invoke `cargo build` directly — `install-local.sh` already does the right thing (picks `local.ron` if present, copies binary into place).

If the user only wants to preview / not install yet, skip step 4 and say so.

## Validation tips

- RON is strict: every field of a segment variant must be present. Missing field → build panics in `build.rs` with `ron::de::SpannedError`.
- Don't introduce unknown segment names — only the variants listed above exist in `SegmentSpec`.
- `Gradient` on `ClaudeResourceUsage` / `Cwd` / `GitBranch` / `GitDiff` / `GitError` / `IdleTime` won't crash but will render as plain text (no colour) — prefer `Named` or `Rgb` there.
- `ClaudeResourceUsage` is Linux-only and requires a matching live `session_id` entry in Claude Code's local session registry. It emits nothing when the process cannot be resolved or on macOS and Windows. CPU `100%` means one fully used core, RAM is the summed RSS of the Claude process tree, and the first CPU sample is `—`.
- Set `statusLine.refreshInterval` to `1` in Claude Code settings for periodic live resource updates.
- `RateLimit` only renders after the first response in a Claude.ai session; absent on API plans. Don't expect it to appear immediately in a fresh transcript.

## Examples

**Recolour the cwd to lavender, install:**

```ron
Cwd(
    color: Rgb(180, 142, 173),
)
```

then `./install-local.sh`.

**Drop the 7-day rate limit segment:** delete the second `RateLimit(...)` block from `segments: [...]`, install.

**Switch 5h to radial with percent:**

```ron
RateLimit(
    window: FiveHour,
    style: RadialPercent,
    fill: Used,
    color_mode: Steps,
    prefix: "5h ",
    low_color: Named(32),
    mid_color: Named(33),
    high_color: Named(31),
)
```
