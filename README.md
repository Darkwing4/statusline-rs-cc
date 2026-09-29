# statusline

**[Build your line →](https://darkwing4.github.io/statusline-rs-cc/)**

A status line for Claude Code. I wrote it after failing to find one on GitHub that was simple, fast, and easy for me to understand: one Rust binary, about 5 ms per render.

<table>
  <tr>
    <td><img src="docs/screenshots/hero.png" alt="default look"/></td>
    <td><img src="docs/screenshots/states.png" alt="worktree + rebase"/></td>
  </tr>
  <tr>
    <td><img src="docs/screenshots/nogit.png" alt="outside git repo"/></td>
    <td><img src="docs/screenshots/token-spend.png" alt="session cost and token spend"/></td>
  </tr>
</table>

<p><img src="docs/screenshots/wrap.png" alt="multi-line wrap when statusline exceeds terminal width"/></p>

<table>
  <tr>
    <td><img src="docs/screenshots/ratelimit-radial.png" alt="ratelimit radial style"/></td>
    <td><img src="docs/screenshots/ratelimit-bar.png" alt="ratelimit bar+percent style"/></td>
    <td><img src="docs/screenshots/ratelimit-percent.png" alt="ratelimit plain percent style"/></td>
  </tr>
</table>

## install

1. Assemble the line in the [builder](https://darkwing4.github.io/statusline-rs-cc/) and run the install command it gives you.

2. Or clone this repo and ask Claude Code to set it up: the bundled [skill](.claude/skills/statusline-config/SKILL.md) edits the RON config and installs the binary. Without Claude Code, run the script: it installs the default line, and a copy of [`config/default.ron`](config/default.ron) at `~/.claude/statusline/config.ron` is yours to edit by hand.

   ```sh
   curl -fsSL https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.sh | sh
   ```

   Windows (PowerShell):

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.ps1 | iex"
   ```

## license

MIT
