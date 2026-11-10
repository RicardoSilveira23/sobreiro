# Brief: claude-tray

**Status:** planned; see [plan.md](plan.md).

## The idea

A small Windows app that shows Claude Code's usage limits at a glance, the way
a Linux status-bar pill does: session (5-hour)
and weekly utilisation, colour shifting from neutral to warning to error as they climb, and
on hover or click, the full picture: plan tier, each limit with its reset time, the weekly
Opus and Sonnet buckets when present, and token expiry. Built for me first, then shared with
colleagues at work.

## How the Linux pill gets the data (to reuse)

Read its `services/ClaudeStatus.qml` (about 13 KB); it has no Linux-specific logic.

- Reads the access token from `~/.claude/.credentials.json`, the file the `claude` CLI writes
  on login. It exists on Windows too, at `%USERPROFILE%\.claude\.credentials.json`.
- Calls `GET https://api.anthropic.com/api/oauth/usage` with `Authorization: Bearer <token>`,
  which returns `five_hour`, `seven_day`, `seven_day_opus` and `seven_day_sonnet`, each with
  `utilization` (0 to 100) and `resets_at`; the model buckets may be `null`.
- Caches the response on disk and polls every 10 minutes, never more often than every 60
  seconds, even on a manual refresh.
- Never refreshes the token itself: when it has expired it shows "auth" and you run `/login` in
  Claude Code. It watches the credentials file and picks up the new token.

## Two projects to read before building (added 2026-11-09)

Neither has a licence, so read them to learn how they work and copy no code.

- **Placing content in the Windows taskbar:**
  a C# now-playing widget embedded in the Windows 10 and 11 taskbar. Read it for how it puts
  and keeps its own content there, since Windows 11 dropped taskbar toolbars (see "Form" below).
- **Fetching usage data for the tray or widget:**
  a TypeScript token usage dashboard. Read it for where it gets its data and how it shapes it.
  Check each source it uses against the build rules in `CLAUDE.md` (data only from the status
  line bridge, no network calls, never the credentials file). A source that breaks one goes to
  Ricardo before it goes in the plan.

## Risks to settle before building

- **The endpoint is undocumented.** It was found inside the `claude` binary and can change or
  disappear without notice. Keep the URL configurable and fail gracefully.
- **Using the token outside Claude Code.** Check Anthropic's current terms on using a Claude
  Code OAuth token in another app before sharing this at work.
- **Who it works for.** The endpoint covers Pro and Max subscription limits. Colleagues on an
  API key, a Console or Team or Enterprise organisation, Bedrock or Vertex may have no such
  limits or no OAuth token. Find out what the company uses; a fallback could sum tokens and
  cost from Claude Code's local logs (`~/.claude/projects/**/*.jsonl`), as `ccusage` does.
- **Security.** The token is a credential: never log it, print it or write it to the cache.
  For a work rollout, expect IT to ask what it reads and where it sends data.

## Questions for the planning session

- **Form.** A tray icon with the session % drawn into it and a flyout on click, a small
  always-on-top pill by the taskbar, or both. Windows 11 dropped taskbar toolbars (deskbands).
- **Stack.** C# / .NET (WPF or WinUI 3; my main language, a single self-contained `.exe`),
  Rust + Tauri, or PowerShell for a quick prototype. The .NET SDK is not installed yet.
- **Extras.** Start with Windows, toast notifications at 80% and 95%, a "resets in" countdown,
  light and dark themes, several accounts.
- **Distribution.** A GitHub release `.exe`, winget, or MSIX. An unsigned `.exe` triggers
  SmartScreen warnings, so a code-signing certificate may be needed for a work rollout.
- **Licence and visibility.** The repo is private for now. The Linux pill is MIT; to share it at
  work, make it public with MIT, or keep it internal.

## What the plan should hold

Goal, users, data source and fallbacks, UI form, stack, security rules, packaging and
distribution, milestones (a prototype first, then a polished release for colleagues).
