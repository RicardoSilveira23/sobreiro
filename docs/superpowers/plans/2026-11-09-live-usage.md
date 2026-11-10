# Live usage and a shorter flyout: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An opt-in "Live usage" mode that asks Anthropic for the account's usage every 5 minutes with Claude Code's sign-in, so the tray stays current while Ricardo works in the desktop app; and a flyout that says each limit's state in one badge instead of two sentences.

**Architecture:** Core gains a `Live` namespace: reading the access token, the request and reply, and the schedule with back-off, all tested without network (a fake `HttpMessageHandler`) and without real credentials (temp files). Live readings go through the existing `UsageMerge` into `usage.json`, so the icon, flyout and toasts keep one state. Core's flyout model swaps the reset and pace lines for a `LimitStatus` badge and "resets in 42m". The tray adds a menu toggle with a consent box, a poller, and draws the badge.

**Tech Stack:** C# / .NET 10, xUnit, `System.Net.Http.HttpClient` (in Core, AOT-safe), `JsonDocument`, WPF for the badge.

**Spec:** [docs/plan.md](../../plan.md): "Data source" › "Live usage (opt-in, chosen 2026-11-09)", "UI" › "Flyout", "Security rules", milestone "2b". [CLAUDE.md](../../../CLAUDE.md) build rules (changed 2026-11-09 for Live usage).

## Global Constraints

- C# / .NET 10. Logic goes in `OpenHeadroom.Core` with an xUnit test; Core stays `net10.0`, `IsAotCompatible`, no reflection-based JSON.
- Warnings as errors; the build ends with 0 warnings.
- The only network call in the whole app: `GET https://api.anthropic.com/api/oauth/usage`, only while Live usage is on. Headers: `Authorization: Bearer <token>`, `anthropic-beta: oauth-2025-04-20`, `Accept: application/json`, `User-Agent: OpenHeadroom/<version>`. 10 s timeout. Redirects are not followed.
- Only Live usage reads `.credentials.json` (in `ClaudePaths.ConfigDirectory`), and only `claudeAiOauth.accessToken` and `claudeAiOauth.expiresAt`, into memory, per request. Never renew the token (renewing rotates the refresh token and signs Claude Code out). Never log, print, store, cache, or put the token in an exception message, a `ToString()`, `usage.json`, `config.json` or any file.
- Tests never read the real `%USERPROFILE%\.claude\.credentials.json` and never touch the network: temp files and a fake `HttpMessageHandler`.
- Ricardo's live setup keeps working: never touch his real `settings.json`, `%LOCALAPPDATA%\OpenHeadroom\`, HKCU `Run` key or `C:\Dev\openheadroom\publish\`. Manual runs use `OPENHEADROOM_DATA_DIR` and `CLAUDE_CONFIG_DIR` scratch folders.
- Night rules for labels: sentence case, plain words, no tracking.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Decisions this plan makes

- **Consent every time** Live usage is switched on (a rare action), so no extra config key. Config key `live_usage` (bool, default false).
- **Schedule:** fetch when due. Due: at switch-on and app start (when on), 5 minutes after a success, 2 minutes after a failure, unreachable or 401/403; after a 429 the wait is `max(Retry-After, step)` with step 5, 10, 20, 30, 30… minutes, reset by a success. With no or expired credentials nothing is sent and the file is checked again after 1 minute. Opening the flyout fetches early when the last success is over 1 minute old, unless a 429 wait is running.
- **Reply:** prefer the `limits` list (`group` `session`/`weekly`, `percent`, `resets_at` ISO 8601, unscoped entries only); without it, the keyed `five_hour` / `seven_day` objects (`utilization`, `resets_at`). Per-model, `oauth_apps` and spend entries are ignored. `percent`/`utilization` are 0–100.
- **Badge rules,** in order: `Resetting` when not reset and the reset is under 30 s away; `Not used yet` when reset or under 2% used; `Used up` at 100%; `Ends <time>` when the pace runs out before the reset (time of day, with the day name when not today, as `When.Format`); `On track` otherwise; `Too early` when the pace has under 5% of the window to go on; no badge when there is no reset time or window length.
- **Badge colours** (text on fill, all ≥ 4.5:1): On track `#1A7F37` / `#FFFFFF` both themes; Ends dark `#FFB020`/`#0B0B0B`, light `#A35F00`/`#FFFFFF`; Used up dark `#FF6B6B`/`#0B0B0B`, light `#C4262E`/`#FFFFFF`; grey (Not used yet, Too early, Resetting) dark `#262626`/`#F4F4F4`, light `#E4E4E4`/`#0D0D0D`.
- **Toasts:** title `Session 82% used`; body `Ends 16:40 · resets in 2h` (badge text, then the resets-in text; either may be missing).
- **Flyout footer** adds one sentence when Live usage is on and not working: expired `Live usage is paused until Claude Code renews its sign-in.`; no credentials `Live usage needs Claude Code to be signed in.`; 429 `Anthropic asked Live usage to wait.`; other failures `Live usage can't reach Anthropic.`
- `ColumnText` goes: the new lines are short enough for the columns.
- Version 0.3.0.

## Review Focus

- The token reaching anything printable: an exception message, `ToString()`, `Debug.WriteLine`, a file → `AccessToken.ToString()` is redacted and every failure path is checked (Task 1 tests `Token_never_appears_in_ToString_or_results`).
- An expired or missing token → no request at all (Task 1 test with a handler that fails when called).
- A redirect from the endpoint → not followed, so the token can't be sent to another host (Task 1 test: 302 → `Failed`, handler sees one request).
- 429 with `Retry-After` in seconds or as an HTTP date → the wait honours it and doubles to the 30-minute cap (Task 1 schedule tests).
- A reply with missing fields, a new shape, per-model entries or non-numeric values → those parts ignored, never an exception (Task 1 parser tests).

## File Structure

```text
src/OpenHeadroom.Core/
  Connect/ClaudePaths.cs           + CredentialsFile(getEnv)
  Live/AccessToken.cs              token + expiry, redacted ToString
  Live/ClaudeCredentials.cs        read .credentials.json → CredentialRead
  Live/UsageApiJson.cs             reply → RateLimits?
  Live/LiveUsageClient.cs          one GET → LiveResult
  Live/LiveSchedule.cs             when to ask next, back-off
  Live/LiveState.cs                Off / Ok / Expired / NoSignIn / RateLimited / Failing + footer text
  Live/LiveUsageMessage.cs         consent and About text
  Config/AppConfig.cs              + LiveUsage (live_usage)
  Display/LimitStatus.cs           badge kind + text
  Display/Pace.cs                  + RunsOutAt
  Display/FlyoutModel.cs           LimitView: Status, ResetsIn instead of ResetText/PaceText/PaceLate; live footer
  Display/ColumnText.cs            removed
  Colours/NightPalette.cs          + Badge(StatusKind)
  Alerts/AlertPolicy.cs            new title and body
src/OpenHeadroom/
  LiveUsage.cs                     poller: timer, HttpClient, merge into usage.json, state
  TrayMenu.cs, TrayController.cs   Live usage toggle with consent; flyout open → early fetch; footer state; About
  Flyout/FlyoutView.cs             badge pill + resets-in line in all three styles
  Snapshot.cs                      + a row with every badge in both themes
Directory.Build.props              Version 0.3.0
README.md, docs/plan.md            Live usage, what it reads and sends, known limits
```

---

### Task 1: Core Live usage: token, request, reply, schedule

**Files:** create `src/OpenHeadroom.Core/Live/*.cs` (AccessToken, ClaudeCredentials, UsageApiJson, LiveUsageClient, LiveSchedule, LiveState, LiveUsageMessage); modify `Connect/ClaudePaths.cs`, `Config/AppConfig.cs`, `Directory.Build.props`. Tests: `ClaudeCredentialsTests`, `UsageApiJsonTests`, `LiveUsageClientTests`, `LiveScheduleTests`, `LiveStateTests`; extend `AppConfigTests`, `ClaudePathsTests`.

**Interfaces (produces):**
- `string ClaudePaths.CredentialsFile(Func<string, string?> getEnv)` → `<config dir>\.credentials.json`.
- `sealed class AccessToken(string value, DateTimeOffset? expiresAt)` with `string Value`, `DateTimeOffset? ExpiresAt`, `ToString()` → `"AccessToken(redacted)"`.
- `enum CredentialStatus { Ok, Missing, Unreadable, Expired }`; `record CredentialRead(CredentialStatus Status, AccessToken? Token)`; `CredentialRead ClaudeCredentials.Read(string path, DateTimeOffset now)` (`expiresAt` is Unix milliseconds; at or before now → Expired; no `claudeAiOauth.accessToken` string → Unreadable; IO errors other than not-found → Unreadable).
- `RateLimits? UsageApiJson.Parse(string json)`.
- `enum LiveOutcome { Ok, Unauthorized, RateLimited, Failed, Unreachable }`; `record LiveResult(LiveOutcome Outcome, RateLimits? Limits, TimeSpan? RetryAfter)`; `sealed class LiveUsageClient(HttpClient http, string userAgent)` with `Task<LiveResult> FetchAsync(AccessToken token, DateTimeOffset now, CancellationToken ct)`. Constants `LiveUsageClient.Endpoint`, `Timeout` (10 s).
- `record LiveSchedule(DateTimeOffset NextDue, DateTimeOffset? LastSuccess, int RateLimitStep, DateTimeOffset? WaitUntil)`; `LiveSchedule.Start(DateTimeOffset now)`; `LiveSchedule AfterFetch(LiveResult result, DateTimeOffset now)`; `LiveSchedule AfterNoCredentials(DateTimeOffset now)`; `bool IsDue(DateTimeOffset now)`; `bool FetchOnOpen(DateTimeOffset now)`.
- `enum LiveState { Off, Ok, Expired, NoSignIn, RateLimited, Failing }`; `string? LiveStateText.Footer(LiveState state)` (the four sentences under Decisions; null for Off and Ok).
- `string LiveUsageMessage.Consent(string credentialsPath)` and `LiveUsageMessage.AboutLine` (below).
- `AppConfig.LiveUsage` (bool, key `live_usage`, default false), kept through load and save.

- [ ] **Step 1: Write the failing tests**
  - Credentials (temp files): valid file with `expiresAt` in the future → Ok with the token; past → Expired (token not returned); missing file → Missing; `{}`, `[]`, broken JSON, `accessToken` a number → Unreadable; no `expiresAt` → Ok with null expiry. `Token_never_appears_in_ToString_or_results`: `AccessToken.ToString()`, `CredentialRead.ToString()` and `LiveResult.ToString()` never contain the token string.
  - Parser: the `limits` list shape (`{"limits":[{"group":"session","percent":27,"resets_at":"2026-11-09T14:00:00Z"},{"group":"weekly","percent":74,"resets_at":"2026-11-13T00:00:00Z"},{"group":"weekly","percent":40,"scope":{"model":"Fable"}}]}`) → five-hour 27 and seven-day 74 with those times, the scoped entry ignored; the keyed shape (`five_hour.utilization` 27.5, `seven_day.utilization` 74, `seven_day_opus`, `seven_day_oauth_apps` present) → 27.5 and 74; a list with no usable entry falls back to the keys; missing `resets_at` → null reset; a string `percent` → that window dropped; nothing usable, `[]`, broken JSON → null.
  - Client (fake handler): 200 with the list shape → Ok with limits, and the handler saw exactly the URL, `Authorization: Bearer t0k3n`, `anthropic-beta: oauth-2025-04-20`, the user agent; 401 and 403 → Unauthorized; 429 with `Retry-After: 120` → RateLimited, 2 min; 429 with an HTTP date 3 minutes after `now` → about 3 min; 500 → Failed; 302 → Failed with one request seen; 200 with an unparseable body → Failed; the handler throwing `HttpRequestException` or a timeout → Unreachable. No result or exception text contains `t0k3n`.
  - Schedule: `Start` is due now; after Ok due at +5 min and `FetchOnOpen` false until 1 min after the success, true after; after Failed/Unreachable/Unauthorized due at +2 min; after 429 without Retry-After due at +5, then +10, +20, +30, +30 with `FetchOnOpen` false while waiting; Retry-After 45 min on the first 429 → +45; an Ok resets the step; `AfterNoCredentials` due at +1 min.
  - `LiveStateText.Footer`: the four sentences; Off and Ok → null.
  - AppConfig: `live_usage` round trip; missing → false; unknown keys still kept.
- [ ] **Step 2: Run, see them fail** — `dotnet test`: compile errors.
- [ ] **Step 3: Implement.** `HttpRequestMessage` per call; `HttpCompletionOption.ResponseHeadersRead` not needed. The client never follows redirects itself: a 3xx is `Failed` (the tray's handler also sets `AllowAutoRedirect = false`). `LiveUsageMessage.Consent(path)`: "Live usage reads the sign-in Claude Code keeps in {path} and sends it to api.anthropic.com every 5 minutes to fetch your usage. It is never stored, logged or renewed, and nothing else is sent.\n\nTurn on Live usage?" `AboutLine`: "With Live usage on, it asks Anthropic for your usage every 5 minutes. Otherwise it makes no network calls." Version 0.3.0.
- [ ] **Step 4: Run the tests** — all pass, 0 warnings.
- [ ] **Step 5: Commit** — `Core: Live usage token, request, reply and schedule`

### Task 2: Core: badges and shorter text

**Files:** create `Display/LimitStatus.cs`; modify `Display/Pace.cs`, `Display/FlyoutModel.cs`, `Colours/NightPalette.cs`, `Alerts/AlertPolicy.cs`; delete `Display/ColumnText.cs` and its tests. Tests: `LimitStatusTests`, extend `PaceTests`, `FlyoutModelTests`, `AlertPolicyTests`, `NightPaletteTests` (or `BarColourTests`).

**Interfaces:**
- Consumes: `LiveState` (Task 1).
- Produces:
  - `PaceProjection(double ProjectedPercent, TimeSpan? RunsOutBeforeReset, DateTimeOffset? RunsOutAt)`.
  - `enum StatusKind { OnTrack, Ends, UsedUp, NotUsedYet, TooEarly, Resetting }`; `record LimitStatus(StatusKind Kind, string Text)`; `LimitStatus? LimitStatus.For(Limit limit, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)`.
  - `LimitView(LimitKind Kind, string Label, bool HasData, string UsedFigure, string LeftFigure, double UsedFraction, double? ProjectedFraction, UsageLevel Level, LimitStatus? Status, string? ResetsIn, string? Detail)`.
  - `string? FlyoutModel.ResetsIn(Limit limit, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)`: `resets in 42m`; after a reset `reset at 14:00`; null without a reset time. Missing window views: `ResetsIn` "No reading yet", `Status` null.
  - `FlyoutModel.From(limits, now, zone, culture, LiveState live = LiveState.Off)`: the footer gets `LiveStateText.Footer(live)` as a second sentence when not null.
  - `record BadgeColours(Rgb Fill, Rgb Text)`; `BadgeColours NightPalette.Badge(StatusKind kind)` per palette (values under Decisions).
  - Alerts: title `{Label} {DisplayPercent}% used`; body `{Status.Text} · {ResetsIn}`, or whichever exists.
- [ ] **Step 1: Failing tests** — `LimitStatus` per rule with now = Mon 2026-11-09 10:00 UTC, en-US: 27% at 109/300 min elapsed → On track; 74% weekly at 83 h of 168 → `Ends Tue 3:09 PM` (run-out = observed + 29.16 h = Tue 15:09:44; the clock drops seconds); 83% session at 150/300 min → `Ends 10:30 AM` (run-out = observed + 30.7 min); 100% → Used up; 1% → Not used yet; reset → Not used yet; 4% elapsed → Too early; reset 20 s away → Resetting; no window → null. `FlyoutModel`: the mockup reading's views carry those statuses and `resets in 3h 11m` / `resets in 3d 13h`; the footer with `LiveState.Expired` is `Updated 3 min ago. Live usage is paused until Claude Code renews its sign-in.` (Footer sentences joined with ". " — make the existing footer end without a full stop and join consistently; assert the exact string.) Badge colours: each `Text` on `Fill` ≥ 4.5:1 in both palettes, values as listed. Alerts: the 80% session alert's title `Session 80% used`, body `On track · resets in 3h` for an on-track reading.
- [ ] **Step 2: Run, see them fail.**
- [ ] **Step 3: Implement**; delete `ColumnText` and its tests; fix every caller in Core (the tray is Task 4; leave it compiling by updating `FlyoutView` minimally to the new fields if the build needs it, or note it).
- [ ] **Step 4: Run the tests** — all pass, 0 warnings.
- [ ] **Step 5: Commit** — `Core: status badges and shorter flyout and toast text`

### Task 3: Tray: Live usage toggle and poller

**Files:** create `src/OpenHeadroom/LiveUsage.cs`; modify `TrayMenu.cs`, `TrayController.cs`.

**Interfaces:** consumes Task 1 (`ClaudeCredentials`, `LiveUsageClient`, `LiveSchedule`, `LiveState`, `LiveUsageMessage`, `AppConfig.LiveUsage`, `ClaudePaths.CredentialsFile`) and `UsageMerge`, `UsageFile`, `AppPaths.UsageFile()`.

- [ ] **Step 1: Poller.** `LiveUsage` owns one `HttpClient` over `new SocketsHttpHandler { AllowAutoRedirect = false }`, a `LiveSchedule`, and a `LiveState`. `Tick(now)` (called from the tray's existing 30 s tick and on start) does nothing when off; when due it reads the credentials (no token → `AfterNoCredentials`, state NoSignIn or Expired, no request), else fetches on a background task; on the dispatcher it merges an Ok reading into `usage.json` (`UsageFile.Read` → `UsageMerge.Merge` → `UsageFile.Write`), updates the schedule and state, and asks the controller to refresh. One fetch at a time. `FlyoutOpened(now)` fetches when `FetchOnOpen`. Failures log only the outcome name with `Debug.WriteLine`, never a token or a reply body.
- [ ] **Step 2: Menu.** `Live usage` (checked from config) after `Bar colour`. Switching on shows `LiveUsageMessage.Consent(credentialsPath)` in a Yes/No box titled `Live usage`; Yes reloads the config, sets `LiveUsage = true`, saves (same error handling as Style), starts the poller with `LiveSchedule.Start`. Switching off saves false, stops it, state Off. About gains `LiveUsageMessage.AboutLine` in place of "It makes no network calls."
- [ ] **Step 3: Flyout and footer.** Opening the flyout (left click or second launch) calls `FlyoutOpened`; `FlyoutModel.From` gets the current `LiveState`.
- [ ] **Step 4: Check it.** Build (0 warnings), `dotnet test`. Run the built tray with scratch `OPENHEADROOM_DATA_DIR` and `CLAUDE_CONFIG_DIR` (a scratch config dir has no credentials, so NoSignIn shows in the footer and no request is sent). Do not switch it on against the real credentials here; the controller does the live check.
- [ ] **Step 5: Commit** — `Tray: Live usage toggle with consent, poller and footer state`

### Task 4: Tray: the badge in the flyout, toasts, snapshot

**Files:** modify `Flyout/FlyoutView.cs` (and any style files under `Flyout/`), `Snapshot.cs`, `TrayController.cs` (toast text comes from Core already).

- [ ] **Step 1: Badge.** A pill: `Border` with corner radius 999, fill and text from `NightPalette.Badge(kind)`, padding 10 × 2, Inter Medium 13 px, no shadow. Bars rows: label and figure, meter with its tick, then one line with the badge and, 8 px after it, `ResetsIn` in Soft 14 px; `Detail` under it for spend. Battery and Thermometer columns: glyph, figure + unit, label (Inter Medium), the badge centred, `ResetsIn` centred in Soft 13 px, 4–8 px gaps as now. No data: no badge, `ResetsIn` ("No reading yet") only. The footer may wrap to two lines.
- [ ] **Step 2: Snapshot.** The sample reading now also includes a third row showing every badge kind in both themes, so all states can be checked at once.
- [ ] **Step 3: Check it.** Build (0 warnings), `dotnet test`, `--snapshot` and look at the sheet: badges readable, nothing wraps mid-phrase, both themes.
- [ ] **Step 4: Commit** — `Tray: status badge in every style; snapshot shows each badge`

### Task 5: Docs

- [ ] README: a "Live usage" section (what it reads, what it sends, how often, how to turn it off, that the endpoint is undocumented and Anthropic's terms don't cover third-party apps using the sign-in, so it's off by default); "How it works" and Known limits updated (numbers stay current in every app with Live usage on; otherwise only after terminal responses); the flyout description with the badge.
- [ ] docs/plan.md "Known limits": the first two bullets now hold only with Live usage off; milestone 2b results.
- [ ] Commit — `Docs: Live usage and the status badge`
