---
name: statusline-config
description: Edit the statusline RON config (config/default.ron or config/local.ron), then rebuild and install the binary so Claude Code picks up the change on the next render. Use when the user wants to add/remove/reorder segments, recolour them, change separator, switch RateLimit style, show Linux Claude CPU and RSS memory usage, toggle git options, or otherwise tweak the statusline appearance.
---

# statusline-config

Skill for editing the statusline-rs config in this repo and reinstalling the binary so the change takes effect.

## When to use

Trigger on requests like:

- "change the branch colour", "recolour the bar", "make 5h radial"
- "add UserIdleTime", "drop PromptCacheTtl", "reorder the segments"
- "show Claude CPU and RSS usage", "show Claude CPU and RAM usage"
- "add an empty line under the statusline"
- "change the separator", "use truecolor instead of ansi"
- "make local config like default but without 7d"
- "rebuild / reinstall the statusline"

## Files

- `config/default.ron` — public config, committed to git, embedded into the prebuilt release binary at build time.
- `config/local.ron` — personal override, **gitignored**. If present, `install-local.sh` builds with this file instead of `default.ron`.
- `build.rs` — picks the config path from `STATUSLINE_CONFIG` env var (relative to manifest dir) or falls back to `config/default.ron`, then writes it to `$OUT_DIR/embedded_config.ron`.
- `src/config_schema.rs` — the schema: `RootConfig`, the `SegmentSpec` enum, and one struct per segment.
- `src/config.rs` — maps each `SegmentSpec` variant to its segment implementation.
- `install-local.sh` — `cargo build --release`, `--check-config` on the selected file, copy binary to `$HOME/.claude/bin/statusline`, copy the selected file to `$HOME/.claude/statusline/config.ron` (the runtime config the binary loads before falling back to the embedded one). Auto-selects `config/local.ron` if present.

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

### Segments

**Read `src/config_schema.rs` before writing any segment block — it is the only catalogue.** One struct there is one segment: its name is the RON variant, its fields are the options (all required unless the field carries a `#[serde(default)]`), and the single `///` line above it says what the segment shows. Do not keep a copy of that list in this file — it goes stale, and a stale name makes `build.rs` reject the config.

`config/default.ron` is a working example of most segments; `config/local.ron`, when it exists, shows what the user actually runs.

### Color

- `Named(code)` — ANSI 30–37 (fg) or 90–97 (bright fg). Common: 31 red, 32 green, 33 yellow, 90 bright-black/grey, 91 bright-red.
- `Rgb(r, g, b)` — truecolor, 0–255 each.
- `Gradient` — only meaningful on `ContextUsage`, `PromptCacheTtl`, and `RateLimit` with `color_mode: Gradient`. On other segments treat as plain (no effect).

Segment order in the vec controls render order. Empty `segments: []` renders nothing.

## Workflow

1. Confirm target file (default `config/local.ron`; switch to `default.ron` only if the user signals "for the repo").
2. Read the target file; if it doesn't exist and the target is `local.ron`, copy `default.ron` first.
3. Read the segment's struct in `src/config_schema.rs` and write every field it declares.
4. Make the edit. Keep RON formatting consistent with the rest of the file (4-space indent, trailing commas, tagged variants like `Named(32)` / `Rgb(r,g,b)`).
5. Run `./install-local.sh` from the project root. This rebuilds, validates the file, copies the binary to `~/.claude/bin/statusline`, and installs the file as `~/.claude/statusline/config.ron`.
6. Report what changed in one line.

Do not invoke `cargo build` directly — `install-local.sh` already does the right thing (picks `local.ron` if present, copies binary into place).

If the user only wants to preview / not install yet, skip step 5 and say so.

## Validation tips

- RON is strict: every required field of a segment variant must be present. Missing field → build panics in `build.rs` with `ron::de::SpannedError`.
- Don't invent segment names — only the variants declared in `SegmentSpec` exist.
- `RateLimit.prefix` replaces a `{t}` token with the time left until reset, in the window's unit, 1 decimal — `"{t}d "` renders `6.9d`..`0.0d`, and `?` when `resets_at` is missing.
- `gradient_midpoint_percentage` defaults to `50.0` when omitted and must be greater than `0` and less than `100`.
- `Gradient` on a segment that doesn't support it won't crash but renders as plain text — prefer `Named` or `Rgb` there.
- `ClaudeResourceUsage` is Linux-only and requires a matching live `session_id` entry in Claude Code's local session registry. It emits nothing when the process cannot be resolved or on macOS and Windows. CPU is shown in logical-core equivalents (`1.00c` is one fully used core), RSS is the summed resident set size of the Claude process tree in MiB, and the first CPU sample is `—`.
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

**Two empty lines under the status line:** add two `Spacer(shape: BlankLine)` blocks, install.

**Start the next segments on a new line without an empty one:** add `Spacer(shape: LineBreak)` where the line should end, install.

**Switch 5h to radial with percent:**

```ron
RateLimit(
    window: FiveHour,
    style: RadialPercent,
    fill: Used,
    color_mode: Steps,
    gradient_midpoint_percentage: 50.0,
    prefix: "5h ",
    low_color: Named(32),
    mid_color: Named(33),
    high_color: Named(31),
)
```
