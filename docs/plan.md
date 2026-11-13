# Plan: Sobreiro

Written 2026-11-01 from [brief.md](brief.md). Milestone 0 is next.

The app was named OpenHeadroom until 2026-11-09. Project names come from Portuguese and Armenian
symbols: Sobreiro is the Portuguese name of the cork oak, whose trunk in section is the tray's two rings.

## Goal

A Windows tray app that shows Claude Code usage limits at a glance: the same two numbers as
the VS Code extension's "Account & Usage" panel. That means session (5-hour) and weekly
(7-day) utilisation, each with its reset time. Built for me first, then published as open
source under GPL-3.0-or-later (MIT up to 0.8.1), which colleagues can install.

The same app also shows GitHub Copilot's monthly AI Credits for people who have both
subscriptions (chosen 2026-11-10; see "GitHub Copilot" below). The core stays provider-neutral,
and the project carries a neutral name: Sobreiro (formerly claude-tray).

## Users

- Me: Claude Code in the terminal and in VS Code, on a Pro or Max subscription.
- Colleagues: a mix of the CLI and the VS Code extension. Some may be on a company gateway
  with a spend limit instead of subscription limits; show that when it is present.
- Anyone on GitHub once the repo is public.

## Data source: the status line bridge

Claude Code passes a JSON object to the configured status line command on stdin. Since 2026
this object includes ([docs](https://code.claude.com/docs/en/statusline#rate-limit-usage)):

- `rate_limits.five_hour.used_percentage` (0 to 100) and `.resets_at` (Unix seconds)
- `rate_limits.seven_day.used_percentage` and `.resets_at`
- `rate_limits.spend_limit.*` behind a Claude apps gateway (percentage, reset, optional USD)

`rate_limits` is present only for Pro, Max and Ultra subscribers, or behind a gateway with a
spend limit, and only after a session's first API response. Each window may be absent on its
own. Claude Code drops a window once its `resets_at` has passed.

Flow:

1. `sobreiro-bridge.exe` is set as the `statusLine.command` in `~/.claude/settings.json`.
2. On each call it reads stdin and writes **only** the `rate_limits` fields plus an
   `observed_at` timestamp to `%LOCALAPPDATA%\Sobreiro\usage.json`. It writes atomically
   (temp file, then rename) and skips the write when stdin has no `rate_limits`.
3. If the user already had a status line command, the bridge runs it with the same stdin and
   passes its output through, so their status line keeps working. Without one, it prints a
   short default: `5h 8% · 7d 44%`.
4. The tray app watches `usage.json` and redraws.

Why the status line is the default and not the OAuth endpoint from the brief: Anthropic's terms
say developers "may not collect, store, or intermediate Claude.ai credentials or session tokens"
([legal and compliance](https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use)),
and the endpoint is undocumented. The bridge uses only documented data and never reads
`.credentials.json`.

### Live usage (always on, chosen 2026-11-09)

The status line only fires in the terminal, and I work mostly in the desktop app, so the tray
sat hours stale. Live usage fixes that the way a usage dashboard does. It is always on,
with no switch: installing the app is the consent, and people who don't want it don't install
it. The README says so near the top.

- Every 5 minutes, and when the flyout opens on a reading over a minute old, it reads the access
  token from `.credentials.json` in the Claude config dir into memory and calls
  `GET https://api.anthropic.com/api/oauth/usage` with it (`anthropic-beta: oauth-2025-04-20`,
  10 s timeout). Nothing else is sent anywhere.
- It never renews the token: renewing rotates the refresh token and would sign Claude Code out.
  An expired token, or one Anthropic refuses, pauses Live usage until Claude Code renews it.
- When Anthropic answers "too often" (429), it waits 5 minutes (or its Retry-After), doubling up
  to 30. Any other failure waits 2 minutes. The last reading stays on screen meanwhile.
- The overall session and weekly windows go through the bridge's merge into `usage.json`, so
  the icon, flyout and toasts use one state, but win within a reset period whatever the %.
  Per-model weekly limits in the reply are left out for now.
- The status line bridge keeps working beside it, and is all there is when Live usage can't fetch.

Colleagues' copies have it too. The terms risk is accepted and stated in the README.

Known limits, stated in the README:

- When Live usage can't fetch (Claude Code not signed in, the sign-in refused or expired,
  Anthropic unreachable), numbers update only when Claude Code receives a response. Between
  sessions the app shows the last known values with their age, and treats a window whose
  `resets_at` has passed as reset.
- When Live usage can't fetch, usage from claude.ai chat or the desktop app shares these limits but
  only shows up after the next Claude Code response.
- There are no Opus or Sonnet buckets, plan tier or token expiry, since none of them is in the
  status line data. Out of scope.
- The status line runs only in the terminal CLI. It never fires in the VS Code extension
  (milestone 0). The desktop app doesn't run it either (seen 2026-11-09). Live usage covers
  both while it can fetch.

## UI

- **Tray icon:** two rings (chosen 2026-11-09 for milestone 3; it was a number and a weekly
  bar until then). The outer ring fills with the session %, the inner with the weekly %,
  clockwise from 12 o'clock, each in the base colour below 70%, amber from 70% and red from 90%.
  Unfilled tracks in the taskbar's ink at 35%, rings 2.5 px at 16 px; with no data only the
  tracks show. No Anthropic or Claude logo. When Copilot shows, it gets a tray icon of its
  own: one ring, the same level colours (chosen 2026-11-10). There is one icon per service.
- **Tooltip (hover):** `Session 8% · resets 2:20 PM` / `Weekly 44% · resets Fri 12:00 AM` /
  `as of 14:02`. Copilot's icon has its own line: `AI credits 17% · resets Tue 1 Dec 12:00 AM`.
- **Flyout (left click):** a borderless window above the tray in Ricardo's Night design (chosen
  2026-11-09 from three mockups): a black or white ground following the Windows theme, plain
  Inter labels, Unbounded figures, hairlines and no shadow. Inter and Unbounded (both OFL) ship
  with the app. For each window it shows the figure and its name, a status badge, and "resets
  in 42m". The badge (chosen 2026-11-09 after a usage dashboard's) is one of "On track"
  (green), "Ends 16:40" (amber: at this pace it runs out then), "Likely used up" (amber, when
  the projected run-out has passed on an old reading), "Used up" (red), or "Not used yet", "Too
  early" and "Resetting" (grey). Below them sit a spend limit row when present, and
  "Updated 3 min ago". With Pace › Weekdays only (2026-11-12; Every day is the default) the
  projection counts Monday to Friday for windows longer than a day, so the badge, the tick and
  the alert text skip weekends ([design](superpowers/specs/2026-11-12-working-day-pace-design.md)).
- **Styles:** Bars (the default) shows the % used, with a tick where use is heading by the
  reset. Battery and Thermometer show the % left, with what the pace will use by the reset
  drawn faint. The menu sets the style and the app's config remembers it. The tray icon stays
  the two rings (% used) in every style.
- **Bar colour:** Sobreiro purple by default (`#A594FF` on dark grounds since 2026-11-09,
  when `#7B62FF` didn't stand out on the taskbar; `#5A3DD1` on light), or the Windows accent colour as an
  option. The accent is used only where it reaches 3:1 against the flyout's ground and its empty
  track in that theme. Before giving up on it, the app tries Windows' lighter accent shades on the
  dark flyout and darker ones on the light flyout (the `AccentPalette`); if none pass, the bars
  stay purple. Amber from 70% and red from 90% stay fixed.
- **Menu (right click):** Connect to Claude Code / Disconnect, Style, Bar colour, Pace, Show,
  Start with Windows, Open data folder, About, Quit.
- **Taskbar pill (milestone 3, chosen 2026-11-09):** a small borderless window over the empty
  taskbar, just left of the tray icons. It shows `5h 37%` above `7d 61%`, two lines like the
  clock, in Night type, then the tray icon's two rings (26 DIP, tracks at 15%; a flame was the
  first idea). With Copilot it gets a second block (chosen 2026-11-10): `AI 17%` over the reset
  date, then a small level bar. Clicking it opens or closes the flyout; right-clicking opens the menu; hovering
  shows the tray icon's tooltip, opened above the taskbar.
  It works the way the now-playing widget does (see [references.md](references.md)):
  - It is owned by the taskbar window rather than parented into it, and never takes focus.
  - It finds the tray icons through UI Automation, so it never covers them.
  - It hides while a full-screen app runs.

  The menu's Show choice is "Tray icon only" or "Tray icon and taskbar pill" (the default). The
  tray icon always stays. Design: [superpowers/specs/2026-11-09-taskbar-pill-design.md](superpowers/specs/2026-11-09-taskbar-pill-design.md).
- **Toasts:** once per window per reset period, at 80% and at 95%. Copilot's credits do the same,
  titled "Copilot AI credits 80% used".

## Stack

- C# on .NET 10 (LTS). WPF for the flyout, WinForms `NotifyIcon` for the tray, and no other
  UI packages.
- `Sobreiro`: WPF app, published as a self-contained single-file `.exe`.
- `Sobreiro.Bridge`: console app, published with Native AOT (2.2 MB, a run takes about 9 ms;
  see [baseline.md](baseline.md)). It runs on every status line refresh, so it must start in
  milliseconds. It is a separate exe for that reason. Publishing it needs the MSVC linker.
- `Sobreiro.Core`: shared library holding the `usage.json` model, parsing, window state
  (reset passed, age, colour level) and the settings.json editor. The UI consumes a
  provider-neutral list of limits (provider, label, percent, resets at, observed at), so a
  second provider adds a source rather than a new UI. Build no provider interface until a
  second provider exists.
- `Sobreiro.Tests`: xUnit tests for Core.
- `Sobreiro.Baseline`: console tool, not shipped. It measures a release build for
  [baseline.md](baseline.md); `scripts/baseline.ps1` runs it.
- Install the SDK with `winget install Microsoft.DotNet.SDK.10`.

```text
src/Sobreiro/          tray app
src/Sobreiro.Bridge/   status line bridge
src/Sobreiro.Core/     shared logic
tests/Sobreiro.Tests/
```

## Connecting to Claude Code

"Connect to Claude Code" edits `settings.json` in the Claude config directory
(`%USERPROFILE%\.claude`, or `CLAUDE_CONFIG_DIR` when set):

- It shows what will change and asks before writing, then backs up the file first.
- It saves any existing `statusLine.command` to the app's own config as the chained command,
  and sets `statusLine` to the bridge.
- "Disconnect" restores the previous `statusLine` exactly.
- It edits only the `statusLine` key and preserves the rest of the file as is.
- The README also documents the manual setup for people who prefer to edit the file themselves.

## Security rules

- Only Live usage reads `.credentials.json`, and only its access token, into memory, per
  request. Never renew, log, print, store, cache or commit any token.
- The bridge persists only `rate_limits` fields and a timestamp. Never the rest of the stdin
  JSON, which contains paths, the session id and the transcript path.
- No network access except Live usage's one call to `api.anthropic.com/api/oauth/usage`: no telemetry and no
  update checks. The README states this for IT.
- `usage.json` and the app config live under `%LOCALAPPDATA%\Sobreiro\`, user-only.
- `log.txt` records times, the version, exit codes and exception type names with stack traces;
  never messages, usage numbers, file contents or tokens.

## Packaging and distribution

- GitHub Actions builds, tests and publishes `Sobreiro.exe` (one file, bridge and fonts inside) on each tag.
- An unsigned `.exe` triggers SmartScreen warnings. Apply to SignPath Foundation (free code
  signing for open-source projects) once the repo is public. Fallback: Azure Trusted Signing.
- winget manifest after the first signed release.
- Since 2026-11-13 signing and winget are nice-to-haves and no release waits for either
  ([roadmap.md](roadmap.md)).
- Licence: GPL-3.0-or-later, chosen 2026-11-10 so copies stay open; MIT up to 0.8.1.
- The site is the user docs: install, setup, what it shows and the known limits (see the Site
  entry under milestone 4).

## Milestones

### 0. Verify (short session)

- Install the .NET 10 SDK.
- Add a throwaway status line command that appends stdin to a scratch file. Trigger it in
  the CLI, the VS Code extension and the desktop app, and check that `rate_limits` arrives
  in each. Restore `settings.json` afterwards.
- Note how Claude Code on Windows runs the command (shell, path quoting).
- If it does not fire in VS Code: stop and decide. Options are accepting CLI-only freshness,
  or something else; the OAuth endpoint stays out of the published app.

Results (2026-11-01, Claude Code 2.1.314):

- .NET SDK 10.0.403 installed.
- CLI: fires. `rate_limits` is absent on a session's first call and present after the first
  response, with `five_hour` and `seven_day` (`used_percentage`, `resets_at`).
- Windows runs the command through Git Bash as `bash.exe -c "<command>"`, with
  `CLAUDE_CODE_ENTRYPOINT=cli`. A double-quoted path with forward slashes and a space works,
  so the bridge path must be written for bash, not cmd.
- VS Code extension: never fires, in an open or a new session. The extension runs
  `claude.exe --output-format stream-json` with no terminal UI, so there is no status line.
- Desktop app: not installed, untested.

### 1. Prototype

- Core model and parser with tests; bridge writes `usage.json`, with chaining.
- Tray icon with the session % and tooltip. Connect manually by editing `settings.json`.
- Use it daily for a few days.

### 2. Usable app

- Flyout in Night: countdown, pace line, spend limit row, theme following Windows.
- The three styles and the bar colour option, remembered in the app's config.
- Colour levels on the icon and the bars. The pace projection and the accent contrast check
  live in Core, with tests.
- Merge readings by window, so an idle session cannot overwrite fresher numbers (from the
  milestone 1 review).
- Connect/Disconnect with backup and restore; Start with Windows (HKCU `Run` key).
- Toasts at 80% and 95%.

Results (2026-11-09), version 0.2.0:

- A hung chained status line command is stopped after 5 s and the default line is shown.
- `SOBREIRO_DATA_DIR` also gives a separate tray instance, and `--snapshot <file.png>`
  renders every style, both themes and the icon levels to one PNG.
- Windows notifications are off on Ricardo's machine, so the toasts won't show until he turns
  them on.
- Milestone 3 needs from Ricardo: the spike runs on his machine and needs him to OK a taskbar
  auto-hide toggle, a scaling change and an Explorer restart; and whether the pill starts on by
  default.
- Still open from milestone 1: Native AOT (needs the MSVC linker) and packaging, both in
  milestone 4.

### 2b. Live usage and a shorter flyout (chosen 2026-11-09)

- Live usage, always on, as described under "Data source". Its token reading, request, reply
  parsing and back-off live in Core, with tests.
- The flyout's reset and pace lines become a status badge and "resets in 42m"; toasts shrink to
  match ("Session 82% used" over "Ends 16:40 · resets in 2h").
- Version 0.3.0.

Results (2026-11-09):

- Shipped: the poller and back-off, the Live
  usage line in the flyout footer, the status badge in every style, and the shorter toasts.
- The live check on 2026-11-09 fetched within 12 s and saved only `observed_at` and
  `rate_limits`.
- A sign-in Anthropic refuses (401 or 403) is not sent again: Live usage reads the file every
  minute and fetches once Claude Code saves a new one, and the footer says so meanwhile.
- The endpoint is undocumented and Anthropic's terms don't allow third-party apps to use Claude
  sign-ins. Live usage is always on now: installing the app is the consent, and the README says so.

### 3. Taskbar pill

- First a spike on Ricardo's machine (Windows 11 25H2): place a window left of the tray icons
  and keep it there through auto-hide, a scaling change and an Explorer restart. If it can't
  be done reliably, stop and decide.
- Then the pill as described under UI, with its menu toggle, on the primary monitor first.

Results (2026-11-09), version 0.4.0:

- The spike passed all three checks on Ricardo's machine (25H2, 2560×1440): auto-hide (the
  pill hides while the taskbar slides and is down), 100% → 125% scaling (re-placed within
  0.7 s) and an Explorer restart (a new pill on the new taskbar 0.4 s after `TaskbarCreated`).
- UI Automation finds the tray in 4–30 ms on a background thread, polled every 2 s; the app
  buttons' right edge moves with no window event.
- The pill starts on (Ricardo). The tray icon became the same two rings.
- Checked live: placement left of the tray, clicks and menu, focus never taken, hidden under
  the snipping overlay (a full-screen state), Show › Tray icon only, and an Explorer restart on
  the finished build (one new pill on the new taskbar 0.9 s after it appeared). Auto-hide and
  scaling were checked with the spike only.
- The final review's fixes: the pill no longer re-asserts its z-order (it was lifting Explorer's
  taskbar above other always-on-top windows every 2 s), other taskbar items left of the tray
  count as obstacles (but never the pill's own window, which UI Automation lists under the
  taskbar: that made it blink), rings are drawn short by their round caps so 90–99% doesn't look
  full, and the pill isn't rebuilt when nothing changed.
- Found live and fixed: the purple didn't stand out on the taskbar (brighter purple, darker
  pill tracks, bigger rings; a bolder tray icon); the tooltip lost its last line (WPF leaves a
  never-activated window's tooltip unowned, so the taskbar covered it; it now opens above the
  taskbar).
- Ricardo's own tray was found stopped mid-session with nothing logged, as before; the
  diagnostics log (PR #7) is meant to catch that.

### 4. Release for colleagues

- README with screenshots, setup, limits and the "what it reads, what it sends" section.
- CI release workflow; first release tag.
- Make the repo public, apply for signing.

Progress (2026-11-09), version 0.6.0:

- `scripts/package.ps1` builds `Sobreiro-<version>-win-x64.zip`: one `Sobreiro` folder with
  `Sobreiro.exe`, `sobreiro-bridge.exe` and the font licences. The zip is self-contained (about
  76 MB), so colleagues need no .NET install.
- `.github/workflows/ci.yml` builds, tests and packages on every PR and push to `main`, and keeps
  the zip as the `Sobreiro-win-x64` artifact for 14 days. `.github/workflows/release.yml` does the
  same on a `v*` tag, checks the tag against `Directory.Build.props`, and creates a GitHub Release
  with the zip (a tag with a suffix, like `v0.6.0-rc.1`, makes a prerelease).
- The repo is still private, so colleagues get the zip from Ricardo for now.
- Still open: the first real release tag, making the repo public and signing.

Progress (2026-11-10), version 0.8.0:

- One file: the release is `Sobreiro.exe` (no version in the name), with the bridge and the fonts
  and their licences inside. `scripts/package.ps1` writes `dist\Sobreiro.exe` only; CI uploads it
  as `Sobreiro-win-x64` and the release workflow attaches it. It is 84,282,808 bytes (about 80 MB).
- Connect copies the bridge to `%LOCALAPPDATA%\Sobreiro\sobreiro-bridge.exe` and points the status
  line there; the tray keeps that copy up to date. A status line still on an older bridge gets
  Reconnect to Claude Code.
- Measured: the installed bridge takes about 62 ms per status line refresh, against 60 ms for the
  0.7.0 bridge measured the same way (20 runs through Git Bash). The status line is as fast as before.

Progress (2026-11-10), version 0.8.1:

- The taskbar pill comes back above the taskbar after a full-screen window closes (the Snipping
  Tool overlay, a full-screen photo viewer). Before, Explorer raised the taskbar past the hidden
  pill and it came back under it, covered until a click on the taskbar.

Site (2026-11-09): the user docs, in `site/`: a home page and a manual, plain HTML, CSS and a
little script, with no build step, in its own look. Design: [design.md](design.md).

- Address: <https://sobreiro.ricardosilveira.dev/>, with the manual at `/manual/`. Project
  sites live on subdomains of `ricardosilveira.dev`. It is a custom domain on the `sobreiro`
  Worker (`routes` in `wrangler.jsonc`), chosen on 2026-11-10 over GitHub Pages, which was
  never switched on. Each page's canonical link names its address there;
  `scripts/check-site.mjs` reads the domain from `wrangler.jsonc` and checks them.
- The pages said `noindex` until the repo went public on 2026-11-10; search engines may list
  them since.
- `.github/workflows/site.yml` runs the tests and the site check (`scripts/check-site.mjs`), then a `wrangler` dry
  run, on pull requests and pushes that touch it.
- Deploys: a Cloudflare Worker serving `site/` as it is.
  Each push to `main` goes to the public address, and to staging at
  <https://sobreiro.rs-staging.workers.dev>; each pull request gets
  `https://pr-<number>-sobreiro.rs-staging.workers.dev` (linked from the PR as a `preview`
  deployment). Staging and previews are behind the GitHub login (Cloudflare
  Access), so a pull request's preview is the check before anything goes public.
  `scripts/smoke.mjs` checks each one; a failed check on `main` rolls the deploy back. It stays
  off until the repository variable `DEPLOY` is `true`. The Access application covers all of
  `*.rs-staging.workers.dev`, so a new Worker needs no Access change, and the custom domain is
  outside it. To switch it on, once:
  1. A Cloudflare API token for this repo alone, `sobreiro-ci`
     (dash.cloudflare.com/profile/api-tokens): Workers Scripts Edit and Account Settings Read on
     the account in `wrangler.jsonc`, User Details Read and Memberships Read, Zone, Workers
     Routes, Edit on the `ricardosilveira.dev` zone only, no IP filter. Then
     `gh secret set CLOUDFLARE_API_TOKEN`.
  2. `CF_ACCESS_CLIENT_ID` and `CF_ACCESS_CLIENT_SECRET`: an Access service token for CI.
  3. The first deploy creates the `sobreiro` Worker, and previews only work once it exists:
     `npm ci`, then `npx wrangler deploy` once.
  4. `gh variable set DEPLOY --body true`.
- The pictures are rendered by the app itself: `scripts/media.ps1` runs `Sobreiro.exe --media`,
  encodes them into `media/`, bakes the README hero and copies the site's picks
  (`media/picks.json`). To refresh them, run it. The taskbar under the hero is one real capture
  (`scripts/capture-taskbar.ps1`).
- The site loads nothing from other hosts. Preview it with `python -m http.server` from `site/`.
- The README is shorter to match: what it is, what it reads and sends, building from source,
  the layout and the licence. Every user step lives in the manual.

Going public (2026-11-10): design in
[superpowers/specs/2026-11-10-going-public-design.md](superpowers/specs/2026-11-10-going-public-design.md).
The order:

1. Merge PR #21, then this PR.
2. Make the repo public.
3. Repo settings: description, homepage, topics; pull requests collaborators only; wiki and
   discussions off; private vulnerability reporting; the `main` ruleset (CI `build`, no
   force-push, no deletion).
4. Remove `noindex` from the site's pages.
5. Apply to SignPath Foundation for signing.

Done on 2026-11-10, all but signing. The public repository started fresh: one commit, at
0.9.0, the first release under GPL-3.0-or-later. The history before it stays private.
Secret scanning and push protection are on as well.

### 5. After release

- Signed builds, winget.
- From here to 3.0: [roadmap.md](roadmap.md).

## GitHub Copilot (chosen 2026-11-10)

Most colleagues have Copilot through the company, with a monthly AI Credits limit, and GitHub
has no documented per-user source for it. So Sobreiro asks the address VS Code's Copilot and the
Copilot CLI use, `GET https://api.github.com/copilot_internal/user`, every 5 minutes, with the
sign-in the Copilot CLI would use, and falls back on the CLI's cache file. The token is never
stored, logged or renewed. Copilot Free has no credit budget and isn't shown. Design:
[superpowers/specs/2026-11-10-copilot-usage-design.md](superpowers/specs/2026-11-10-copilot-usage-design.md).

## Risks and open points

- **The status line runs only in the CLI.** Neither the VS Code extension nor the desktop app
  runs it, so their usage shows up after the next response in the terminal.
- **Name.** Settled on Sobreiro (no GitHub repo used it on 2026-11-01). It keeps
  "Claude" and "Copilot" out of the product name, as trademark guidelines require. Mention
  them only in plain text ("works with Claude Code") and use no logos. Still to do: rename
  the GitHub repo and the local folder.
- **Field changes.** The `rate_limits` fields are documented but young. The parser treats
  every field as optional and shows "no data" rather than failing.
- **Several Claude Code sessions at once** all write `usage.json`. They report the same
  account, so readings merge by window: within one reset period the higher used % stays, and a
  new period, or a new spend cap, replaces the old.
- **Several accounts or config dirs:** out of scope for now.
- **The taskbar pill depends on the taskbar's undocumented layout.** Windows updates move and
  resize its parts (in 25H2 the taskbar's window grew taller than its drawn band). The tray
  icon stays as the fallback, and the pill can be turned off. Its window is owned by Explorer's
  taskbar across processes, which ties the two threads' input together, so the tray's UI thread
  must never block.
