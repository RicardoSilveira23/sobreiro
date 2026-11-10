# Design: GitHub Copilot's AI credits

Written 2026-11-10 with Ricardo. Replaces "Later: GitHub Copilot" in [plan.md](../../plan.md).
The data findings come from Ricardo's research on his work PC (2026-11-10) and a check on his
home PC the same day. The mock-ups are in [candidates/copilot-ui](../../candidates/copilot-ui/index.html).

## Intent

Most colleagues get Copilot through the company, with a monthly AI-credit budget (Ricardo's work
seat: 15,000 credits, resetting on the 1st). GitHub shows that number only on
github.com/settings/copilot and in the editors' Copilot menus. Sobreiro should put it on the
taskbar the way it does Claude's limits: readable at a glance, without hovering or clicking. A PC
with only Claude Code shows Claude, a PC with only Copilot shows Copilot, and a PC with both shows
both. The first real test is Ricardo's work PC, with the build from this branch.

## Goals

- Copilot's AI credits used this month, as a % with its reset date, on the pill, a tray icon, the
  flyout and in toasts, next to Claude's limits or on their own.
- Live: refreshed every 5 minutes from GitHub with the sign-in the Copilot CLI already uses, and no
  setup step.
- Still something when there's no sign-in it can use: the Copilot CLI's own cached reading.

## Non-goals

- Copilot Free. It has no AI-credit budget, only chat and completions caps, which don't fit the
  "AI 17%" block. Copilot shows only when there's a capped credit budget (Business, Enterprise,
  Pro).
- Per-request costs, the org's pooled credits, or billing in dollars.
- Reading VS Code's or Visual Studio's own GitHub sign-in (encrypted in their own stores).
- A switch to turn Copilot off (Live usage has none either; see "Calls to check").

## Anti-goals

- A taskbar that needs a hover or click to show Copilot. That was option A, and it was turned down.
- Noise: a third ring at 16 px was turned down for that.
- Any change to Claude's path (bridge, `usage.json`, merge, Connect) beyond grouping the display.
- Storing, logging or renewing a GitHub token, or sending one anywhere but `api.github.com`.
- Scraping github.com with browser cookies.

## What the research settled

**The endpoint.** `GET https://api.github.com/copilot_internal/user` is undocumented. VS Code's
Copilot and the Copilot CLI use it. For a Business seat with usage-based billing it answers:

```json
{
  "copilot_plan": "business",
  "token_based_billing": true,
  "quota_reset_date_utc": "2026-12-01T00:00:00.000Z",
  "quota_snapshots": {
    "premium_interactions": {
      "entitlement": 15000, "remaining": 12355, "quota_remaining": 12355.8,
      "percent_remaining": 82.3, "unlimited": false, "has_quota": true,
      "overage_permitted": true, "overage_count": 0, "overage_entitlement": 0,
      "token_based_billing": true, "timestamp_utc": "2026-11-10T09:25:19.365Z"
    },
    "chat": { "unlimited": true },
    "completions": { "unlimited": true }
  }
}
```

- AI credits sit under `quota_snapshots.premium_interactions`, in credits despite the name. Newer
  VS Code code reads `premium_models` first when present.
- Used = `entitlement - quota_remaining`. `quota_remaining` keeps the fraction; `remaining` is
  rounded.
- On 2026-11-10 a live reply also carried `credits_used` and `quota_reset_at` (an integer, 0 on
  Free). Timestamps can carry an offset (`2026-11-09T03:06:03.986-08:00`).
- Copilot Free (home PC): `premium_interactions.entitlement` is 0 with `has_quota: false`, and
  `chat` (200) and `completions` (2,000) are capped. It is out (Non-goals).

**Sign-ins that work.** The Copilot CLI's own `gho_` token and `gh auth token` both work. The `gh`
one was verified on the home PC on 2026-11-10. The CLI's docs give its own order:
`COPILOT_GITHUB_TOKEN`, `GH_TOKEN`, `GITHUB_TOKEN`, then its sign-in in Windows Credential Manager
(service name `copilot-cli`), then `gh auth token`. Fine-grained tokens need the "Copilot Requests"
permission. Tokens from third-party OAuth apps are reported to fail. The work PC has no `gh`. The
exact Credential Manager target name there is not yet known.

**The cache.** `%LOCALAPPDATA%\copilot\copilot-user-cache.json` holds no tokens. It is JSON behind
leading `//` comment lines: `copilotUserCache` → `{ "v1:<hash>": { schemaVersion, generation,
retrievedAt, response } }`, where `response` is the endpoint's body. It updates only when the
Copilot CLI runs (possibly also when VS Code runs its bundled CLI; unverified) and is
undocumented.

## Approaches

1. **Copilot as a second provider beside Live usage** (chosen). It is a poller of its own, like
   `LiveUsage`, with its reading kept in memory and the CLI's cache as the start value. At each
   refresh, Claude's and Copilot's limits join into one list, and the display groups them by
   provider. Claude's path doesn't change.
2. Copilot written into `usage.json`. Rejected: it mixes Copilot into the bridge's file and Claude's
   merge rules, which CLAUDE.md keeps to `rate_limits`.
3. A separate helper exe. Rejected: nothing outside the app has data to hand in.

Ricardo chose the sign-in chain (all three sources, cache underneath), option G for the pill and T2
for the tray. He left the build approach to Claude.

## The design, as decided

### Where Copilot's numbers come from

Each poll tries sign-ins in the Copilot CLI's order and stops at the first one GitHub accepts:

1. The environment variables `COPILOT_GITHUB_TOKEN`, `GH_TOKEN`, then `GITHUB_TOKEN`, when set.
2. The Copilot CLI's sign-in in Credential Manager. Enumerate generic credentials with the
   filter `copilot-cli*`. If none match, enumerate all of them and keep those whose target name
   contains `copilot-cli`. Copy out only the matching blobs and free the enumeration at once.
   The blob's layout isn't known yet. It may be a bare token or wrapped in JSON. So decode it as
   UTF-8, or as UTF-16 when it has zero bytes, and take the first match of
   `(gho|ghu|ghp|github_pat)_[A-Za-z0-9_]+`. Try each match.
3. `gh auth token`: run `gh.exe` with no window and a 5 s timeout, and read stdout. Stderr is
   dropped.

A 401, 403 or 404 means "this token can't read it", and the poll moves on to the next sign-in.
So does a 200 with no counted budget (a Free account, say). The first reply with a budget wins.
Tokens are read fresh on every poll and never cached between polls, stored, logged or renewed.
This is the same rule as Claude's token.

The request is `GET https://api.github.com/copilot_internal/user` with
`Authorization: Bearer <token>`, `Accept: application/json`, `X-GitHub-Api-Version: 2025-04-01` and
`User-Agent: Sobreiro/<version>`. It has a 10 s timeout, follows no redirects and keeps no cookies,
like `LiveUsageClient`. It sends no editor headers: Sobreiro says what it is.

**When it polls:** it follows Live usage's rhythm. That means every 5 minutes, and when the flyout
opens on a reading over a minute old. A 429 waits for Retry-After or 5 minutes, doubling up to 30.
Any other failure waits 2 minutes. "No sign-in" and "all refused" retry at the normal 5 minutes,
since the user may sign in at any time.

**Reading the reply:** take `quota_snapshots.premium_models`, else `premium_interactions`. Count it
only when it isn't `unlimited` and `entitlement > 0`. The rest of the reading is:

- percent = (entitlement − quota_remaining) / entitlement × 100
- ResetsAt = `quota_reset_date_utc`
- window = one calendar month back from the reset, for the pace badge
- ObservedAt = the snapshot's `timestamp_utc`, else the cache's `retrievedAt`, else now

Every field is optional. Used comes from the first of these that is present:

1. `entitlement - quota_remaining` (this matched github.com on 2026-11-10)
2. `entitlement - remaining`
3. `entitlement × (100 - percent_remaining) / 100`
4. `credits_used`

With none of them, there's nothing to show. A reply without a counted budget means "no Copilot to
show", not an error. Over budget (`overage_count > 0` or `quota_remaining < 0`) shows as over
100%, so it reads Used up. These fields haven't been seen non-zero yet, so check them when it
happens.

Once `ResetsAt` has passed, a Copilot reading is treated the way Claude's windows are: 0%, "reset
at …". The row's age line says how old the reading is.

**The cache:** read at startup and on each poll. Drop lines that start with `//` (after leading
spaces). Take the entry with the newest `retrievedAt` among those with a counted budget, and read
its `response` as above. Entries can be from different accounts, so the newest one may be Free
and must not hide an older Business one.

**What shows:** the newer of the last live reading and the cache's reading, by ObservedAt. At
startup the cache fills in at once. A live reading takes over when it arrives. If live usage stops
working while the CLI keeps running, the cache's newer readings still come through.

### Which services show

The app checks on every refresh.

- **Claude Code** shows when it is on the PC. That means `.credentials.json` exists in the Claude
  config dir, `usage.json` has a reading, or the status line is connected (or still on the legacy
  bridge).
- **Copilot** shows when there's a Copilot reading with a counted budget, live or cached.
- With neither, the app looks as it does today: Claude's empty state.
- A service that isn't shown is gone everywhere: its pill block, tray icon, flyout section and
  tooltip lines. Its state sentences don't show either. On a Copilot-only PC, Claude's Live usage
  keeps its schedule, but with no `.credentials.json` it sends nothing, as today.
- A service appears or disappears at the next refresh, with no grace period. In practice neither
  disappears mid-session: the last reading stays in memory.

### The pill (option G)

- Both services: Claude's block as today (`5h 37%` over `7d 61%`, then the two rings), then
  Copilot's block. Copilot's block is `AI 17%` over the reset date in soft ink, then a level bar.
  The bar is 5 × 26 DIP, fully rounded, with its track in ink at 15%, and it fills from the bottom
  in the level colour (base below 70%, amber from 70%, red from 90%).
- The reset date is the day and abbreviated month as `d MMM` in the invariant culture ("1 Dec").
- Copilot only: Copilot's block alone. Claude only: as today.
- Click, right-click and hover work as today. The hover tooltip lists both services.

### The tray icons (option T2)

- There is one tray icon per service shown. Claude's is today's two rings. Copilot's is one ring:
  16 px, a 2.5 px stroke, the track at 35% and the level colours. At 16 px a lone bar reads as a
  battery, so the tray keeps a ring.
- Both icons open the same flyout on a left click and the same menu on a right click.
- Each icon's tooltip covers its own service. Copilot's reads: `AI credits 17% · resets Tue 1 Dec 12:00 AM` (the time in the user's format),
  `2,644 of 15,000 credits`, `as of 14:02`.
- Both NotifyIcons are created at startup in a fixed order (Claude, then Copilot) and are only
  shown or hidden, never recreated. WinForms numbers them in creation order, and Windows keys a
  tray icon's pinned place on the exe path and that number. So a Copilot icon first shown after
  the first poll still keeps its place across restarts.
- A toast comes from the icon of the service it is about, which is always shown when there is a
  reading to warn about.

### The flyout

- Claude's section as today under its "Claude Code" heading, spend row included. Then a "GitHub
  Copilot" heading (ruled in ink, like the first) and one "AI credits" row. The row shows % used, a
  meter with the pace tick, the badge, "resets in 20d 14h", and "2,644 of 15,000 credits"
  (rounded, invariant thousands separators).
- In the Battery and Thermometer styles, the Copilot row stays a bar row, as the spend row does.
- When Copilot's reading isn't live and fresh, its row adds one soft line. The line starts with
  the last poll's problem, if there was one:
  - "No GitHub sign-in Sobreiro can use." when no source had a token.
  - "GitHub didn't accept the sign-in." when every token was refused or had no budget.
  - "Can't reach GitHub." when the call fails.
  - "GitHub asked Sobreiro to wait." on a 429.

  It then adds the reading's source when that's the cache: "From the Copilot CLI, as of 09:25."
  A live reading over 15 minutes old says "As of 09:25." instead.
- The footer covers Claude as today. With Copilot only, it gives Copilot's "Updated … ago".

### Toasts

Copilot's credits toast at 80% and 95%, once per reset period, as Claude's windows do. Their marks
are saved in `alerts.json` like Claude's, under a new key (`copilot_credits`), so a restart
doesn't repeat a toast. An existing file without the key still loads, and Claude's keys don't
change. Balloon title: "Copilot AI credits 80% used".

### The menu

The menu is unchanged, and Connect stays Claude-only. About becomes: "Shows Claude Code's session
and weekly limits and GitHub Copilot's monthly AI credits." and "Not affiliated with or endorsed
by Anthropic or GitHub."

### Diagnostics log

Each Copilot poll logs its outcome (Ok, NoSignIn, Refused, RateLimited, Failed). It also logs,
per source kind tried (`env`, `credential-manager`, `gh`), whether it found a token and what came
back (refused, no budget, ok), and whether the cache was used. It never logs tokens, target names,
account names or numbers. That keeps to the log's rules and tells the work-PC test which path ran
and why the others didn't.

### Rules and docs that change

- **CLAUDE.md, Build rules:**
  - Data also comes from `GET https://api.github.com/copilot_internal/user`, every 5 minutes, with
    the sign-in the Copilot CLI would use, and from the CLI's cache file. Still no other network
    calls.
  - The token rule covers GitHub tokens: read per request into memory, sent only to that address,
    never renewed, logged, printed, stored, cached or committed.
- **plan.md:** Goal and UI pick up Copilot. "Later: GitHub Copilot" becomes a short summary that
  links here.
- **README** "What it reads, what it sends", and its warning near the top: GitHub's address, the
  sign-in sources, the cache file, and that the address is undocumented.
- **Site manual:** the data section (`#data`) says the same, and the part on what the app shows
  mentions Copilot. The site's pictures stay as they are.

### Where the code goes

- **`Sobreiro.Core/Copilot/`** (net10.0, all tested with xUnit):
  - parsing the endpoint's body into a reading
  - parsing the cache file
  - the sign-in order and fall-through, over injected sources
  - the HTTP client and its outcomes
  - one poll
  - the states and their sentences
  - picking between the live and cached readings

  Fixtures: the Business body above (no identifiers), the Free shape (must yield nothing), a
  cache with two comment lines and two entries, and offset timestamps.
- **`Sobreiro.Core` display and limits:**
  - `Limit` gets a `Credits` kind and provider names for "Claude Code" and "GitHub Copilot".
  - `FlyoutModel`, `PillModel`, `RingsModel`, `TooltipText` and `AlertPolicy` work per provider
    instead of assuming one.
- **`Sobreiro` (net10.0-windows):**
  - the Credential Manager reader (P/Invoke) and the `gh` runner, behind the Core interfaces
  - a `CopilotUsage` poller like `LiveUsage`
  - in `TrayController`: the second NotifyIcon and joining the limits
  - the bar block in `PillView`, the section heading in `FlyoutView`, the single ring in
    `IconRenderer`
  - `--snapshot` gains both-services and Copilot-only cases
- **Version 0.7.0.** CI builds the zip, and the PR's `Sobreiro-win-x64` artifact is the build
  Ricardo takes to the work PC.

### The work-PC test

1. Unzip and run. The pill shows Copilot's block, and the % matches github.com/settings/copilot
   within rounding. The flyout's credits and reset match too.
2. `log.txt` shows Copilot polls with outcome Ok, and which source worked.
3. A second tray icon appears in the overflow. Drag it out, restart the app, and it stays put.
4. Cut the network: after the next poll the row says "Can't reach GitHub.", and the last reading
   stays.
5. Note the Credential Manager target name (`cmdkey /list | findstr copilot`) for this doc.

## Left to the builder

- Exact spacing between the pill's two blocks (the mock-ups use the pill's own 8 DIP padding),
  and the bar's position against the text's baseline.
- How the per-provider display types are shaped in code, as long as Claude's look doesn't change.
- Where the Copilot row's extra line sits among its text lines.
- Snapshot sheet layout for the new cases.

## Calls to check

These are Claude's calls, not Ricardo's. Check them while reading:

- Sign-ins follow the Copilot CLI's own order, environment variables included. A refused token,
  or one for an account with no budget, falls through to the next.
- Copilot Free is left out.
- There's no switch to turn Copilot off.
- The pill label is "AI".
- With Copilot alone, the pill uses the bar too, but Copilot's tray icon is a ring.
- Copilot's reading stays in memory. The only new thing on disk is its toast mark in
  `alerts.json`.
- No editor headers: an honest User-Agent.
- Toasts at 80% and 95% for Copilot too.
- The Credential Manager blob is searched for a token pattern, since its layout is unknown until
  the work PC.
- After the reset date, a Copilot reading shows 0% as Claude's do, with its age underneath.
