# statusline

The fastest Claude Code statusline: a single Rust binary, ~5 ms per render, zero runtime deps. Edit the config yourself, or ask Claude Code to do it — a bundled skill rewrites the RON, rebuilds, and reinstalls in one step.

<table>
  <tr>
    <td><img src="docs/screenshots/hero.png" alt="default look"/></td>
    <td><img src="docs/screenshots/states.png" alt="worktree + rebase"/></td>
  </tr>
  <tr>
    <td><img src="docs/screenshots/nogit.png" alt="outside git repo"/></td>
    <td><img src="docs/screenshots/debug.png" alt="debug segment below statusline"/></td>
  </tr>
</table>

When the line is wider than the terminal, the renderer wraps it across multiple lines instead of truncating:

<p><img src="docs/screenshots/wrap.png" alt="multi-line wrap when statusline exceeds terminal width"/></p>

Segments on their own line (`standalone: true`, `LlmMessage`) are folded by words to the same width, with the colour reopened on every wrapped line, so an answer wider than a split-screen terminal is wrapped instead of cut off.

Every segment is tweakable from the RON config, and some ship with multiple styles. For example, `RateLimit` has radial dial, bar, and plain percent (plus `BarPercent` / `RadialPercent` which combine a graphic with the number):

<table>
  <tr>
    <td><img src="docs/screenshots/ratelimit-radial.png" alt="ratelimit radial style"/></td>
    <td><img src="docs/screenshots/ratelimit-bar.png" alt="ratelimit bar+percent style"/></td>
    <td><img src="docs/screenshots/ratelimit-percent.png" alt="ratelimit plain percent style"/></td>
  </tr>
</table>

<details>
<summary>cheat sheet — non-obvious bits</summary>

- `cache 4m32s` is the Anthropic prompt-cache TTL countdown. Once `cache cold`, the colour ramps by `context_window` % — cheap when context is empty, expensive when full.
- `5h` / `7d` are Claude.ai rolling usage limits: green <50%, yellow 50–80%, red >80%. Absent on API plans and before the first response.
- `⑂feature` means you're inside a git worktree (resolved by reading `.git`, no `fork()`).
- git state like `[REBASE 2/5]` only shows during the op (`MERGE`, `CHERRY-PICK`, `REVERT`, `BISECT`, `AM n/m`).
- diff `~2 +1 -1` = modified tracked / untracked / deleted.

</details>

## install

Downloads the right prebuilt binary, drops it in `~/.claude/bin/statusline` (or `%USERPROFILE%\.claude\bin\statusline.exe` on Windows), and patches `settings.json` so Claude Code picks it up.

**Linux / macOS:**

```sh
curl -fsSL https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.sh | sh
```

**Windows (PowerShell):**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.ps1 | iex"
```

Supported targets: Linux x86_64 / aarch64, macOS x86_64 / aarch64, Windows x86_64 / aarch64.

<details>
<summary>env vars, settings patch, build from source</summary>

| var | default |
|---|---|
| `STATUSLINE_TAG` | `latest` |
| `STATUSLINE_INSTALL_DIR` | `$HOME/.claude/bin` |
| `STATUSLINE_SETTINGS` | `$HOME/.claude/settings.json` |
| `STATUSLINE_SKIP_SETTINGS` | unset — set to `1` to skip the JSON patch |
| `STATUSLINE_REPO` | `Darkwing4/statusline-rs-cc` |

The settings patch is non-destructive: preserves every other key in `settings.json`, writes a `.bak` next to the original, no-ops if already pointed at the binary. If `python3` is missing it skips the patch and prints the snippet to paste manually.

Build from source:

```sh
cargo build --release
cp target/release/statusline ~/.claude/bin/statusline
```

</details>

## claude code skill

Ships with a project-local skill at [`.claude/skills/statusline-config/`](.claude/skills/statusline-config/SKILL.md). Open Claude Code in the cloned repo and it auto-discovers it — then ask in plain language and Claude edits the RON, rebuilds, and copies the binary into place:

> recolour the branch to lavender
> make 5h radial
> drop the 7d segment

The skill defaults to editing `config/local.ron` (gitignored personal override) and runs `./install-local.sh` to reinstall. Say "for the repo" to edit `config/default.ron` instead.

## configuration

The whole config is an external [RON](https://github.com/ron-rs/ron) file at [`config/default.ron`](config/default.ron). `build.rs` embeds it into the binary at compile time; [`src/config.rs`](src/config.rs) parses it into segments at startup. Point `STATUSLINE_CONFIG` at a different file to swap the embedded config without touching the source — `install-local.sh` auto-picks `config/local.ron` if it exists (gitignored personal override).

```ron
(
    separator: " ",
    separator_color: Named(90),
    segments: [
        Model(
            color: Rgb(180, 142, 173),
            prefix: "",
        ),
        Effort(
            color: Named(90),
            prefix: "",
        ),
        Context(
            color: Gradient,
            prefix: "", prefix_color: Rgb(180, 142, 173),
            suffix: "", suffix_color: Rgb(180, 142, 173),
        ),
        CacheTtl(color: Gradient, prefix: "cache "),
        RateLimit(
            window: FiveHour, style: Bar, fill: Remaining, color_mode: Gradient,
            gradient_midpoint_percentage: 50.0,
            prefix: "5h ",
            low_color:  Rgb(103, 175, 103),
            mid_color:  Rgb(195, 179, 100),
            high_color: Rgb(220,  60,  60),
        ),
        RateLimit(
            window: SevenDay, style: Bar, fill: Remaining, color_mode: Gradient,
            gradient_midpoint_percentage: 50.0,
            prefix: "7d ",
            low_color:  Rgb(103, 175, 103),
            mid_color:  Rgb(195, 179, 100),
            high_color: Rgb(220,  60,  60),
        ),
        SubagentStats(
            color: Named(90),
            active_color: Rgb(150, 200, 100),
            stall_color: Rgb(220, 60, 60),
            prefix: "agents ",
            stall_marker: "!",
            stall_seconds: 120,
            show_tokens: true,
        ),
        Reminder(
            color: Rgb(230, 180, 80),
            prefix: "\u{23F0} ",
            separator: " \u{B7} ",
            max_chars: 80,
            standalone: false,
        ),
        Notice(
            color: Named(93),
            prefix: "\u{1F4CC} ",
            max_chars: 120,
            show_remaining: true,
            standalone: true,
        ),
        Cwd(color: Rgb(95, 175, 175)),
        GitBranch(
            color: Named(32), state_color: Named(91),
            show_worktree: true, show_ahead_behind: true, show_state: true,
        ),
        GitDiff(
            modified_color:  Named(33),
            untracked_color: Named(32),
            deleted_color:   Named(31),
        ),
        GitError(color: Named(91), text: "no git"),
        SessionTask(
            color: Named(90),
            prefix: "\u{BB} ",
            max_chars: 48,
            standalone: false,
        ),
    ],
)
```

Reorder, drop, or re-colour by editing the list, then rebuild. `Color` variants: `Named(code)` for ANSI 30–37 / 90–97, `Rgb(r, g, b)` for truecolor, `Gradient` (meaningful on `Context`, `CacheTtl`, and `RateLimit` when `color_mode: Gradient`).
`RateLimit.gradient_midpoint_percentage` places `mid_color` within the gradient and must be greater than `0` and less than `100`; existing configs without the field use `50.0`.

### Subagent stats

`SubagentStats` counts the `Agent` tool calls in the session and renders `agents 2/7 4m12s 1.2M`: two subagents still running out of seven launched, the oldest running one started 4m12s ago, and 1.2M tokens burned by subagents in total. Once every agent has finished the active counter and the age drop off, leaving `agents 7 1.2M`.

A launch is a `tool_use` block named `Agent` in the main thread; it finishes on its `tool_result` — or, for async agents whose first result is only `async_launched`, on the `<task-notification>` that reports the matching `<tool-use-id>`. Token totals sum `input`, `output`, `cache_creation`, and `cache_read` across `<session>/subagents/**/*.jsonl`, so agents started by `Workflow` are counted in the total and in the tokens even though they never appear as an `Agent` tool call.

`stall_seconds` guards against agents that never report back: when subagents are active but nothing has been written to any of their transcripts for that long, the segment appends `stall_marker` and switches to `stall_color`. Set `show_tokens: false` to skip the token pass entirely.

Both scans are incremental — a cache under `$XDG_CACHE_HOME/statusline` (or `~/.cache/statusline`) keeps the byte offset reached in every transcript, so each render only parses what was appended since the previous one. A truncated or rewritten transcript resets its offset. On a 4 MB transcript with 4 MB of subagent transcripts the first render costs ~31 ms and later ones ~11 ms.

### LLM message

`LlmMessage` runs any command that prints text and renders its last non-empty output line on its own line below the main one:

```ron
LlmMessage(
    color: Rgb(150, 140, 120),
    prefix: "» ",
    command: "codex",
    args: ["exec", "--skip-git-repo-check", "-s", "read-only", "-c", "approval_policy=never", "-c", "model_reasoning_effort=low"],
    prompt: "One short motivational line. Text only, no quotes, no explanation.",
    ttl_seconds: 900,
    max_chars: 90,
)
```

The worker runs the command in an empty `workdir` under the cache directory, never in the project, and its stdout is the only thing taken from it. An agent CLI still has to be told to stay a text model — for `codex` that is `-s read-only -c approval_policy=never` — because whatever it reads on stdin is a prompt-injection path into everything it is allowed to touch. Files the worker writes are created readable by the owner only.

`prompt` is written to the command's stdin — put it in `args` instead if the tool expects it as an argument. The command does not have to be an LLM: `command: "curl"` with `args: ["-s", "https://example.com/tip"]` renders whatever the server answers.

The render never waits for the command: it prints the cached text and, when that text is older than `ttl_seconds`, re-executes the binary as `statusline --refresh <fingerprint>` detached in the background. The worker looks up the segment whose command, args, and stdin hash to that fingerprint, runs it, and swaps the result in via a rename, so a render never sees a half-written line. A failed or hanging command leaves the previous text in place and is retried after another `ttl_seconds` — a hanging one is not killed, so pick a command that terminates on its own.

Output is treated as untrusted: ANSI escapes and control characters are stripped, whitespace is collapsed, and the text is cut to `max_chars` with an ellipsis. This segment is opt-in — it is not in `config/default.ron`.

### LLM insight

`LlmInsight` is the open slot: you write the prompt, you decide how often it runs. Every `every_turns` prompts in the session it hands the model its own previous answer plus the conversation since then, and renders the one line that comes back.

```ron
LlmInsight(
    color: Rgb(150, 190, 150),
    prefix: "\u{1F3AF} ",
    command: "codex",
    args: ["exec", "--skip-git-repo-check", "-s", "read-only", "-c", "approval_policy=never", "-m", "gpt-5.6-sol", "-c", "model_reasoning_effort=low"],
    prompt: "One short sentence: what the user is after and what is being done for it.",
    every_turns: 2,
    scan_whole_session: true,
    initial_scan_bytes: 262144,
    context_chars: 12000,
    max_chars: 128,
    standalone: true,
)
```

The stdin the command receives has five parts: your `prompt`, a `[hard limit]` line, `[your previous answer]`, `[conversation so far, oldest first]`, and `[new since your previous answer]`. The hard limit repeats `max_chars` back to the model and tells it to fit the whole thought inside it, contracting a word or two (`сокр-я`) when that is all it takes, so it packs the answer instead of getting cut off — keep the character count out of your own `prompt` and let this line carry it. The context window is the newest `context_chars` characters of the session's user and assistant text and is never cleared, so the model can see what has already been done and does not suggest it again; the fresh part holds only what arrived since its last answer and is cleared after each run. Tool results, subagent traffic, and records Claude Code marks as `isMeta` — slash-command wrappers and the skill documents they pull in — are left out of both, and a single message is cut at 800 characters so one pasted wall of text cannot push the real conversation out of the window.

How much history the window starts from is separate from how often the command runs. `scan_whole_session: true` reads the transcript from its first byte on the first render of a session; with `false` it starts `initial_scan_bytes` before the end. Either way the scan is one-off — every later render resumes at the byte offset it stopped at. Reading a 3.4 MB transcript whole cost 69 ms once and 9 ms per render afterwards.

Add the block twice with different prompts and you get two independent lines — a goal tracker and, say, a critic that suggests what the last prompt was missing. Each instance keys its cache off `command`, `args`, and `prompt`, so they never overwrite each other.

A run is skipped while a previous worker is still starting (30 s guard) and when nothing new has arrived. `every_turns: 0` freezes the segment on its last answer without ever launching the command again.

Every answer is appended to one machine-wide log, `insight-history.jsonl` in the cache directory, as `{at, session, cwd, prompt, input, answer}` — the segment only ever shows its latest line, the log is what lets you look back at what was shown, in which session, and on what input. `input` is the new conversation the answer was based on; the context window is left out of the log because it is recoverable from the transcript. Entries older than three months are dropped, checked whenever the file passes 256 KB.

### Weather

`Weather` renders a [wttr.in](https://wttr.in) one-liner such as `🌦️ +27°C` through the same background refresh as `LlmMessage`:

```ron
Weather(
    color: Rgb(120, 170, 200),
    prefix: "",
    location: "",
    format: "%c+%t",
    ttl_seconds: 1800,
    max_chars: 24,
)
```

`location` is a fallback: the city is taken from the system timezone first — `TZ`, then `/etc/timezone`, then the `/etc/localtime` symlink — so `Asia/Bangkok` becomes `Bangkok`. Timezones that name no city (`UTC`) and systems without either file fall back to the configured `location`; leave both empty and wttr.in resolves the location by IP. `format` is passed to wttr.in as-is (`%c` condition, `%t` temperature, `%l` location, `%w` wind).

The request is `curl -s --max-time 10`, so no HTTP client is linked into the binary. Location and format are filtered before they reach the URL — path characters outside letters, digits, spaces, `-_,.` are dropped and `&#?` in the format are percent-encoded. Also opt-in.

### Notice

`Notice` renders a message written from outside the render — a reminder, a hand-off note, whatever Claude Code (or a hook, or a cron job) put there:

```ron
Notice(
    color: Named(93),
    prefix: "\u{1F4CC} ",
    max_chars: 120,
    show_remaining: true,
    standalone: true,
)
```

The binary itself is the write side:

```sh
statusline --notice "созвон 15:00, проверить логи" --ttl 3600
statusline --notice "без срока живёт до --notice-clear"
statusline --notice-clear
```

Notices are per session. The session id comes from `--session <id>` or, when it is omitted, from the `CLAUDE_CODE_SESSION_ID` environment variable that Claude Code exports into every command it runs — so a plain `statusline --notice "..."` from a Claude Code shell lands on that session's status line and nowhere else. Without either, the command fails instead of guessing.

`--ttl <seconds>` sets a deadline; `show_remaining: true` appends the time left as `(9m55s)`. A notice past its deadline is not rendered and its file is deleted on the next render. Without `--ttl` the notice stays until it is replaced or cleared. One notice per session — a second `--notice` overwrites the first.

The text is stored as JSON in the cache directory (`notice-<session>.json`), written through a temp file and a rename so a render never sees half a notice. It is treated as untrusted on the way out: ANSI escapes and control characters are stripped, whitespace is collapsed, and it is cut to `max_chars`. Set `standalone: false` to render it inline among the other segments instead of on its own line.

### Session task

`SessionTask` shows what this window was last asked to do — the latest prompt the user actually typed:

```ron
SessionTask(
    color: Named(90),
    prefix: "\u{BB} ",
    max_chars: 48,
    standalone: false,
)
```

It scans the transcript backwards and stops at the first `user` record that is a real prompt: tool results, subagent (`isSidechain`) messages, `isMeta` records (slash commands and the skill text they inject), hook output, and `Caveat:` notes are skipped, so `/compact` in the middle of a session does not replace the task with the word `compact`. Nothing is written anywhere — the transcript is the only source, so the segment costs one backward scan of its tail.

With several Claude Code windows open this is the fastest way to tell them apart. Claude Code has no on-disk todo list to read, so this is the prompt, not a checklist step.

### Reminders

`Reminder` holds messages that are written now and shown later, across every session on the machine:

```ron
Reminder(
    color: Rgb(230, 180, 80),
    prefix: "\u{23F0} ",
    separator: " \u{B7} ",
    max_chars: 80,
    standalone: false,
)
```

```sh
statusline --remind "созвон" --in 30m           # shows up in 30 minutes, stays an hour
statusline --remind "снять кофе" --in 90s --for 10m
statusline --remind-clear                       # drop the ones already on screen
statusline --remind-clear --all                 # drop the pending ones too
```

`--in` and `--for` take `45s`, `30m`, `2h`, `1d`, or a bare number of seconds; `--for` defaults to an hour and counts from the moment the reminder fires. Reminders live in one machine-wide `reminders.json` — unlike `Notice` they are not tied to a session, so a reminder written in one Claude Code window appears in all of them. Everything due at once is joined with `separator` behind a single `prefix`; expired entries are dropped on the next render.

There is no timer and no daemon: a reminder is a timestamp on disk, and every render compares it to the clock. It therefore appears on the first render after its time — set `statusLine.refreshInterval` in Claude Code settings if you want that to happen without touching the keyboard.

### Failed-command hook

The binary can also be a hook. Point Claude Code's `PostToolUse` at it and a failed shell command lands in the status line as a `Notice`:

```json
"PostToolUse": [
  {
    "matcher": "Bash",
    "hooks": [
      { "type": "command", "command": "~/.claude/bin/statusline --hook" }
    ]
  }
]
```

The hook reads the event JSON on stdin, and for `Bash` calls only: a non-zero `exit_code` or an interrupted command writes `\u{2717} cargo test (exit 101)` for that session, and the next command that succeeds removes it again. `--ttl <secs>` sets how long the message survives (15 minutes by default).

It only ever removes a notice it wrote itself — notices written by `--notice` carry no `source` and are left alone, so a hand-written reminder is not wiped by the next green test run. No jq, no shell wrapper, no extra process: the same binary that renders the line handles the event.

### Linux resource usage

`ClaudeResourceUsage` is an opt-in Linux-only segment:

```ron
ClaudeResourceUsage(
    color: Named(90),
    cpu_prefix: "CPU ",
    memory_prefix: "RSS ",
)
```

It validates `session_id` against Claude Code's local session registry instead of guessing by working directory. If no matching live process exists, or on macOS and Windows, it emits nothing. CPU is shown in logical-core equivalents, so `1.00c` means one fully used core. RSS is the summed resident set size of the Claude process tree and is displayed in MiB. The first CPU sample is shown as `—` because no previous sample exists.

Set `statusLine.refreshInterval` to `1` in Claude Code settings for periodic live updates.

## extending

Declare the segment's config fields in `src/config_schema.rs` and add a `SegmentSpec` variant for them, define the logic in `src/segments/*.rs` (re-export the schema struct and `impl Segment` for it), register the module in `src/segments.rs`, map the variant in `src/config.rs`, add the segment to `config/default.ron`, then build.

`IdleTime` is the concrete extension example, added in [`fac22e1`](https://github.com/Darkwing4/statusline-rs-cc/commit/fac22e1c1b04822b332c00268305bfc9224547b1). It reads `transcript_path`, ignores tool-result messages, finds the latest real user input timestamp, and renders values like `idle 42s`, `idle 3m12s`, or `idle 1h0m`.

To make `IdleTime` tick without new Claude events, opt in with `statusLine.refreshInterval` in Claude Code settings.

Register it with `pub mod idle_time;` in `src/segments.rs`, then add it to `config/default.ron`:

```ron
IdleTime(
    color: Named(90),
    prefix: "idle ",
    threshold_seconds: 0,
)
```

Segments receive the raw `serde_json::Value` so they own which input fields they read — only the config fields go through `config_schema.rs`, which `build.rs` uses to reject an invalid config at build time. For git-aware segments take `git: &mut GitCache` and call `git.dir()` / `git.status()` — `git status` is forked at most once per render, shared. For segments that render on their own line below the main one (multi-line debug output), override `fn standalone(&self) -> bool { true }`.

<details>
<summary>source layout</summary>

```
src/
├── main.rs                 entry: build Renderer, write to stdout
├── config.rs               loads the embedded config, maps SegmentSpec to segments
├── config_schema.rs        RON schema, shared with build.rs for build-time validation
├── statusline_renderer.rs  owns segments, joins them, wraps to terminal width
├── statusline_renderer/
│   ├── segment_wrapping.rs wraps segments to lines by visible (ANSI-stripped) width
│   ├── terminal_width.rs   terminal columns via ioctl, COLUMNS, parent process tree
│   └── word_wrapping.rs    folds a standalone line by words, reopening colour per line
├── statusline_input.rs     reads + parses stdin JSON from Claude Code
├── statusline_cache_dir.rs    XDG cache directory used by cross-render caches
├── private_file.rs         writes cache files readable by the owner only
├── statusline_cli.rs       argument parsing: render, --refresh, --notice, --remind, --hook
├── statusline_hook.rs      PostToolUse payload -> notice about a failed command
├── statusline_notice_store.rs per-session notice file, written by the CLI
├── statusline_reminder_store.rs machine-wide reminders with a show-at timestamp
├── statusline_insight_history.rs append-only log of insight answers, kept 3 months
├── transcript_tail_reader.rs  scans transcript JSONL backwards in 64 KB blocks
├── transcript_forward_reader.rs  scans transcript JSONL forward, record by record
├── types.rs / types/       shared types (Color, RESET)
└── segments/
    ├── model.rs            current model name
    ├── effort.rs           current reasoning effort level
    ├── context.rs          context window % with gradient
    ├── cwd.rs              shortened cwd
    ├── idle_time.rs        time since last real user input
    ├── claude_resource_usage.rs  opt-in Linux process-tree CPU/RSS
    ├── background_command.rs   detached refresh worker + TTL cache shared by the two below
    ├── llm_message.rs      opt-in background command output, cached by TTL
    ├── llm_insight.rs      opt-in per-session prompt run every N turns on the chat delta
    ├── llm_insight/
    │   ├── turn_delta.rs      counts real prompts and collects the text added since last run
    │   └── insight_cache.rs   scan offset, pending delta, and the paths a worker writes to
    ├── weather.rs          opt-in wttr.in line, city from the system timezone
    ├── notice.rs           message written by `statusline --notice`, expires by TTL
    ├── reminder.rs         reminders that are due, written by `statusline --remind`
    ├── session_task.rs     latest real user prompt, scanned from the transcript tail
    ├── single_line_text.rs ANSI/control stripping + truncation for untrusted text
    ├── duration_format.rs  1h02m / 4m12s / 5s durations
    ├── weather/
    │   └── system_location.rs  city name out of TZ / /etc/timezone / /etc/localtime
    ├── subagent_stats.rs   active/launched subagents, age, token totals
    ├── subagent_stats/
    │   ├── agent_lifecycle.rs     Agent launches and completions in the main transcript
    │   ├── agent_token_totals.rs  token sums over the subagent transcripts
    │   └── session_cache.rs       scan offsets carried between renders
    ├── git/
    │   ├── tools.rs        GitCache shared by branch + diff (one git status fork)
    │   ├── branch.rs       branch name, worktree marker, state, ahead/behind
    │   └── diff.rs         ~N +N -N counts
    └── debug/              gated behind cfg(debug_assertions), see Debug below
```

</details>

## under the hood

Claude Code invokes the statusline after each assistant message (and a few other events), feeds JSON on stdin, renders whatever lands on stdout. Execution is async — a slow statusline never blocks input, in-flight runs are cancelled on update. Contract:

```json
{ "cwd": "...", "context_window": { "used_percentage": 42.5 }, "model": { "display_name": "Opus" }, "effort": { "level": "high" }, "workspace": {...} }
```

Hot path on Linux x86_64 (i7-12700H, median of 60): **~4.9 ms** inside a git repo, **~2.1 ms** outside, **~424 KB** stripped release binary. `git status --branch --porcelain=v2` forks once; everything else (HOME shortening, `.git` ancestor walk, state detection, terminal width via `ioctl`) runs in-process.

## license

MIT
