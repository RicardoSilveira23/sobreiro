# What the two reference projects taught

Read on 2026-11-09, as [brief.md](brief.md) asked. No code was copied from either.

The brief says neither has a licence. On 2026-11-09 the taskbar widget had an MIT `LICENSE`
and the dashboard was GPL-3.0. Sobreiro was MIT when these were studied, and GPL code could not
go into it, so the rule stays for provenance: learn from them, copy nothing, and its code
stays its own.

## A now-playing taskbar widget: content in the taskbar

C# / WPF, Windows 10 and 11.

It is not a deskband and is not parented into the taskbar. It is a borderless WPF window that
sits over the taskbar's empty band:

- **Ownership, not parenting.** It sets the taskbar (`Shell_TrayWnd`, or
  `Shell_SecondaryTrayWnd` on other monitors) as its window's owner, so Windows keeps it above
  the taskbar without z-order fights, and re-asserts `HWND_TOPMOST`. It is a tool window that
  never takes focus (`WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`), so it stays out of Alt+Tab.
- **Anchors from UI Automation.** It finds the Widgets button and the Start button by their
  automation ids and the task buttons by class name, and places itself in the gap, so it never
  covers icons. It reads `TaskbarAl` to tell a centred taskbar from a left-aligned one.
- **Keeping its place.** A 600 ms position timer, a WinEvent hook on the taskbar's location so
  it rides the auto-hide slide, a foreground hook, physical-pixel positioning per monitor DPI,
  and a window region that clips it to the monitor. When Explorer restarts, the owned window
  dies with the taskbar and is recreated.
- **The cost.** About 90 KB of window code, much of it fixes for Windows updates (one for
  25H2, where the taskbar's window grew taller than its drawn band).

What it means for Sobreiro: a pill in the taskbar is possible on Windows 10 and 11 without
deskbands, but it is a lot of fragile code that Windows updates break. On 2026-11-09 Ricardo
made it milestone 3, after the flyout in milestone 2. The tray icon stays as the fallback.

## A usage dashboard: where usage data comes from

TypeScript (Bun) with a Tauri shell, GPL-3.0.

Its limit sources (`src/core/limits.ts`, `src/core/statusline.ts`), checked against the build
rules (data only from the status line bridge, no network calls, never the credentials file):

| Source | What it reads | Rules |
|---|---|---|
| Claude | `~/.claude/.credentials.json` (macOS keychain), then `GET api.anthropic.com/api/oauth/usage` with the token | Breaks all three |
| omp | runs `omp usage --json`, which asks each provider with omp's logins | Network, not the bridge |
| pi, opencode | their `auth.json` logins, sent to Anthropic, ChatGPT or GitHub | Credentials and network |
| Copilot | Copilot CLI token, keychain or `gh auth token`, then `api.github.com/copilot_internal/user` | Credentials and network |
| Codex | `rate_limits` that Codex writes into its own session logs | Local and offline, but not the bridge |
| Token and cost history | Claude Code's `~/.claude/projects/**/*.jsonl` | Local and offline, but not the bridge |
| Status line | `rate_limits.five_hour` from Claude Code's status line stdin | Keeps the rules |

Only the status line source keeps the rules, and it is the one the plan already uses. Nothing
else goes in the plan. The Codex logs and the JSONL history are local and offline but break
"data only from the bridge", so they would need Ricardo's yes before any later milestone uses
them.

How it shapes the data, which the prototype follows in its own code:

- Every field is optional; anything missing or malformed reads as "no data", never an error.
- A window carries its used fraction, its reset time and when the reading was taken.
- A reading whose reset time has passed is shown as reset, at 0%, not as the stale figure.
- Its status line command never fails: whatever goes wrong, a shorter line still prints.
