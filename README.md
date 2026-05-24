# statusline

Minimal, fast Claude Code statusline. Single Rust binary, ~5 ms per invocation, no runtime deps.

![statusline in Claude Code](hero.png)

## cheat sheet

![statusline preview](preview.png)

Default look is codex-inspired: dim middot separator, soft teal cwd, magenta-lavender context prefix, gradient percentage, green branch.

| segment | colour | example |
|---|---|---|
| context | grey-yellow → red gradient by usage | `42%` |
| cwd | soft teal `#5fafaf` | `~/proj` |
| git branch | green, `⑂` prefix inside worktree | `main`, `⑂feature` |
| git state | bright red, only during merge/rebase/etc. | `[REBASE 2/5]` |
| ahead/behind | branch colour | `(↑2 ↓1)` |
| diff | yellow / green / red | `~2 +1 -1` |
| separator | bright black middot | ` · ` |

Element-by-element rules:

- `~N` — modified tracked files (yellow)
- `+N` — untracked files (green)
- `-N` — deleted files (red)
- `[MERGE]`, `[REBASE n/m]`, `[CHERRY-PICK]`, `[REVERT]`, `[BISECT]`, `[AM n/m]` — bright red, only during the operation; rebase/am include `done/total`
- `~/proj` — `$HOME` shortened to `~`
- `⑂` — appears only inside a git worktree (resolved by reading `.git` gitfile without forking git)

Outside a git repo only context and cwd render. Worktree gitfiles (`/.git/worktrees/<name>`) are resolved in-process.

## install

One command. Downloads the right prebuilt binary for your platform, drops it in `~/.claude/bin/statusline` (or `%USERPROFILE%\.claude\bin\statusline.exe` on Windows), and patches `settings.json` so Claude Code picks it up on the next event.

**Linux / macOS:**

```sh
curl -fsSL https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.sh | sh
```

**Windows (PowerShell):**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.ps1 | iex"
```

Supported targets: Linux x86_64 / aarch64, macOS x86_64 / aarch64, Windows x86_64 / aarch64.

The settings patch is non-destructive: it preserves every other key in `settings.json`, writes a `.bak` next to the original before changing anything, and is a no-op if the file already points at the binary. If `python3` is missing it skips the patch and prints the snippet to paste manually.

Env vars the installer respects:

| var | default |
|---|---|
| `STATUSLINE_TAG` | `latest` |
| `STATUSLINE_INSTALL_DIR` | `$HOME/.claude/bin` |
| `STATUSLINE_SETTINGS` | `$HOME/.claude/settings.json` |
| `STATUSLINE_SKIP_SETTINGS` | unset — set to `1` to skip the JSON patch |
| `STATUSLINE_REPO` | `Darkwing4/statusline-rs-cc` |

## build from source

```sh
cargo build --release
cp target/release/statusline ~/.claude/bin/statusline
```

The binary is self-contained. To tweak the look — edit the `vec![...]` block in `src/main.rs` and rebuild.

## configuration

The set of segments, their order, colours, and the separator are declared as a literal `Renderer { ... }` value in [`src/main.rs`](src/main.rs):

```rust
let renderer = Renderer {
    separator: " · ",
    separator_color: Color::Named(90),
    items: vec![
        Box::new(Context {
            color: Color::Gradient,
            prefix: "",
            prefix_color: Color::Rgb(180, 142, 173),
            suffix: "",
            suffix_color: Color::Rgb(180, 142, 173),
        }),
        Box::new(Cwd { color: Color::Rgb(95, 175, 175) }),
        Box::new(GitBranch {
            color: Color::Named(32),
            state_color: Color::Named(91),
            show_worktree: true,
            show_ahead_behind: true,
            show_state: true,
        }),
        Box::new(GitDiff {
            modified_color: Color::Named(33),
            untracked_color: Color::Named(32),
            deleted_color: Color::Named(31),
        }),
    ],
};
```

Reorder, drop, or re-colour by editing the vec. `Color` supports:

| variant | example |
|---|---|
| `Color::Named(code)` | `30..=37`, `90..=97` for ANSI base + bright |
| `Color::Rgb(r, g, b)` | truecolor 0–255 each |
| `Color::Gradient` | only meaningful on `Context.color`; interpolates dim grey → yellow → red by usage |

## extending

Source layout (Rust 2018+ module style, no `mod.rs`):

```
src/
├── main.rs                 entry: build Renderer, write to stdout
├── statusline_renderer.rs  owns items, joins them, truncates to terminal width
├── statusline_input.rs     reads + parses stdin JSON from Claude Code
├── types.rs / types/       shared types (Color, RESET)
└── items/
    ├── context.rs          context window % with gradient
    ├── cwd.rs              shortened cwd
    ├── git/                
    │   ├── tools.rs        GitCache shared by branch + diff (one git status fork)
    │   ├── branch.rs       branch name, worktree marker, state, ahead/behind
    │   └── diff.rs         ~N +N -N counts
    └── debug/              gated behind cfg(debug_assertions), see Debug below
```

To add a new segment — e.g. `Model` showing the model name Claude Code sends:

1. **New file** `src/items/model.rs`:

```rust
use serde_json::Value;
use crate::items::{GitCache, Item};
use crate::types::Color;

pub struct Model { pub color: Color }

impl Item for Model {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let name = json.get("model")?.get("display_name")?.as_str()?;
        Some(self.color.paint(name))
    }
}
```

2. **Register** in `src/items.rs`: add `pub mod model;`.

3. **Use** in `src/main.rs` vec: add `Box::new(Model { color: Color::Named(35) })`.

Three touch-points, the segment is fully self-contained otherwise. Items receive the raw `serde_json::Value` so they own which fields they read — no central schema struct to update.

For items that need git data, take `git: &mut GitCache` — call `git.dir()` or `git.status()`. The cache lazily forks `git status` at most once per render, shared across all git items.

For items that should render on their own line below the main one (useful for multi-line debug output), override `fn standalone(&self) -> bool { true }`.

## debug

Two mechanisms, both **only active in debug builds** (`cargo build` without `--release`):

**File dump.** Every invocation writes the raw stdin JSON to `~/.claude/statusline-debug.json`, overwriting on each run. So the file always holds the latest payload Claude Code sent:

```sh
cat ~/.claude/statusline-debug.json
```

**In-statusline JSON.** The `InputFromClaudeToStatusline` item (registered in `main.rs` under `#[cfg(debug_assertions)]`) pretty-prints the JSON on its own line below the main statusline, dim grey. Useful when you want to see the contract live while iterating.

Both compile away to zero bytes in `--release` builds — the entire `items/debug/` module is gated by `#[cfg(debug_assertions)]`.

To use:

```sh
cargo build                                        # debug build
cp target/debug/statusline ~/.claude/bin/statusline
# … trigger Claude Code event, observe …
cargo build --release                              # back to release
cp target/release/statusline ~/.claude/bin/statusline
```

## performance

Median of 60 runs on Linux x86_64 (Intel i7-12700H):

| scenario | time |
|---|---|
| inside git repo | ~4.9 ms |
| outside git repo | ~2.1 ms |
| release binary size | ~424 KB |

The hot path forks `git status --branch --porcelain=v2` once and parses its output inline. Everything else (HOME shortening, ancestor walk for `.git`, state detection, terminal width via `stty`) runs in-process.

## how it works

Claude Code invokes the statusline command after each assistant message and a few other events, feeding it JSON on stdin and rendering whatever the command writes to stdout. The execution is asynchronous: a slow statusline never blocks input, but in-flight runs are cancelled when a new update fires.

This binary reads:

```json
{ "cwd": "...", "context_window": { "used_percentage": 42.5 }, "model": {...}, "workspace": {...} }
```

…and writes one ANSI-coloured line (optionally followed by additional standalone lines from debug items) back. That is the entire contract.

## license

MIT
