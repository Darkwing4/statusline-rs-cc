# statusline

The fastest status line for Claude Code: one Rust binary, about 5 ms per render, no runtime dependencies. Assemble the line in the browser and install it with one command, or write the config by hand.

<table>
  <tr>
    <td><img src="docs/screenshots/hero.png" alt="default look"/></td>
    <td><img src="docs/screenshots/states.png" alt="worktree + rebase"/></td>
  </tr>
</table>

## install

Linux / macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.sh | sh
```

Windows (PowerShell):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.ps1 | iex"
```

The installer downloads the prebuilt binary for your platform, puts it in `~/.claude/bin/statusline` (`%USERPROFILE%\.claude\bin\statusline.exe` on Windows), and points `settings.json` at it. Prebuilt for Linux, macOS, and Windows on x86_64 and aarch64.

<details>
<summary>env vars, settings patch, build from source</summary>

| var | default |
|---|---|
| `STATUSLINE_TAG` | `latest` |
| `STATUSLINE_INSTALL_DIR` | `$HOME/.claude/bin` |
| `STATUSLINE_SETTINGS` | `$HOME/.claude/settings.json` |
| `STATUSLINE_SKIP_SETTINGS` | unset — set to `1` to skip the JSON patch |
| `STATUSLINE_REPO` | `Darkwing4/statusline-rs-cc` |
| `STATUSLINE_INSTALL_CONFIG` | unset — a config code from the [builder](https://darkwing4.github.io/statusline-rs-cc/): decoded, checked with the downloaded binary, written as the runtime config |
| `STATUSLINE_CONFIG_PATH` | `$HOME/.claude/statusline/config.ron` |

The settings patch is non-destructive: preserves every other key in `settings.json`, writes a `.bak` next to the original, no-ops if already pointed at the binary. If `python3` is missing it skips the patch and prints the snippet to paste manually.

Build from source:

```sh
cargo build --release
cp target/release/statusline ~/.claude/bin/statusline
```

</details>

## build your line

Open the [statusline builder](https://darkwing4.github.io/statusline-rs-cc/), drag the pieces where you want them, tune the selected one, and copy the install command. That command carries your exact config: the installer decodes it, checks it with the freshly downloaded binary, and writes it to `~/.claude/statusline/config.ron`. On Windows download the RON from the page and put it at `%USERPROFILE%\.claude\statusline\config.ron`.

Or write it by hand. The config is a [RON](https://github.com/ron-rs/ron) file; the binary reads `~/.claude/statusline/config.ron` when it exists and falls back to the embedded [`config/default.ron`](config/default.ron). `statusline --check-config <path>` validates a file without rendering, `statusline --config <path>` renders with it.

```ron
(
    separator: " ",
    separator_color: Rgb(147, 153, 178),
    segments: [
        Model(color: Rgb(203, 166, 247), prefix: "", replacements: []),
        ContextUsage(
            color: Gradient,
            prefix: "", prefix_color: Rgb(203, 166, 247),
            suffix: "", suffix_color: Rgb(203, 166, 247),
        ),
        Spacer(shape: Gap),
        Cwd(color: Rgb(137, 180, 250)),
        GitBranch(
            color: Rgb(148, 226, 213), state_color: Rgb(250, 179, 135),
            show_worktree: true, show_ahead_behind: true, show_state: true,
        ),
    ],
)
```

Every segment is one struct in [`src/config_schema.rs`](src/config_schema.rs): the struct name is the RON tag, its fields are the options, and the `///` line above each one says what it does. That file is the catalogue; `statusline --schema` prints it as JSON and the builder is generated from it.

Inside a clone of this repo, Claude Code picks up the bundled [skill](.claude/skills/statusline-config/SKILL.md) and edits the config for you — "recolour the branch to lavender", "make 5h radial", "drop the 7d segment". It writes `config/local.ron` (gitignored) and runs `./install-local.sh`, which rebuilds, validates, and installs the binary together with that file.

## what it shows

The default line is the model, its effort level, context usage, the prompt-cache countdown, the 5-hour and 7-day rate limits, the working directory, the branch, and the diff counts:

- `cache 4m32s` counts down the Anthropic prompt cache. Once it reads `cache cold`, the colour follows context usage: cheap when the context is empty, expensive when full.
- `5h` / `7d` are the Claude.ai rolling usage limits, green under 50%, yellow to 80%, red above. Absent on API plans and before the first response. `window: Fable` adds the per-model weekly window, fetched in the background.
- `⑂feature` means you are inside a git worktree; `[REBASE 2/5]` and friends appear only during the operation; `~2 +1 -1` is modified, untracked, deleted.

The rest of the shelf is what a status line usually cannot do, each documented in [docs/segments.md](docs/segments.md):

- **LLM insight** — an external model reads the transcript every few turns and answers your prompt in one line: the current goal, what the last prompt was missing, whatever you ask. Add it twice for two independent lines.
- **My last prompt** — the last thing you typed, so several Claude Code windows are easy to tell apart.
- **Session notice** — `statusline --notice "standup at 15:00" --ttl 3600` pins a message to this session.
- **Reminders** — `statusline --remind "take the coffee" --in 30m` shows up in every session at its time.
- **Failed-command hook** — the binary as a `PostToolUse` hook puts `✗ cargo test (exit 101)` on the line until the next command succeeds.
- **Subagent stats** — subagents running and launched, how long the oldest has been at it, and the tokens they burned.
- **Command output** — the last line any command prints, re-run once its TTL runs out.
- **Weather** — a [wttr.in](https://wttr.in) one-liner for the city your timezone points at.
- **Claude resource usage** — CPU and memory of the Claude Code process tree, Linux only.

## tuning

A segment with `standalone: true` gets a line of its own where it sits in the config, wrapped by words to the terminal width; `MyLastPrompt` is cut with `…` instead. `Spacer(shape: LineBreak)` starts the next segments on a new line, `Spacer(shape: BlankLine)` leaves an empty one, `Spacer(shape: Gap)` is just a gap, and `Spacer(shape: Divider)` draws a `│` in `separator_color` between the segments around it, disappearing when there is nothing shown on one side. A main line wider than the terminal wraps at segment boundaries:

<p><img src="docs/screenshots/wrap.png" alt="multi-line wrap when statusline exceeds terminal width"/></p>

Colours are `Named(code)` for ANSI 30–37 and 90–97, `Rgb(r, g, b)` for truecolor, or `Gradient` where a segment supports it (`ContextUsage`, `PromptCacheTtl`, `RateLimit` with `color_mode: Gradient`).

Claude Code re-renders the line after every assistant message. Segments that change on their own — idle time, reminders, resource usage — need `statusLine.refreshInterval` in Claude Code settings to tick between messages.

## for hackers

<details>
<summary>adding a segment</summary>

Declare the segment's config fields in `src/config_schema.rs` and add a `SegmentSpec` variant for them, define the logic in `src/segments/*.rs` (re-export the schema struct and `impl Segment` for it), register the module in `src/segments.rs`, map the variant in `src/config.rs`, add the segment to `config/default.ron`, then build.

The config struct carries a single `///` line saying what the segment shows — that line is the segment's entry in the catalogue, so nothing has to be added to this page for a new segment to be documented. `statusline --schema` prints that catalogue as JSON — every segment's name, its pitch, and its fields with their kinds — and the builder page is built from it at deploy time, so a new segment shows up there without touching `site/`.

`UserIdleTime` is the concrete extension example, added in [`fac22e1`](https://github.com/Darkwing4/statusline-rs-cc/commit/fac22e1c1b04822b332c00268305bfc9224547b1). It reads `transcript_path`, ignores tool-result messages, finds the latest real user input timestamp, and renders values like `idle 42s`, `idle 3m12s`, or `idle 1h0m`.

To make `UserIdleTime` tick without new Claude events, opt in with `statusLine.refreshInterval` in Claude Code settings.

Register it with `pub mod user_idle_time;` in `src/segments.rs`, then add it to `config/default.ron`:

```ron
UserIdleTime(
    color: Rgb(147, 153, 178),
    prefix: "idle ",
    threshold_seconds: 0,
)
```

Segments receive the raw `serde_json::Value` so they own which input fields they read — only the config fields go through `config_schema.rs`, which `build.rs` uses to reject an invalid config at build time. For git-aware segments take `git: &mut GitCache` and call `git.dir()` / `git.status()` — `git status` is forked at most once per render, shared. For segments that render on a line of their own (multi-line debug output), override `fn standalone(&self) -> bool { true }`; such a line is wrapped by words unless the segment also overrides `fn overflow(&self) -> Overflow { Overflow::Truncate }`.

</details>

<details>
<summary>source layout</summary>

```
src/
├── main.rs                 entry: build Renderer, write to stdout
├── config.rs               loads the runtime config or the embedded one, maps SegmentSpec to segments
├── config_schema.rs        RON schema, shared with build.rs for build-time validation
├── segment_catalog.rs      reads the pitches and fields out of config_schema.rs for --schema
├── statusline_renderer.rs  owns segments, joins them, wraps to terminal width
├── statusline_renderer/
│   ├── segment_wrapping.rs wraps segments to lines by visible (ANSI-stripped) width
│   ├── terminal_width.rs   terminal columns via ioctl, COLUMNS, parent process tree
│   └── line_overflow.rs    folds a standalone line by words or cuts it at the width, reopening colour per line
├── statusline_input.rs     reads + parses stdin JSON from Claude Code
├── statusline_cache_dir.rs    XDG cache directory used by cross-render caches
├── private_file.rs         writes cache files readable by the owner only
├── statusline_cli.rs       argument parsing: render, --refresh, --notice, --remind, --hook
├── statusline_hook.rs      PostToolUse payload -> notice about a failed command
├── statusline_notice_store.rs per-session notice file, written by the CLI
├── statusline_reminder_store.rs machine-wide reminders with a show-at timestamp
├── transcript_tail_reader.rs  scans transcript JSONL backwards in 64 KB blocks
├── transcript_forward_reader.rs  scans transcript JSONL forward, record by record
├── types.rs / types/       shared types (Color, RESET)
└── segments/            one file per segment, each pitched in one line in config_schema.rs
    ├── background_command.rs   detached refresh worker + TTL cache shared by the command-driven ones
    ├── single_line_text.rs ANSI/control stripping + truncation for untrusted text
    ├── git/                GitCache shared by branch + diff (one git status fork)
    └── debug/              gated behind cfg(debug_assertions)
```

</details>

<details>
<summary>the builder page</summary>

The page lives in [`site/`](site/) and [`.github/workflows/pages.yml`](.github/workflows/pages.yml) deploys it after every release, or by hand from the Actions tab. Nothing runs server-side: the browser gzips the RON and encodes it as base64url into `STATUSLINE_INSTALL_CONFIG`, and `install.sh` decodes it, validates it with `--check-config` on the freshly downloaded binary, and writes it to `~/.claude/statusline/config.ron`.

The page is written in ClojureScript. The pure core (catalogue, segment model, layout, preview, RON) lives in `.cljc` files, so `bb test` and `bb ron` in `site/` run it in [babashka](https://babashka.org/) without a JVM; the browser bundle needs JDK 21+ for [shadow-cljs](https://shadow-cljs.github.io/docs/UsersGuide.html). To work on the page locally:

```sh
cargo run -- --schema > site/segment-catalog.json
cd site
npm ci && npm run assets
npx shadow-cljs watch app
```

`watch` serves `site/public` at http://localhost:8080 and recompiles on save. CI builds the release bundle, runs the tests in node, and validates the RON the page emits for every segment with `--check-config`.

</details>

<details>
<summary>under the hood</summary>

Claude Code invokes the statusline after each assistant message (and a few other events), feeds JSON on stdin, renders whatever lands on stdout. Execution is async — a slow statusline never blocks input, in-flight runs are cancelled on update. Contract:

```json
{ "cwd": "...", "context_window": { "used_percentage": 42.5 }, "model": { "display_name": "Opus" }, "effort": { "level": "high" }, "workspace": {...} }
```

Hot path on Linux x86_64 (i7-12700H, median of 60): **~4.9 ms** inside a git repo, **~2.1 ms** outside, **~424 KB** stripped release binary. `git status --branch --porcelain=v2` forks once; everything else (HOME shortening, `.git` ancestor walk, state detection, terminal width via `ioctl`) runs in-process.

</details>

## license

MIT
