# Copilot usage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show GitHub Copilot's monthly AI credits beside Claude Code's limits (or alone) on the
pill, a tray icon of its own, the flyout and toasts, read live from GitHub with the sign-in the
Copilot CLI uses, falling back to the CLI's cache.

**Architecture:**
- Copilot is a second provider beside Live usage. A `CopilotUsage` poller in the app runs
  `CopilotPoll` from Core every 5 minutes and keeps the latest reading in memory. Core parses the
  CLI's cache, and the newer of the live and cached readings is shown.
- `TrayController` joins Claude's limits (from `usage.json`, unchanged) with Copilot's into one
  `Limit` list. The display models split that list by provider.

**Tech Stack:** C# / .NET 10, WPF + WinForms NotifyIcon, xUnit, `System.Text.Json`, advapi32
`CredEnumerateW`.

**Spec:** [docs/superpowers/specs/2026-11-10-copilot-usage-design.md](../specs/2026-11-10-copilot-usage-design.md)

## Global Constraints

- **Request:** `GET https://api.github.com/copilot_internal/user` with these headers and nothing
  else:
  - `Authorization: Bearer <token>`
  - `Accept: application/json`
  - `X-GitHub-Api-Version: 2025-04-01`
  - `User-Agent: Sobreiro/<version>`

  It has a 10 s timeout, no redirects and no cookies (reuse `LiveUsageClient.CreateHandler()`),
  and no editor headers.
- **Tokens:** read fresh on every poll, sent only to that address, never cached between polls,
  stored, logged, printed or renewed. No token may appear in a result record, an exception message
  or a log line.
- **Diagnostics log:** never logs tokens, credential target names, account names or usage numbers.
- **Claude's path stays as it is:** the bridge, `usage.json`, `UsageMerge`, Connect, the Live
  usage request. So does Claude's look when Copilot isn't shown.
- **Logic** goes in `Sobreiro.Core` (net10.0) with xUnit tests. P/Invoke and NotifyIcon code go
  in `Sobreiro` (net10.0-windows).
- **Level colours** come from the existing `UsageLevels.For` / `colours.Fill(level)`: base below
  70%, amber from 70%, red from 90%.
- **Copy is verbatim from the spec:**
  - "No GitHub sign-in Sobreiro can use."
  - "GitHub didn't accept the sign-in."
  - "Can't reach GitHub."
  - "GitHub asked Sobreiro to wait."
  - "From the Copilot CLI, as of {time}."
  - "As of {time}."
  - "{used} of {budget} credits"
  - Headings "Claude Code" and "GitHub Copilot"; the row label "AI credits"; the pill label "AI".
- **Plan decisions on copy:**
  - The balloon title is "Copilot AI credits 80% used", to match Claude's "Session 80% used".
  - The pill's date is invariant `d MMM` ("1 Dec"), like `When.Format`.

  Task 10 updates the spec to match both.
- **Version:** `Directory.Build.props` goes to `0.7.0`.

## Review Focus

1. A Credential Manager blob that is UTF-16, has a BOM, or wraps the token in JSON must still yield
   the token. Pinned in Task 3.
2. `gh` missing or hanging must neither throw nor stall the poll beyond its 5 s. Pinned in Task 3.
3. A cache file that's being written (truncated JSON) or locked must give no reading without
   throwing, and the last good cache reading must stay. Pinned in Tasks 2 and 9.
4. A Copilot reading whose reset has passed must show 0% "reset at …" and fire no toast. Pinned in
   Task 6.
5. With both services shown, Claude's tooltip and footer ages must ignore Copilot's newer
   ObservedAt, so they never claim Claude is fresher than it is. Pinned in Task 7.

---

### Task 1: Read the endpoint's reply

**Files:**
- Create: `src/Sobreiro.Core/Copilot/CopilotReading.cs`
- Create: `src/Sobreiro.Core/Copilot/CopilotUserJson.cs`
- Test: `tests/Sobreiro.Tests/CopilotUserJsonTests.cs`

**Interfaces:**
- Produces:
  - `public enum CopilotSource { Live, Cache }`
  - `public sealed record CopilotReading(double Budget, double Used, DateTimeOffset? ResetsAt, DateTimeOffset ObservedAt, CopilotSource Source)`
    with `public double Percent => Used / Budget * 100;`
  - `public static class CopilotUserJson`:
    - `public static CopilotReading? Parse(string body, DateTimeOffset now)`, which gives a `Live`
      reading
    - `internal static CopilotReading? FromResponse(JsonElement response, DateTimeOffset fallbackObservedAt, CopilotSource source)`

- [ ] **Step 1: Write the failing tests.** Use the spec's Business body as `Business` (no
  identifiers). `Now = 2026-11-10T12:00:00Z`.

```csharp
[Fact] public void Business_seat_reads_used_from_quota_remaining()
{
    var r = CopilotUserJson.Parse(Business, Now)!;
    Assert.Equal(15000, r.Budget);
    Assert.Equal(2644.2, r.Used, 1);
    Assert.Equal(17.628, r.Percent, 3);
    Assert.Equal(DateTimeOffset.Parse("2026-12-01T00:00:00Z"), r.ResetsAt);
    Assert.Equal(DateTimeOffset.Parse("2026-11-10T09:25:19.365Z"), r.ObservedAt);
    Assert.Equal(CopilotSource.Live, r.Source);
}
[Fact] public void Premium_models_wins_over_premium_interactions()      // premium_models entitlement 300, quota_remaining 150 → Budget 300, Used 150
[Fact] public void Free_plan_has_no_budget()                            // entitlement 0, has_quota false, chat/completions capped → null
[Fact] public void Unlimited_has_no_budget()                            // unlimited true, entitlement 15000 → null
[Fact] public void Used_falls_back_to_remaining()                       // no quota_remaining, remaining 12355 → Used 2645
[Fact] public void Used_falls_back_to_percent_remaining()               // only percent_remaining 82.3 → Used 2655 (15000 × 17.7 / 100)
[Fact] public void Used_falls_back_to_credits_used()                    // only credits_used 2644.2 → Used 2644.2
[Fact] public void No_used_field_means_nothing()                        // entitlement only → null
[Fact] public void Overage_count_reads_over_budget()                    // quota_remaining 0, overage_count 210 → Used 15210, Percent 101.4
[Fact] public void Negative_quota_remaining_reads_over_budget()         // quota_remaining -50 → Used 15050
[Fact] public void Offset_timestamp_is_kept()                           // "2026-11-09T03:06:03.986-08:00" → ObservedAt.UtcDateTime 11:06:03.986
[Fact] public void Missing_timestamp_uses_now()                         // no timestamp_utc → ObservedAt == Now
[Fact] public void Missing_reset_date_gives_null_ResetsAt()
[Theory, InlineData(""), InlineData("[]"), InlineData("{"), InlineData("{\"quota_snapshots\":5}")]
public void Malformed_bodies_give_null(string body)
```

- [ ] **Step 2: Run `dotnet test --filter CopilotUserJsonTests`.** Expected: it fails to compile
  (the types don't exist yet).

- [ ] **Step 3: Implement both files.**
  - The snapshot is `quota_snapshots.premium_models` if it's an object, else
    `premium_interactions`.
  - Budget = `entitlement`. Count the snapshot only if `unlimited` isn't `true` and Budget > 0.
  - Used is the first that's present of:
    1. `Budget - quota_remaining`
    2. `Budget - remaining`
    3. `Budget × (100 - percent_remaining) / 100`
    4. `credits_used`
  - Then, if `overage_count > 0` and Used ≤ Budget, Used = Budget + overage_count.
  - ResetsAt = root `quota_reset_date_utc`. ObservedAt = the snapshot's `timestamp_utc`, else
    the fallback.
  - Every field is optional. Wrong types count as absent, and `JsonException` gives null.

- [ ] **Step 4: Run `dotnet test --filter CopilotUserJsonTests`.** Expected: all pass.

- [ ] **Step 5: Commit** `Copilot: read AI credits from copilot_internal/user`.

### Task 2: Read the Copilot CLI's cache

**Files:**
- Create: `src/Sobreiro.Core/Copilot/CopilotCache.cs`
- Test: `tests/Sobreiro.Tests/CopilotCacheTests.cs`

**Interfaces:**
- Consumes: `CopilotUserJson.FromResponse` from Task 1.
- Produces: `public static class CopilotCache` with:
  - `public static string DefaultPath(Func<string, string?> getEnv)`, which is
    `%LOCALAPPDATA%\copilot\copilot-user-cache.json` (`LOCALAPPDATA`, else
    `Environment.SpecialFolder.LocalApplicationData`)
  - `public static CopilotReading? Parse(string text)`
  - `public static CopilotReading? Read(string path)`, which gives null for a missing,
    unreadable or locked file and never throws

- [ ] **Step 1: Write the failing tests.** The cache text starts with two `// …` lines, then
  `{"copilotUserCache":{"v1:aaa":{"schemaVersion":1,"generation":"g","retrievedAt":"…","response":{…}}, …}}`.

```csharp
[Fact] public void Newest_entry_with_a_budget_wins()       // v1:a Business at 09:25, v1:b Business at 10:00 → Used from v1:b
[Fact] public void Newer_free_entry_does_not_hide_business() // v1:a Business 09:25, v1:b Free 11:00 → v1:a's reading
[Fact] public void Reading_is_marked_cache()                 // Source == CopilotSource.Cache
[Fact] public void Retrieved_at_is_used_when_snapshot_has_no_timestamp()
[Fact] public void Comment_lines_with_leading_spaces_are_dropped()   // "   // x"
[Fact] public void No_entries_with_a_budget_gives_null()
[Theory, InlineData(""), InlineData("// only a comment"), InlineData("{\"copilotUserCache\":{\"v1:a\":{\"retrievedAt\":\"x\",\"response\":{")]
public void Malformed_or_truncated_cache_gives_null(string text)
[Fact] public void Missing_file_gives_null()                  // Read(Path.Combine(temp, "none.json")) == null
[Fact] public void Locked_file_gives_null()                   // open with FileShare.None, then Read → null
[Fact] public void Default_path_is_under_local_app_data()     // getEnv("LOCALAPPDATA") = @"C:\L" → @"C:\L\copilot\copilot-user-cache.json"
```

- [ ] **Step 2: Run `dotnet test --filter CopilotCacheTests`.** Expected: it fails to compile.
- [ ] **Step 3: Implement.** Drop lines whose trimmed start is `//`, then parse. Entries whose
  `retrievedAt` doesn't parse are skipped. Of the entries with a budget, the latest `retrievedAt`
  wins. `Read` catches `IOException` and `UnauthorizedAccessException`.
- [ ] **Step 4: Run `dotnet test --filter CopilotCacheTests`.** Expected: all pass.
- [ ] **Step 5: Commit** `Copilot: read the Copilot CLI's cached reading`.

### Task 3: Find tokens: environment, blobs, `gh`

**Files:**
- Create: `src/Sobreiro.Core/Copilot/CopilotToken.cs`
- Create: `src/Sobreiro.Core/Copilot/GhCli.cs`
- Test: `tests/Sobreiro.Tests/CopilotTokenTests.cs`
- Test: `tests/Sobreiro.Tests/GhCliTests.cs`

**Interfaces:**
- Produces:
  - `public enum CopilotSourceKind { Env, CredentialManager, Gh }`
  - `public sealed record CopilotSignIn(CopilotSourceKind Kind, Func<IReadOnlyList<string>> Tokens)`.
    `Tokens` is called only when the poll reaches that source, and it may throw: the poll treats a
    throw as "no token".
  - `public static class CopilotToken`:
    - `public static string? Find(string? text)` returns the first match of
      `(gho|ghu|ghp|github_pat)_[A-Za-z0-9_]+`, else null.
    - `public static string? FromBlob(ReadOnlySpan<byte> blob)` decodes UTF-16LE when any byte is
      0, else UTF-8 (BOMs ignored), then runs `Find`.
    - `public static IReadOnlyList<string> FromEnvironment(Func<string, string?> getEnv)` returns
      `COPILOT_GITHUB_TOKEN`, `GH_TOKEN` and `GITHUB_TOKEN`, in that order. Each is trimmed and
      non-empty, distinct, and taken as-is (an env token need not match the pattern).
  - `public static class GhCli`:
    - `public static IReadOnlyList<string> Token()` is `Run("gh", "auth token", TimeSpan.FromSeconds(5))`.
    - `internal static IReadOnlyList<string> Run(string exe, string arguments, TimeSpan timeout)`
      runs the process with no window and stdout and stderr redirected. A timeout kills the process
      tree. It returns `Find(stdout)` as a one-item list only when the exit code is 0, and an empty
      list on any failure, including `Win32Exception` when there's no such exe. It never throws.

- [ ] **Step 1: Write the failing tests.** Use a fake token string such as
  `"gho_" + new string('a', 36)`, never a real one.

```csharp
[Fact] public void Finds_a_bare_token()
[Fact] public void Finds_a_token_inside_json()               // {"oauth_token":"gho_…","user":"x"}
[Fact] public void Finds_a_fine_grained_token()              // github_pat_11AB…
[Fact] public void Text_without_a_token_gives_null()
[Fact] public void Utf16_blob_with_bom_is_decoded()          // Encoding.Unicode.GetPreamble() + GetBytes(json)
[Fact] public void Utf8_blob_is_decoded()
[Fact] public void Environment_order_is_copilot_gh_github()  // all three set → [copilot, gh, github]
[Fact] public void Blank_and_duplicate_environment_values_are_skipped()
// GhCliTests
[Fact] public void Missing_exe_gives_no_token()              // GhCli.Run(@"C:\nope\gh-missing.exe", "auth token", 5 s) is empty, no throw
[Fact] public void Printed_token_is_returned()                // GhCli.Run("cmd.exe", "/c echo gho_aaaa…", 5 s) == ["gho_aaaa…"]
[Fact] public void Non_zero_exit_gives_no_token()             // GhCli.Run("cmd.exe", "/c echo gho_aaaa… & exit 1", 5 s) is empty
[Fact] public void Hanging_exe_is_killed_after_the_timeout() // GhCli.Run("ping.exe", "-n 30 127.0.0.1", 1 s) is empty; elapsed < 4 s
```

- [ ] **Step 2: Run `dotnet test --filter "CopilotTokenTests|GhCliTests"`.** Expected: it fails
  to compile.
- [ ] **Step 3: Implement both files.** `GhCli` reads stdout asynchronously before waiting, so a
  full pipe can't deadlock it.
- [ ] **Step 4: Run the same filter.** Expected: all pass.
- [ ] **Step 5: Commit** `Copilot: find sign-ins in the environment, blobs and gh`.

### Task 4: Ask GitHub

**Files:**
- Create: `src/Sobreiro.Core/Copilot/CopilotUsageClient.cs`
- Modify: `src/Sobreiro.Core/Live/LiveUsageClient.cs`. Make `RetryAfter(HttpResponseMessage,
  DateTimeOffset)` `internal static` so the two clients share it. No behaviour change.
- Test: `tests/Sobreiro.Tests/CopilotUsageClientTests.cs`

**Interfaces:**
- Consumes: `CopilotUserJson.Parse` from Task 1.
- Produces:
  - `public enum CopilotOutcome { Ok, NoBudget, Refused, RateLimited, Failed, Unreachable }`
  - `public sealed record CopilotFetch(CopilotOutcome Outcome, CopilotReading? Reading, TimeSpan? RetryAfter)`
  - `public sealed class CopilotUsageClient`:
    - `public const string Endpoint = "https://api.github.com/copilot_internal/user";`
    - `public CopilotUsageClient(HttpClient http, string userAgent)`, plus an internal overload
      with a timeout, as in `LiveUsageClient`
    - `public Task<CopilotFetch> FetchAsync(string token, DateTimeOffset now, CancellationToken ct)`

- [ ] **Step 1: Write the failing tests.** Copy `FakeHandler`, `ThrowingHandler` and `Reply` from
  `LiveUsageClientTests`.

```csharp
[Fact] public async Task Sends_the_four_headers_and_nothing_else()
{   // Authorization "Bearer t0k3n"; Accept application/json; X-GitHub-Api-Version 2025-04-01; User-Agent Sobreiro/0.7.0;
    // RequestUri == Endpoint; no "Editor-Version" header; method GET
}
[Fact] public async Task Ok_with_a_budget_gives_the_reading()          // 200 + Business body → Ok, Reading.Used 2644.2
[Fact] public async Task Ok_without_a_budget_is_NoBudget()             // 200 + Free body → NoBudget, Reading null
[Theory, InlineData(401), InlineData(403), InlineData(404)]
public async Task Refusals(int code)                                   // → Refused
[Fact] public async Task Too_many_requests_carries_retry_after()       // 429 + Retry-After: 120 → RateLimited, 2 min
[Fact] public async Task Server_error_is_Failed()                      // 500 → Failed
[Fact] public async Task Unparsable_200_is_Failed()                    // 200 "{" → Failed
[Fact] public async Task Redirect_is_Failed()                          // 302 → Failed
[Fact] public async Task Network_error_is_Unreachable()                // HttpRequestException → Unreachable
[Fact] public async Task Timeout_is_Unreachable()                      // handler delays past an internal 50 ms timeout
[Fact] public async Task Caller_cancellation_throws()                  // pre-cancelled token → OperationCanceledException
[Fact] public async Task Token_never_appears_in_the_result()           // Assert.DoesNotContain("t0k3n", result.ToString())
```

- [ ] **Step 2: Run `dotnet test --filter CopilotUsageClientTests`.** Expected: it fails to
  compile.
- [ ] **Step 3: Implement it like `LiveUsageClient.FetchAsync`.** The request never leaves the
  method, a `FormatException` from the header is Failed, and only the caller's cancellation
  escapes.
- [ ] **Step 4: Run `dotnet test --filter "CopilotUsageClientTests|LiveUsageClientTests"`.**
  Expected: all pass.
- [ ] **Step 5: Commit** `Copilot: ask GitHub for the AI credits`.

### Task 5: One poll, its schedule and its sentences

**Files:**
- Create: `src/Sobreiro.Core/Copilot/CopilotPoll.cs`
- Create: `src/Sobreiro.Core/Copilot/CopilotState.cs`
- Modify: `src/Sobreiro.Core/Live/LiveSchedule.cs`. Add `AfterNoReading`.
- Test: `tests/Sobreiro.Tests/CopilotPollTests.cs`
- Test: `tests/Sobreiro.Tests/CopilotStateTests.cs`
- Test: add one test to `tests/Sobreiro.Tests/LiveScheduleTests.cs`

**Interfaces:**
- Consumes: `CopilotSignIn` and `CopilotSourceKind` (Task 3), and `CopilotUsageClient` and
  `CopilotOutcome` (Task 4).
- Produces:
  - `public LiveSchedule AfterNoReading(DateTimeOffset now)`, which is
    `this with { NextDue = now + 5 min, WaitUntil = now + 5 min, Refused = null }`. That means no
    retry on flyout open either.
  - `public enum CopilotState { Ok, NoSignIn, Refused, RateLimited, Failing }`
  - `public sealed record CopilotPollResult(LiveSchedule Schedule, CopilotState State, CopilotReading? Reading, string Log)`
  - `public static class CopilotPoll` with
    `public static Task<CopilotPollResult> RunAsync(IReadOnlyList<CopilotSignIn> signIns, CopilotUsageClient client, LiveSchedule schedule, DateTimeOffset now, CancellationToken ct)`
  - `public static class CopilotShown` with
    `public static CopilotReading? Pick(CopilotReading? live, CopilotReading? cache)`: the later
    ObservedAt wins, and a tie goes to live.
  - `public static class CopilotStateText` with
    `public static string? Line(CopilotState state, CopilotReading reading, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)`

**The poll** goes through the sources in order. A source's `Tokens()` throwing counts as no
tokens. For each token it fetches:

| Fetch outcome | What the poll does |
|---|---|
| `Ok` | Return `Ok`, the reading, and `AfterFetch(new LiveResult(LiveOutcome.Ok, null, null), now)` |
| `Refused`, `NoBudget` | Go on to the next token or source |
| `RateLimited` | Stop: `RateLimited`, `AfterFetch(RateLimited with RetryAfter)` |
| `Failed`, `Unreachable` | Stop: `Failing`, `AfterFetch(Failed)` |

If the end is reached, the state is `Refused` if any token was found, else `NoSignIn`, with
`AfterNoReading(now)`.

`Log` is `copilot <State> env=<r> credential-manager=<r> gh=<r>`. Each `<r>` is one of `none`,
`refused`, `no-budget`, `ok`, `error`, `rate-limited`, `failed` or `skipped` (not reached). For a
source with several tokens, it records the last result.

**The sentences:**
- The problem comes first:

  | State | Sentence |
  |---|---|
  | `NoSignIn` | "No GitHub sign-in Sobreiro can use." |
  | `Refused` | "GitHub didn't accept the sign-in." |
  | `Failing` | "Can't reach GitHub." |
  | `RateLimited` | "GitHub asked Sobreiro to wait." |
  | `Ok` | none |

- Then the source:
  - For `Cache`: "From the Copilot CLI, as of {When.Format(ObservedAt)}."
  - For `Live` with `now - ObservedAt > 15 min`: "As of {When.Format(ObservedAt)}."
- The two are joined with a space. If neither applies, the line is null.

- [ ] **Step 1: Write the failing tests.** Use a fake `CopilotUsageClient` over `FakeHandler` that
  answers per `Authorization` value, and sign-ins built from lists.

```csharp
[Fact] public async Task First_accepted_token_wins_and_later_sources_are_not_called()
{   // env ["A"] → 200 Business; gh Tokens() throws if called → State Ok, Reading not null,
    // Log == "copilot Ok env=ok credential-manager=skipped gh=skipped", Schedule.LastSuccess == Now
}
[Fact] public async Task Refused_and_no_budget_fall_through()           // env A→401, cm B→200 Free, gh C→200 Business → Ok; "env=refused credential-manager=no-budget gh=ok"
[Fact] public async Task All_refused_is_Refused_with_a_five_minute_wait() // → Refused, NextDue == Now+5m, FetchOnOpen(Now+2m) false
[Fact] public async Task No_tokens_anywhere_is_NoSignIn()              // "env=none credential-manager=none gh=none", no request sent
[Fact] public async Task Throwing_source_is_error_and_skipped()         // cm Tokens() throws → "credential-manager=error", goes on to gh
[Fact] public async Task Rate_limit_stops_the_chain()                   // env 429 → RateLimited; gh not called
[Fact] public async Task Network_failure_stops_the_chain()              // env HttpRequestException → Failing, NextDue Now+2m
[Fact] public async Task Log_never_holds_a_token()                      // tokens "gho_…" → Assert.DoesNotContain("gho_", result.Log)
// CopilotStateTests
[Fact] public void Fresh_live_ok_has_no_line()                          // null
[Fact] public void Old_live_ok_says_as_of()                             // 20 min old → "As of 09:25." (en-GB, UTC zone)
[Fact] public void Cache_says_from_the_copilot_cli()                    // "From the Copilot CLI, as of 09:25."
[Fact] public void Problem_comes_before_source()                        // Failing + cache → "Can't reach GitHub. From the Copilot CLI, as of 09:25."
[Theory] // each state → its exact sentence, fresh live reading
[Fact] public void Pick_takes_the_newer_and_live_on_a_tie()
// LiveScheduleTests
[Fact] public void After_no_reading_waits_five_minutes_and_blocks_open()
```

- [ ] **Step 2: Run `dotnet test --filter "CopilotPollTests|CopilotStateTests|LiveScheduleTests"`.**
  Expected: it fails to compile.
- [ ] **Step 3: Implement.** `CopilotPoll` throws only `OperationCanceledException` for `ct`.
- [ ] **Step 4: Run the same filter.** Expected: all pass, and the existing `LiveScheduleTests`
  still pass.
- [ ] **Step 5: Commit** `Copilot: one poll through the sign-in chain`.

### Task 6: Copilot as a limit, presence and toasts

**Files:**
- Create: `src/Sobreiro.Core/Limits/Providers.cs`
- Modify: `src/Sobreiro.Core/Limits/Limit.cs`
- Modify: `src/Sobreiro.Core/Alerts/AlertState.cs`
- Modify: `src/Sobreiro.Core/Alerts/AlertPolicy.cs`
- Test: `tests/Sobreiro.Tests/LimitListTests.cs`, `AlertStateTests.cs` and `AlertPolicyTests.cs`
  (add tests)

**Interfaces:**
- Consumes: `CopilotReading` (Task 1).
- Produces:
  - `public static class Providers { public const string Claude = "Claude Code"; public const string Copilot = "GitHub Copilot"; }`.
    `LimitList` uses `Providers.Claude` in place of its private const.
  - `public enum LimitKind { Session, Weekly, Spend, Credits }`, with `Credits` added last.
  - `Limit` gains the optional trailing parameters `double? UsedCredits = null, double? CreditBudget = null`.
  - `public static IReadOnlyList<Limit> LimitList.FromCopilot(CopilotReading? reading, DateTimeOffset now)`
    returns one Credits limit, or none. Its fields:
    - Provider `Providers.Copilot`, Label "AI credits", Percent = `reading.Percent`
    - `Window = ResetsAt - ResetsAt.AddMonths(-1)` (null without ResetsAt)
    - `IsReset` and 0% once ResetsAt ≤ now, as `Make` does
  - `public static bool LimitList.ShowClaude(bool claudeOnPc, bool copilotShown)`, which is
    `claudeOnPc || !copilotShown`
  - `public static bool LimitList.ClaudeOnPc(bool credentialsFileExists, int claudeLimitCount, ConnectionState connection)`,
    which is `credentialsFileExists || claudeLimitCount > 0 || connection != ConnectionState.NotConnected`
  - The `AlertState` key for `Credits` is `"copilot_credits"`. `Load` and `Save` include it, and
    old files without it still load.
  - `AlertPolicy`: for `Credits` the title is `$"Copilot {Label} {pct}% used"`; other kinds don't
    change.

- [ ] **Step 1: Write the failing tests.**

```csharp
[Fact] public void Copilot_reading_becomes_one_credits_limit()      // Provider "GitHub Copilot", Kind Credits, Label "AI credits", 17.628, UsedCredits 2644.2, CreditBudget 15000, Window 30 days (Nov)
[Fact] public void No_copilot_reading_gives_no_limits()
[Fact] public void Passed_reset_reads_zero_and_reset()             // ResetsAt 2026-11-01, now 2026-11-10 → Percent 0, IsReset
[Fact] public void Claude_shows_when_on_pc_or_when_nothing_else_shows()   // truth table of ShowClaude
[Fact] public void Claude_is_on_pc_by_credentials_readings_or_connection() // truth table of ClaudeOnPc, Legacy counts
// AlertStateTests
[Fact] public void Credits_mark_round_trips_as_copilot_credits()   // Save then raw JSON has "copilot_credits"; Load gives the mark back
[Fact] public void Old_file_without_credits_still_loads()          // {"session":{"resets_at":…,"threshold":80}} → session mark only
// AlertPolicyTests
[Fact] public void Credits_at_80_toasts_with_copilot_title()       // Title "Copilot AI credits 80% used"
[Fact] public void Reset_credits_never_toast()                     // IsReset → no alert
[Fact] public void Claude_titles_unchanged()                       // existing "Session 80% used" still
```

- [ ] **Step 2: Run `dotnet test`.** Expected: the new tests fail, and the existing ones still
  compile.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run `dotnet test`.** Expected: everything passes.
- [ ] **Step 5: Commit** `Copilot: credits as a limit, with its own toast mark`.

### Task 7: Display models per provider

**Files:**
- Modify: `src/Sobreiro.Core/Display/Rings.cs`, `PillModel.cs`, `FlyoutModel.cs` and
  `TooltipText.cs`
- Test: `tests/Sobreiro.Tests/RingsModelTests.cs`, `PillModelTests.cs`, `FlyoutModelTests.cs` and
  `TooltipTextTests.cs` (add tests; existing ones must pass unchanged)

**Interfaces:**
- Consumes: `LimitKind.Credits`, `Providers` and `UsedCredits`/`CreditBudget` (Task 6). The
  Copilot note is a string computed by the caller with `CopilotStateText.Line` (Task 5), because a
  `Limit` doesn't know whether its reading came from the cache.
- Produces:
  - **Rings:** `public sealed record RingsModel(Ring Outer, Ring Inner, bool HasInner = true)`,
    plus `public static RingsModel Copilot(IReadOnlyList<Limit> limits)`, which is
    `new(Ring.From(credits), Ring.None, HasInner: false)`.
  - **Pill:**
    - `public sealed record PillCopilot(PillLine Credits, string ResetDate, double Fraction, UsageLevel Level)`.
      Credits is `new PillLine("AI", "17%")`. ResetDate is the local reset date as
      `"d MMM"`, invariant culture, or "–". Fraction is clamped to 0..1.
    - `PillModel` becomes
      `(PillLine Session, PillLine Weekly, RingsModel Rings, PillCopilot? Copilot = null, bool ShowClaude = true)`.
    - The new overload `public static PillModel From(IReadOnlyList<Limit> limits, TimeZoneInfo zone, bool showClaude)`
      sets Copilot from the Credits limit. `From(limits)` stays as it is.
  - **Flyout:**
    - `public sealed record CopilotSection(LimitView Credits, string? Note)`. The Credits view's
      Detail is `"{used:#,0} of {budget:#,0} credits"` (invariant, used rounded).
    - `FlyoutModel` gains `CopilotSection? Copilot = null, bool ShowClaude = true`.
    - `From(...)` gains the optional `string? copilotNote = null, bool showClaude = true`.
    - The title is always `Providers.Claude`.
    - The footer's age and its "No numbers yet" use only Claude's limits when `ShowClaude`, and
      only Copilot's otherwise. When Claude is hidden, Live usage's sentence and the legacy
      sentence are left out.
  - **Tooltip:**
    - `TooltipText.Render` uses only Claude's kinds for its lines and its "as of".
    - New `public static string RenderCopilot(Limit credits, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)`
      gives `"AI credits 17% · resets {When.Format}\n2,644 of 15,000 credits\nas of {When.Format}"`,
      cut to `MaxLength`.

- [ ] **Step 1: Write the failing tests.**

```csharp
[Fact] public void Copilot_rings_have_one_ring()                        // HasInner false, Outer.Percent 17
[Fact] public void Pill_has_copilot_block_when_credits_present()        // Copilot.Credits == ("AI","17%"), ResetDate "1 Dec" (UTC zone), Fraction ≈ .176
[Fact] public void Pill_reset_date_is_local()                           // zone UTC-5 → "30 Nov"
[Fact] public void Pill_without_credits_has_no_copilot_block()
[Fact] public void Pill_hides_claude_when_told()                        // showClaude false → ShowClaude false
[Fact] public void Flyout_copilot_section_has_detail_and_note()         // Detail "2,644 of 15,000 credits"; copilotNote "Can't reach GitHub." → Note
[Fact] public void Flyout_title_stays_claude_with_copilot_first_in_list()
[Fact] public void Flyout_footer_ignores_copilot_age_when_claude_shown() // Claude 3 h old, Copilot just now → "Updated 3h 0m ago"-style per AgeText
[Fact] public void Flyout_copilot_only_footer_uses_copilot_age()        // showClaude false → "Updated 2 min ago"; no Live usage sentence
[Fact] public void Claude_tooltip_ignores_copilot()                     // "as of" from Claude's ObservedAt; no "AI credits" line
[Fact] public void Copilot_tooltip_has_three_lines()                    // exact text for en-US, UTC; Length ≤ 127
```

- [ ] **Step 2: Run `dotnet test`.** Expected: the new tests fail.
- [ ] **Step 3: Implement.** Existing callers and tests keep compiling through the defaults.
- [ ] **Step 4: Run `dotnet test`.** Expected: everything passes.
- [ ] **Step 5: Commit** `Display: Claude and Copilot as separate parts of the pill, flyout and tooltips`.

### Task 8: Draw Copilot: pill block, flyout section, single-ring icon

**Files:**
- Modify: `src/Sobreiro/Pill/PillView.cs`
- Modify: `src/Sobreiro/Flyout/FlyoutView.cs`
- Modify: `src/Sobreiro/IconRenderer.cs`
- Modify: `src/Sobreiro/Snapshot.cs`
- Modify: `src/Sobreiro/TrayMenu.cs` (the About text)

**Interfaces:**
- Consumes: `PillModel.Copilot` and `ShowClaude`, `FlyoutModel.Copilot` and `ShowClaude`, and
  `RingsModel.HasInner` (Task 7).

**What it draws:**
- **PillView:**
  - If `ShowClaude`, Claude's block as today.
  - If `Copilot` is set, a block 12 DIP to its right: two lines (`AI` + figure, then the reset
    date in ink at 60% alpha, Inter 11), then 8 DIP, then a level bar.
  - The bar is a 5 × 26 DIP rounded rectangle with radius 2.5. Its track is ink at
    `TaskbarColours.TrackAlpha`. It fills from the bottom by `Fraction` in
    `colours.Fill(Level)`, clipped to the rounded shape.
  - The hover background covers the whole pill.
- **FlyoutView:**
  - If `ShowClaude`, as today.
  - If `Copilot` is set: a ruled "GitHub Copilot" heading (the same `Ruled(Text(…,15,Soft), Ink, …)`,
    with a 10 DIP top margin when Claude is above it), then `BarRow(Credits)` in every style.
  - If `Note` is set, add it as one soft 14 px wrapping line under the row's lines.
- **IconRenderer:** skip the inner ring when `!model.HasInner`.
- **TrayMenu About:** "Shows Claude Code's session and weekly limits and GitHub Copilot's monthly AI
  credits. " + the existing `LiveUsageMessage.AboutLine`, then "Not affiliated with or endorsed by
  Anthropic or GitHub."
- **Snapshot:** add a row with a both-services pill and a Copilot-only pill (the sample is
  2,644 / 15,000, resetting 1 Dec), both flyouts (dark and light, Bars style), and the Copilot
  icon at 16, 20, 24 and 32 px.

- [ ] **Step 1: Run `dotnet build Sobreiro.slnx`.** Expected: 0 warnings, 0 errors.
- [ ] **Step 2: Render the snapshot.** Run
  `dotnet run --project src/Sobreiro -- --snapshot <scratchpad>/sheet.png` and look at the new
  row. It should match `docs/candidates/copilot-ui` options G, E and T2 and the flyouts. Claude's
  existing cells must look unchanged.
- [ ] **Step 3: Run `dotnet test`.** Expected: everything passes.
- [ ] **Step 4: Commit** `Draw Copilot on the pill, the flyout and its own tray icon`.

### Task 9: Wire it up: sign-ins, poller, second tray icon

**Files:**
- Create: `src/Sobreiro/Copilot/CredentialStore.cs`
- Create: `src/Sobreiro/CopilotUsage.cs`
- Modify: `src/Sobreiro/TrayController.cs`
- Modify: `src/Sobreiro/App.xaml.cs` (pass the log)

**Interfaces:**
- Consumes: everything above.
- Produces:
  - `internal static class CredentialStore` with `public static IReadOnlyList<string> CopilotTokens()`:
    - `CredEnumerateW("copilot-cli*", 0, …)`. On `ERROR_NOT_FOUND`, call `CredEnumerateW(null, 0, …)`
      and keep entries whose `TargetName` contains `copilot-cli` (OrdinalIgnoreCase).
    - Copy each kept blob into a byte array, `CopilotToken.FromBlob` it, and zero the copy.
    - `CredFree` in `finally`. Return a distinct list. Never log names.
  - `internal sealed class CopilotUsage : IDisposable` has the shape of `LiveUsage`:
    - `CopilotUsage(Action changed, Action<string> log)`
    - `Start(now)`, `Tick(now)`, `FlyoutOpened(now)`, `Dispose()`
    - `CopilotState State` (Ok until the first poll answers)
    - `CopilotReading? Shown => CopilotShown.Pick(_live, _cache)`
  - Inside `CopilotUsage`:
    - The sign-ins are `[Env: CopilotToken.FromEnvironment(Environment.GetEnvironmentVariable), CredentialManager: CredentialStore.CopilotTokens, Gh: () => GhCli.Token()]`.
    - The poll and the cache read run on `Task.Run`. The cache comes from
      `CopilotCache.Read(CopilotCache.DefaultPath(...))` at `Start` and on every poll. A null
      cache read keeps the previous cache reading.
    - It logs `result.Log + " cache=" + (Shown?.Source == Cache ? "used" : cache is null ? "none" : "older")`
      only when that text differs from the last line logged.
  - `TrayController(Action<string>? log = null)`:
    - **Icons:** create `_notifyIcon` and then `_copilotIcon` in the constructor (fixed order).
      Both share `_menu.Strip` and `OnMouseClick`, and `_copilotIcon` starts with
      `Visible = false`.
    - **Polling:** start, tick and flyout-open `_copilot` beside `_live`.
    - **Refresh:**
      - The limits are `LimitList.From(_last, now)` filtered by `showClaude`, plus
        `LimitList.FromCopilot(_copilot.Shown, now)`.
      - `showClaude = LimitList.ShowClaude(LimitList.ClaudeOnPc(File.Exists(ClaudePaths.CredentialsFile(Environment.GetEnvironmentVariable)), claudeCount, ConnectFlow.CurrentState()), copilotShown)`.
      - `_notifyIcon.Visible = showClaude` and `_copilotIcon.Visible = copilotShown`.
      - The Copilot icon is drawn from `RingsModel.Copilot` with the `RenderCopilot` tooltip.
      - The flyout gets `copilotNote: _copilot.Shown is { } r ? CopilotStateText.Line(_copilot.State, r, now, zone, culture) : null`.
      - The pill tooltip is Claude's tooltip (if shown) plus `"\n"` plus Copilot's (if shown).
    - **Alerts:** split by provider. Claude's alerts make a balloon on `_notifyIcon`, Credits
      alerts on `_copilotIcon`.
    - **Dispose:** dispose both icons and `_copilot`.
  - `App.xaml.cs`: `new TrayController(e => _log?.Write(e))`.

- [ ] **Step 1: Implement.** Run `dotnet build Sobreiro.slnx`. Expected: 0 warnings, 0 errors.
- [ ] **Step 2: Run `dotnet test`.** Expected: everything passes.
- [ ] **Step 3: Check it live on this PC.** It has Claude, `gh` signed in to a Copilot Free
  account, and a Free-only cache. Quit the running Sobreiro first, then run the built app with
  `SOBREIRO_DATA_DIR=<scratchpad>/data` so the real data folder isn't touched. Expected:
  - Claude's pill and icon are unchanged, and there's no Copilot block or icon.
  - `<scratchpad>/data/log.txt` holds one line,
    `copilot Refused env=none credential-manager=none gh=no-budget cache=none`.
  - Nothing in the log looks like a token: `grep -E "gh[opu]_|github_pat_"` finds nothing.
- [ ] **Step 4: Commit** `Copilot: poll GitHub and show a second tray icon`.

### Task 10: Rules, docs and version

**Files:**
- Modify: `CLAUDE.md` (Build rules: the two bullets in the spec's "Rules and docs that change")
- Modify: `docs/plan.md`:
  - The Goal paragraph.
  - UI: the pill gets the Copilot block, and the tray gets an icon per service.
  - "Later: GitHub Copilot" becomes "GitHub Copilot (chosen 2026-11-10)": a 5-line summary
    linking the spec.
- Modify: `README.md`:
  - The warning near the top names GitHub's undocumented address too.
  - "What it reads, what it sends" gets the GitHub request, the sign-in sources in order, the
    cache file, and "the token is never stored, logged or renewed".
  - "That is its only network call" becomes "These are its only network calls".
- Modify: `site/manual/index.html`. Section `#data` says the same as the README, and the part on
  what the app shows mentions Copilot's credits, its pill block and its tray icon.
- Modify: `docs/superpowers/specs/2026-11-10-copilot-usage-design.md`. Match the two copy decisions
  in Global Constraints (the balloon title and the invariant `d MMM` date).
- Modify: `Directory.Build.props` (`<Version>0.7.0</Version>`)

- [ ] **Step 1: Edit the docs.** Use plain words in the existing voice, and keep CLAUDE.md short.
- [ ] **Step 2: Run `npm run check`.** Expected: the site checks pass.
- [ ] **Step 3: Run `dotnet test`.** Expected: everything passes.
- [ ] **Step 4: Commit** `Docs: what Sobreiro reads and sends for Copilot; 0.7.0`.
