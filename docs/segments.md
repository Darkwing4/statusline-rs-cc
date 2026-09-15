# Segments in depth

What each of the less obvious segments does, where it reads from, and what its fields mean. The one-line pitch of every segment and the hint on every field live in [`src/config_schema.rs`](../src/config_schema.rs); `statusline --schema` prints them as JSON, and the [builder](https://darkwing4.github.io/statusline-rs-cc/) shows them next to every control.

## Rate limit styles

`RateLimit` renders the 5-hour and 7-day Claude.ai windows as a plain percentage, a bar, or a radial dial, and `BarPercent` / `RadialPercent` put the number next to the graphic:

<table>
  <tr>
    <td><img src="screenshots/ratelimit-radial.png" alt="ratelimit radial style"/></td>
    <td><img src="screenshots/ratelimit-bar.png" alt="ratelimit bar+percent style"/></td>
    <td><img src="screenshots/ratelimit-percent.png" alt="ratelimit plain percent style"/></td>
  </tr>
</table>

`color_mode: Steps` switches between `low_color`, `mid_color`, and `high_color` at fixed thresholds; `Gradient` blends them, and `gradient_midpoint_percentage` (greater than `0`, less than `100`, `50.0` when omitted) is where `mid_color` sits. `prefix` may contain `{t}`, which becomes the time left until the window resets: `"{t}h "` renders `3.1h`, `"{t}d "` renders `4.8d`.

## Fable rate limit

`window: FiveHour` and `window: SevenDay` read `rate_limits` straight from the status line
payload. Claude Code does not put the per-model Fable window there, so `window: Fable` gets it
itself: it reads the OAuth token from `.credentials.json` in the Claude config directory
(`CLAUDE_CONFIG_DIR`, or `~/.claude`), fetches `https://api.anthropic.com/api/oauth/usage` with
a detached `curl`, and caches the response under the statusline cache directory with owner-only
permissions, one file per config directory so parallel accounts never read each other's
numbers. The Fable entry is the one in
`limits[]` with `kind: "weekly_scoped"` and `scope.model.display_name: "Fable"`.

The segment is skipped entirely unless `model.id` or `model.display_name` mentions Fable, so no
token is read and no request is made on other models. Missing credentials, an expired token, or
no network simply hide it. As with every background-refreshed segment, the first render after
the cache expires still shows the previous value.

Three fields exist for this window and default to being invisible, so `FiveHour` / `SevenDay`
blocks need no changes:

| field | default | meaning |
| --- | --- | --- |
| `usage_ttl_seconds` | `300` | how often `/api/oauth/usage` is refetched |
| `active_marker` | `""` | appended while the server reports `is_active` — the window currently doing the limiting |
| `severity_markers` | `[]` | `severity` → marker pairs, e.g. `[("warning", "!")]`; unmatched values render nothing |

```ron
        RateLimit(
            window: Fable, style: BarPercent, fill: Used, color_mode: Gradient,
            prefix: "{t}d ",
            usage_ttl_seconds: 300,
            active_marker: "*",
            severity_markers: [("warning", "!"), ("critical", "!!")],
            low_color:  Rgb(166, 227, 161),
            mid_color:  Rgb(249, 226, 175),
            high_color: Rgb(243, 139, 168),
        ),
```

## Model name replacements

`Model.replacements` is an optional list of literal `(search, replace)` pairs applied in order to the model name before it is coloured:

```ron
Model(
    color: Rgb(203, 166, 247),
    prefix: "",
    replacements: [
        ("Opus 5 (1M context)", "Opus"),
        (" (1M context)", ""),
    ],
)
```

Substring matches, no regex. The search string must not be empty. Existing configs without the field replace nothing, and the segment hides itself if the replacements leave an empty name.

## Subagent stats

`SubagentStats` counts the `Agent` tool calls in the session and renders `agents 2/7 4m12s 1.2M`: two subagents still running out of seven launched, the oldest running one started 4m12s ago, and 1.2M tokens burned by subagents in total. Once every agent has finished the active counter and the age drop off, leaving `agents 7 1.2M`.

A launch is a `tool_use` block named `Agent` in the main thread; it finishes on its `tool_result` — or, for async agents whose first result is only `async_launched`, on the `<task-notification>` that reports the matching `<tool-use-id>`. Token totals sum `input`, `output`, `cache_creation`, and `cache_read` across `<session>/subagents/**/*.jsonl`, counting each response once however many transcript rows repeat its usage, so agents started by `Workflow` are counted in the total and in the tokens even though they never appear as an `Agent` tool call.

`stall_seconds` guards against agents that never report back: when subagents are active but nothing has been written to any of their transcripts for that long, the segment appends `stall_marker` and switches to `stall_color`. Set `show_tokens: false` to skip the token pass entirely.

Both scans are incremental — a cache under `$XDG_CACHE_HOME/statusline` (or `~/.cache/statusline`) keeps the byte offset reached in every transcript, so each render only parses what was appended since the previous one. A truncated or rewritten transcript resets its offset. On a 4 MB transcript with 4 MB of subagent transcripts the first render costs ~31 ms and later ones ~11 ms.

## Tokens by model

`TokensByModel` renders `tokens opus-5 3.4M · haiku-4-5 45k`: every token the session has spent, main thread and subagents together, grouped by the model that spent it and listed from the biggest spender down. It is the same four-bucket sum `SubagentStats` shows, so on a long session cache reads make up most of the number.

Claude Code writes a transcript row per content block and repeats the response's `usage` on each of them, with `output_tokens` growing as the response streams, so a response is counted once, from the last row carrying its `message.id`. Model ids lose the `claude-` prefix and a trailing release date, so `claude-haiku-4-5-20251001` shows as `haiku-4-5`. The scan is incremental like the subagent one and keeps its offsets in `session-tokens-<session>.json` in the same cache directory, shared with `TokenSpend`.

## Token spend

`TokenSpend` renders `turn 58k: in 1.2k out 3.4k think 1.1k cache read 52k write 1.9k` — the total first, then what it is made of. `in` is fresh input, `cache read` is context replayed from the prompt cache, `cache write` is what was added to the cache, and `out` is everything the model produced. `think` is the reasoning share of `out`, taken from `usage.output_tokens_details.thinking_tokens`, so it is not added to the total a second time; transcripts written before Claude Code reported it show `think 0`.

`scope: LastTurn` counts from your last prompt, so the numbers grow while the answer runs and stay on screen once it is done. A prompt is a user row you typed: tool results, skill bodies marked `isMeta`, and injected wrappers such as `<task-notification>` or a slash command keep the current turn going. Subagent rows join the turn when they were written after that prompt. `scope: Session` counts everything since the session started — the total `TokensByModel` splits by model. Add the block twice to see both.

## Session cost

`SessionCost` renders `$4.20` from `cost.total_cost_usd`, the estimate Claude Code itself sends to the status line: list prices, or a `modelPricing` table when one is set, applied to every API call of the session. On a subscription it is what the same traffic would cost on the API, not a bill. It stays hidden until the first response is priced.

## Command output

`CommandOutput` runs any command that prints text and renders its last non-empty output line on a line of its own.

The worker runs the command in an empty `workdir` under the cache directory, never in the project, and its stdout is the only thing taken from it. An agent CLI still has to be told to stay a text model — for `codex` that is `-s read-only -c approval_policy=never` — because whatever it reads on stdin is a prompt-injection path into everything it is allowed to touch. Files the worker writes are created readable by the owner only.

`prompt` is written to the command's stdin — put it in `args` instead if the tool expects it as an argument. The command does not have to be an LLM: `command: "curl"` with `args: ["-s", "https://example.com/tip"]` renders whatever the server answers.

The render never waits for the command: it prints the cached text and, when that text is older than `ttl_seconds`, re-executes the binary as `statusline --refresh <fingerprint>` detached in the background. The worker looks up the segment whose command, args, and stdin hash to that fingerprint, runs it, and swaps the result in via a rename, so a render never sees a half-written line. A failed or hanging command leaves the previous text in place and is retried after another `ttl_seconds` — a hanging one is not killed, so pick a command that terminates on its own.

Output is treated as untrusted: ANSI escapes and control characters are stripped, whitespace is collapsed, and the text is cut to `max_chars` with an ellipsis. This segment is opt-in — it is not in `config/default.ron`.

## LLM insight

`LlmInsight` is the open slot: you write the prompt, you decide how often it runs. Every `every_turns` prompts in the session it hands the model its own previous answer plus the conversation since then, and renders the one line that comes back.

The stdin the command receives has five parts: your `prompt`, a `[hard limit]` line, `[your previous answer]`, `[conversation so far, oldest first]`, and `[new since your previous answer]`. The hard limit repeats `max_chars` back to the model and tells it to fit the whole thought inside it, contracting a word or two when that is all it takes, so it packs the answer instead of getting cut off — keep the character count out of your own `prompt` and let this line carry it. The context window is the newest `context_chars` characters of the session's user and assistant text and is never cleared, so the model can see what has already been done and does not suggest it again; the fresh part holds only what arrived since its last answer and is cleared after each run. Tool results, subagent traffic, and records Claude Code marks as `isMeta` — slash-command wrappers and the skill documents they pull in — are left out of both, and a single message is cut at 800 characters so one pasted wall of text cannot push the real conversation out of the window.

How much history the window starts from is separate from how often the command runs. `scan_whole_session: true` reads the transcript from its first byte on the first render of a session; with `false` it starts `initial_scan_kib` KiB before the end. Either way the scan is one-off — every later render resumes at the byte offset it stopped at. Reading a 3.4 MB transcript whole cost 69 ms once and 9 ms per render afterwards.

Add the block twice with different prompts and you get two independent lines — a goal tracker and, say, a critic that suggests what the last prompt was missing. Each instance keys its cache off `command`, `args`, and `prompt`, so they never overwrite each other.

A run is skipped while a previous worker is still starting (30 s guard) and when nothing new has arrived. `every_turns: 0` freezes the segment on its last answer without ever launching the command again.

## Weather

`Weather` renders a [wttr.in](https://wttr.in) one-liner such as `🌦️ +27°C` through the same background refresh as `CommandOutput`.

`location` is a fallback: the city is taken from the system timezone first — `TZ`, then `/etc/timezone`, then the `/etc/localtime` symlink — so `Asia/Bangkok` becomes `Bangkok`. Timezones that name no city (`UTC`) and systems without either file fall back to the configured `location`; leave both empty and wttr.in resolves the location by IP. `format` is passed to wttr.in as-is (`%c` condition, `%t` temperature, `%l` location, `%w` wind).

The request is `curl -s --max-time 10`, so no HTTP client is linked into the binary. Location and format are filtered before they reach the URL — path characters outside letters, digits, spaces, `-_,.` are dropped and `&#?` in the format are percent-encoded. Also opt-in.

## Session notice

`SessionNotice` renders a message written from outside the render — a reminder, a hand-off note, whatever Claude Code (or a hook, or a cron job) put there.

The binary itself is the write side:

```sh
statusline --notice "standup at 15:00, check the logs" --ttl 3600
statusline --notice "no deadline, stays until --notice-clear"
statusline --notice-clear
```

Notices are per session. The session id comes from `--session <id>` or, when it is omitted, from the `CLAUDE_CODE_SESSION_ID` environment variable that Claude Code exports into every command it runs — so a plain `statusline --notice "..."` from a Claude Code shell lands on that session's status line and nowhere else. Without either, the command fails instead of guessing.

`--ttl <seconds>` sets a deadline; `show_remaining: true` appends the time left as `(9m55s)`. A notice past its deadline is not rendered and its file is deleted on the next render. Without `--ttl` the notice stays until it is replaced or cleared. One notice per session — a second `--notice` overwrites the first.

The text is stored as JSON in the cache directory (`notice-<session>.json`), written through a temp file and a rename so a render never sees half a notice. It is treated as untrusted on the way out: ANSI escapes and control characters are stripped, whitespace is collapsed, and it is cut to `max_chars`. Set `standalone: false` to render it inline among the other segments instead of on its own line.

## My last prompt

`MyLastPrompt` shows what this window was last asked to do — the latest prompt the user actually typed.

Inline, `max_chars` keeps it from crowding the main line. On its own line (`standalone: true`) set `max_chars: 0` and the prompt is cut only where the terminal ends.

It scans the transcript backwards and stops at the first `user` record that is a real prompt: tool results, subagent (`isSidechain`) messages, `isMeta` records (slash commands and the skill text they inject), hook output, and `Caveat:` notes are skipped, so `/compact` in the middle of a session does not replace the task with the word `compact`. Nothing is written anywhere — the transcript is the only source, so the segment costs one backward scan of its tail.

With several Claude Code windows open this is the fastest way to tell them apart. Claude Code has no on-disk todo list to read, so this is the prompt, not a checklist step.

## Reminders

`Reminder` holds messages that are written now and shown later, across every session on the machine.

```sh
statusline --remind "standup" --in 30m          # shows up in 30 minutes, stays an hour
statusline --remind "take the coffee" --in 90s --for 10m
statusline --remind-clear                       # drop the ones already on screen
statusline --remind-clear --all                 # drop the pending ones too
```

`--in` and `--for` take `45s`, `30m`, `2h`, `1d`, or a bare number of seconds; `--for` defaults to an hour and counts from the moment the reminder fires. Reminders live in one machine-wide `reminders.json` — unlike `SessionNotice` they are not tied to a session, so a reminder written in one Claude Code window appears in all of them. Everything due at once is joined with `separator` behind a single `prefix`; expired entries are dropped on the next render.

There is no timer and no daemon: a reminder is a timestamp on disk, and every render compares it to the clock. It therefore appears on the first render after its time — set `statusLine.refreshInterval` in Claude Code settings if you want that to happen without touching the keyboard.

## Failed-command hook

The binary can also be a hook. Point Claude Code's `PostToolUse` at it and a failed shell command lands in the status line as a `SessionNotice`:

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

## Linux resource usage

`ClaudeResourceUsage` is an opt-in Linux-only segment.

It validates `session_id` against Claude Code's local session registry instead of guessing by working directory. If no matching live process exists, or on macOS and Windows, it emits nothing. CPU is shown in logical-core equivalents, so `1.00c` means one fully used core. RSS is the summed resident set size of the Claude process tree and is displayed in MiB. The first CPU sample is shown as `—` because no previous sample exists.

Set `statusLine.refreshInterval` to `1` in Claude Code settings for periodic live updates.
