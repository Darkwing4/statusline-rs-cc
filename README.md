# statusline

Minimal, fast, extensible Claude Code statusline. Single Rust binary, ~5 ms per invocation, no runtime deps, new segment in 3 touch-points.

<table>
  <tr>
    <td><img src="docs/screenshots/hero.png" alt="default look"/></td>
    <td><img src="docs/screenshots/states.png" alt="worktree + rebase"/></td>
  </tr>
  <tr>
    <td><img src="docs/screenshots/nogit.png" alt="outside git repo"/></td>
    <td><img src="docs/screenshots/debug.png" alt="debug item below statusline"/></td>
  </tr>
</table>

## cheat sheet

Codex-inspired: dim middot separator, soft teal cwd, gradient percentage, green branch, bright-red git state. Outside a git repo only context and cwd render.

| segment | colour | example | notes |
|---|---|---|---|
| context | grey → yellow → red gradient | `42%` | dim grey under 20%, then ramps |
| cwd | soft teal `#5fafaf` | `~/proj` | `$HOME` shortened to `~` |
| git branch | green | `main`, `⑂feature` | `⑂` only inside a worktree (resolved by reading `.git` gitfile, no fork) |
| git state | bright red | `[REBASE 2/5]` | `MERGE`, `REBASE n/m`, `CHERRY-PICK`, `REVERT`, `BISECT`, `AM n/m` — only during the op |
| ahead/behind | branch colour | `(↑2 ↓1)` | only when non-zero |
| diff | yellow / green / red | `~2 +1 -1` | modified tracked / untracked / deleted |
| separator | bright black | ` · ` | middot |

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

## configuration & extending

The whole config is a literal `Renderer { ... }` in [`src/main.rs`](src/main.rs):

```rust
let renderer = Renderer {
    separator: " · ",
    separator_color: Color::Named(90),
    items: vec![
        Box::new(Context { color: Color::Gradient, prefix: "", prefix_color: Color::Rgb(180, 142, 173), suffix: "", suffix_color: Color::Rgb(180, 142, 173) }),
        Box::new(Cwd { color: Color::Rgb(95, 175, 175) }),
        Box::new(GitBranch { color: Color::Named(32), state_color: Color::Named(91), show_worktree: true, show_ahead_behind: true, show_state: true }),
        Box::new(GitDiff { modified_color: Color::Named(33), untracked_color: Color::Named(32), deleted_color: Color::Named(31) }),
    ],
};
```

Reorder, drop, or re-colour by editing the vec. `Color` variants: `Named(code)` for ANSI 30–37 / 90–97, `Rgb(r, g, b)` for truecolor, `Gradient` (only meaningful on `Context`).

**Adding a new segment** — say `Model` showing the model name. Three touch-points:

1. `src/items/model.rs`:

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

2. `src/items.rs` — add `pub mod model;`.
3. `src/main.rs` — drop `Box::new(Model { color: Color::Named(35) })` into the vec.

Items receive the raw `serde_json::Value` so they own which fields they read — no central schema to update. For git-aware items take `git: &mut GitCache` and call `git.dir()` / `git.status()` — `git status` is forked at most once per render, shared. For items that render on their own line below the main one (multi-line debug output), override `fn standalone(&self) -> bool { true }`.

<details>
<summary>source layout</summary>

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

</details>

## debug

Two mechanisms, both **only active in debug builds** (`cargo build` without `--release`):

- **File dump** — every invocation writes raw stdin JSON to `~/.claude/statusline-debug.json`, overwriting. Always the latest payload Claude Code sent.
- **In-statusline JSON** — the `InputFromClaudeToStatusline` item pretty-prints the JSON on its own line below the main statusline, dim grey (see screenshot above). Useful for watching the contract live while iterating.

Both compile away to zero bytes in `--release` — the entire `items/debug/` module is gated by `#[cfg(debug_assertions)]`.

```sh
cargo build && cp target/debug/statusline ~/.claude/bin/statusline                # poke around
cargo build --release && cp target/release/statusline ~/.claude/bin/statusline    # back to prod
```

## under the hood

Claude Code invokes the statusline after each assistant message (and a few other events), feeds JSON on stdin, renders whatever lands on stdout. Execution is async — a slow statusline never blocks input, in-flight runs are cancelled on update. Contract:

```json
{ "cwd": "...", "context_window": { "used_percentage": 42.5 }, "model": {...}, "workspace": {...} }
```

Hot path on Linux x86_64 (i7-12700H, median of 60): **~4.9 ms** inside a git repo, **~2.1 ms** outside, **~424 KB** stripped release binary. `git status --branch --porcelain=v2` forks once; everything else (HOME shortening, `.git` ancestor walk, state detection, terminal width via `stty`) runs in-process.

## license

MIT
