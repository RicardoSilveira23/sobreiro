# Provider Interface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put Claude Code's usage and Copilot's credits behind one `IProvider` in `Sobreiro.Core`, with the polling loop both tray pollers repeat moved into Core and tested.

**Architecture:** A new `Sobreiro.Core.Polling` namespace holds `ProviderState`, `ProviderPoll`, `IProvider`, `ProviderPoller` and `PollRun`. `ClaudeProvider` and `CopilotProvider` wrap today's `LivePoll` and `CopilotPoll`, which do not change. In the tray, `LiveUsage.cs` and `CopilotUsage.cs` give way to one `ProviderHost` that only hops between threads.

**Tech Stack:** C#, .NET 10, xUnit. No new package.

**Spec:** [docs/superpowers/specs/2026-11-13-provider-interface-design.md](../specs/2026-11-13-provider-interface-design.md)

## Global Constraints

- A refactor: nothing the pill, the flyout, the tray icons, the tooltips, the alerts or `diagnostics.log` show may change.
- No new network call. A token is read per poll, sent to its one address, and never renewed, stored, cached, logged or printed. No exception's text goes to any log: its type name only.
- `LivePoll`, `CopilotPoll`, `LiveSchedule`, `LiveUsageClient`, `CopilotUsageClient`, `UsageFile` and `UsageMerge` keep their code, apart from the enum's name.
- Existing tests change only where they name the enum (`LiveState.` or `CopilotState.` becomes `ProviderState.`, plus a `using Sobreiro.Core.Polling;`). No assertion changes, no test is removed.
- Logic goes in `Sobreiro.Core` with an xUnit test in `tests/Sobreiro.Tests`. Test classes follow the existing ones: a fixed `Now`, temp folders named `sobreiro-tests-<guid>` removed in `Dispose`.
- Never name or link other people's projects in code comments, docs or commit messages.
- Do not start, stop or signal any running `Sobreiro.exe`, and never touch `%LOCALAPPDATA%\Sobreiro`. Nothing in this plan needs the tray to run.
- Every commit message ends with `Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>`.
- Run from the repository root: `dotnet test -c Release`. It passes with 884 tests before Task 1.

## Review Focus

1. A poll that comes back after `Dispose`: nothing is kept, nothing redraws, nothing throws. (Task 2)
2. `Stop` then `Start` while a poll runs: the old answer is dropped and the next poll starts as soon as it comes back, never two at once. (Task 2)
3. `usage.json` held by the bridge when a reading arrives: the reading is skipped, the file is left as it was, nothing throws. (Task 3)
4. The Copilot CLI's cache file gone or unreadable on a later poll: the cached reading from before is kept. (Task 4)
5. A provider's log line never holds a token, also when the poll throws. (Tasks 3 and 4)

---

### Task 1: One state enum, one word table

**Files:**
- Create: `src/Sobreiro.Core/Polling/ProviderState.cs`
- Modify: `src/Sobreiro.Core/Live/LiveState.cs` (enum removed, `LiveStateText` stays), `src/Sobreiro.Core/Copilot/CopilotState.cs` (enum removed, `CopilotShown` and `CopilotStateText` stay), `src/Sobreiro.Core/Display/PillWords.cs`, and every file under `src/` and `tests/` that names `LiveState` or `CopilotState` (find them with a search; 12 under `src/`, 7 under `tests/`)

**Interfaces:**
- Produces: `namespace Sobreiro.Core.Polling; public enum ProviderState { Off, Ok, Expired, NoSignIn, Refused, RateLimited, Offline, BadAnswer }` with the doc comment "Where a provider's poll stands: Off and Ok, or the reason its numbers stopped."

- [ ] **Step 1: Add `ProviderState`, remove both old enums, rename every use.** `LiveState.X` and `CopilotState.X` become `ProviderState.X`; parameter and property types follow. In `src/Sobreiro/LiveUsage.cs` and `src/Sobreiro/CopilotUsage.cs` only the type name changes (Task 5 removes those files). In tests, only the type name and the `using`.

- [ ] **Step 2: Fold the word table in `PillWords`.** Add `private static PillWords? Of(ProviderState state)` with the six pairs `ForClaude` has today (`Expired` "sign-in"/"expired", `NoSignIn` "no"/"sign-in", `Refused` "sign-in"/"refused", `Offline` "no"/"network", `BadAnswer` "no"/"answer", `RateLimited` "asked"/"to wait", else null). `ForClaude` and `ForCopilot` keep their signatures and their own age rules and call `Of`.

- [ ] **Step 3: Verify.**

Run: `dotnet test -c Release`
Expected: 884 passed, 0 failed, no new warning.

Run: `git diff --stat -- tests` then `git diff -- tests | grep '^[-+]' | grep -v 'ProviderState\|LiveState\|CopilotState\|Sobreiro.Core.Polling\|^+++\|^---'`
Expected: the second command prints nothing.

- [ ] **Step 4: Commit** as "One state enum for both providers".

---

### Task 2: `IProvider`, `ProviderPoll`, `ProviderPoller`

**Files:**
- Create: `src/Sobreiro.Core/Polling/IProvider.cs` (the interface and `ProviderPoll`), `src/Sobreiro.Core/Polling/ProviderPoller.cs` (`ProviderPoller` and `PollRun`)
- Test: `tests/Sobreiro.Tests/ProviderPollerTests.cs`

**Interfaces:**
- Consumes: `ProviderState` (Task 1); `LiveSchedule`, `LiveResult`, `LiveOutcome` from `Sobreiro.Core.Live`; `Limit` from `Sobreiro.Core.Limits`.
- Produces:

```csharp
namespace Sobreiro.Core.Polling;

/// One poll's answer. Log names outcomes only, never a token. Keep takes the reading in, on the
/// caller's thread, with the time it is called; a dropped poll's Keep is never called.
public sealed record ProviderPoll(
    LiveSchedule Schedule, ProviderState State, string Log, Action<DateTimeOffset>? Keep = null);

public interface IProvider
{
    string Name { get; }  // the name its limits carry
    /// One poll, safe to run off the caller's thread. Throws only an OperationCanceledException for ct.
    Task<ProviderPoll> PollAsync(LiveSchedule schedule, DateTimeOffset now, CancellationToken ct);
    /// The limits to show now, from what the kept polls brought. Called on the caller's thread.
    IReadOnlyList<Limit> Limits(DateTimeOffset now);
}

/// A poll the host runs off its thread (RunAsync), then hands back on it (Done).
public sealed class PollRun
{
    public Task<ProviderPoll> RunAsync();
    public void Done(Task<ProviderPoll> task, DateTimeOffset now);
}

/// Not thread-safe: every member but PollRun.RunAsync is called on one thread.
public sealed class ProviderPoller : IDisposable
{
    public ProviderPoller(IProvider provider, Action<PollRun> run, Action changed, Action<string>? trace = null);
    public IProvider Provider { get; }
    public ProviderState State { get; }   // Ok until the first poll answers
    public int Misses { get; }            // the schedule's; 0 while off
    public void Start(DateTimeOffset now);
    public void Stop();
    public void Tick(DateTimeOffset now);
    public void FlyoutOpened(DateTimeOffset now);
    public void Dispose();
}
```

The behaviour is the one `src/Sobreiro/LiveUsage.cs` has today (read it: `Start`, `Stop`, `Tick`, `FlyoutOpened`, `Poll`, `Done`, `Dispose`), minus the threads and the storing:

- `Start`: nothing when disposed or already on; else `LiveSchedule.Start(now)` and `Tick(now)`.
- `Tick` polls when the schedule is due; `FlyoutOpened` when it is due or `FetchOnOpen(now)`. A poll starts only when not disposed and none is running: it creates a `CancellationTokenSource`, and passes a `PollRun` to `run`. `RunAsync` calls `provider.PollAsync(schedule, now, token)` with the schedule and time the poll started with.
- `Stop`: schedule off, `State = Off`, the running poll's token cancelled.
- `Done(task, now)`: note whether the token was cancelled, dispose the source, mark no poll running. Disposed: return. Cancelled or off: `Tick(now)` and return (the answer is dropped). Otherwise take `task.Result` when it completed successfully, else `new ProviderPoll(schedule.AfterFetch(new LiveResult(LiveOutcome.Failed, null, null), started), ProviderState.BadAnswer, $"{provider.Name} poll threw {type name of the base exception, or "nothing" when there is none}")` using the schedule and time the poll started with. Then, in this order: set the schedule, set `State`, call `Keep(now)` when there is one, `trace(poll.Log)`, `changed()`.
- `Dispose`: mark disposed, then `Stop`. Idempotent.

- [ ] **Step 1: Write the failing tests** in `ProviderPollerTests`. A private `FakeProvider : IProvider` with `Name => "Fake"`, a `Func<LiveSchedule, DateTimeOffset, CancellationToken, Task<ProviderPoll>> Answer`, a list of the tokens it was given, and `Limits` returning `[]`. The poller is built with `run: runs.Add`, `changed: () => changed++`, `trace: traced.Add`. `Now` is `2026-11-13 12:00 UTC`. A helper `Ok(at)` builds `new ProviderPoll(LiveSchedule.Start(at).AfterFetch(new LiveResult(LiveOutcome.Ok, null, null), at), ProviderState.Ok, "fake Ok", keep)`.

| Test | Asserts |
|---|---|
| `Before_start_it_is_ok_with_no_misses` | `State == Ok`, `Misses == 0`, no run after `Tick(Now)` and `FlyoutOpened(Now)` |
| `Start_polls_at_once_and_only_once` | one run after `Start(Now)`; still one after a second `Start(Now)` |
| `The_poll_gets_the_schedule_and_time_it_started_with` | after `runs[0].RunAsync()`, the fake saw `now == Now` and a schedule with `NextDue == Now` |
| `One_poll_at_a_time` | with a run outstanding, `Tick(Now.AddMinutes(10))` and `FlyoutOpened(Now.AddMinutes(10))` add none |
| `An_answer_sets_the_state_keeps_the_reading_and_redraws` | `Done(Task.FromResult(poll with State RateLimited, Keep recording its argument), Now.AddSeconds(3))`: `State == RateLimited`, Keep called once with `Now.AddSeconds(3)`, `changed == 1`, `traced` is `["fake Ok"]`; Keep ran before `changed` |
| `Misses_follow_the_schedule` | an answer whose schedule is `LiveSchedule.Start(Now).AfterNoCredentials(Now)` gives `Misses == 1`; a following `Ok(…)` answer gives 0 |
| `Tick_waits_for_the_schedule` | after `Ok(Now)`: no run at `Now.AddMinutes(4).AddSeconds(59)`, one at `Now.AddMinutes(5)` |
| `Opening_the_flyout_asks_early` | after an answer with `AfterNoCredentials(Now)` (due in a minute, no wait): `Tick(Now.AddSeconds(10))` adds no run, `FlyoutOpened(Now.AddSeconds(10))` adds one |
| `Opening_the_flyout_respects_a_wait` | after an answer with `AfterFetch(Failed)` (a two-minute wait): `FlyoutOpened(Now.AddSeconds(10))` adds no run |
| `A_poll_that_threw_counts_as_a_bad_answer` | `Done(Task.FromException<ProviderPoll>(new InvalidOperationException("secret")), Now.AddSeconds(1))`: `State == BadAnswer`, `Misses == 1`, `changed == 1`, `traced` is `["Fake poll threw InvalidOperationException"]` (no "secret"); no run at `Now.AddMinutes(1).AddSeconds(59)`, one at `Now.AddMinutes(2)` (two minutes from when the poll started) |
| `A_cancelled_task_without_stop_counts_as_a_bad_answer` | `Done(Task.FromCanceled<ProviderPoll>(new CancellationToken(true)), …)`: `State == BadAnswer` |
| `Stop_cancels_the_poll_and_turns_it_off` | after `RunAsync()` then `Stop()`: the token the fake got is cancelled, `State == Off`, `Misses == 0` |
| `A_poll_stopped_while_it_ran_is_dropped` | `Stop()`, then `Done(Task.FromResult(Ok with Keep))`: Keep not called, `changed == 0`, `State == Off`, no new run |
| `Stop_then_start_while_a_poll_runs_waits_for_it` | `Stop()`, `Start(Now.AddSeconds(1))`: still one run; `runs[0].Done(Ok with Keep, Now.AddSeconds(2))`: Keep not called, a second run exists; its answer is then accepted (`State` follows it) |
| `After_dispose_nothing_happens` | `Dispose()`, then `runs[0].Done(Ok with Keep, …)`: Keep not called, `changed == 0`, no throw; `Start`, `Tick`, `FlyoutOpened` add no run; a second `Dispose()` does not throw |

- [ ] **Step 2: Run them and see them fail.** Run: `dotnet test -c Release --filter ProviderPollerTests`. Expected: a build error naming `ProviderPoller`.

- [ ] **Step 3: Write `IProvider.cs` and `ProviderPoller.cs`** to the signatures and behaviour above. `PollRun` has an internal constructor; the poller is its only maker.

- [ ] **Step 4: Run all tests.** Run: `dotnet test -c Release`. Expected: 884 plus the new ones passed, 0 failed.

- [ ] **Step 5: Commit** as "Add the provider interface and its poller to Core".

---

### Task 3: `ClaudeProvider`

**Files:**
- Create: `src/Sobreiro.Core/Live/ClaudeProvider.cs`
- Test: `tests/Sobreiro.Tests/ClaudeProviderTests.cs`

**Interfaces:**
- Consumes: `IProvider`, `ProviderPoll` (Task 2); `LivePoll.RunAsync(string credentialsPath, LiveUsageClient client, LiveSchedule schedule, DateTimeOffset now, CancellationToken ct)` returning `LivePollResult(Schedule, State, Limits, Outcome)`; `UsageFile.Read`, `UsageFile.TryReadForMerge`, `UsageFile.Write`, `UsageMerge.Merge(stored, incoming, now, authoritative: true)`; `LimitList.From(UsageSnapshot?, DateTimeOffset)`; `Providers.Claude`.
- Produces:

```csharp
namespace Sobreiro.Core.Live;

public sealed class ClaudeProvider : IProvider
{
    public ClaudeProvider(LiveUsageClient client, string credentialsPath, string usagePath, Action<string>? trace = null);
    internal ClaudeProvider(
        Func<LiveSchedule, DateTimeOffset, CancellationToken, Task<LivePollResult>> poll,
        string usagePath, Action<string>? trace = null);
    public string Name { get; }  // Providers.Claude
    public Task<ProviderPoll> PollAsync(LiveSchedule schedule, DateTimeOffset now, CancellationToken ct);
    public IReadOnlyList<Limit> Limits(DateTimeOffset now);
}
```

- `PollAsync` awaits the poll and returns `new ProviderPoll(r.Schedule, r.State, "Live usage " + r.Outcome, keep)`, where `keep` is null without `r.Limits` and otherwise stores them.
- Storing is `Store` from `src/Sobreiro/LiveUsage.cs`, moved as it is: `TryReadForMerge`; when it fails, sleep 50 ms and try once more; still failing, `trace("usage.json was busy, so this Live usage reading was not saved")` and return; else `UsageFile.Write(path, UsageMerge.Merge(stored, limits, now, authoritative: true))` with the `now` that `Keep` was given. Any exception: `trace("saving the Live usage reading failed: " + type name)`, nothing thrown.
- `Limits(now)`: `_last = UsageFile.Read(usagePath) ?? _last; return LimitList.From(_last, now);`. It lets `UsageFile.Read`'s exceptions through, as the tray's caller catches them today.

- [ ] **Step 1: Write the failing tests** in `ClaudeProviderTests` (`IDisposable`, temp folder, `Now` as in Task 2). Build the provider with the internal constructor and a lambda returning a fixed `LivePollResult`. `Reading` is `new RateLimits(FiveHour, null, null)` with a five-hour `LimitWindow` at 27 % resetting at `Now.AddHours(2)` (build the window the way `tests/Sobreiro.Tests/UsageMergeTests.cs` does).

| Test | Asserts |
|---|---|
| `It_carries_claudes_name` | `Name == "Claude Code"` |
| `A_poll_passes_on_the_schedule_the_state_and_the_outcome` | result `(schedule, ProviderState.Ok, Reading, "Ok")` gives the same schedule instance, `State == Ok`, `Log == "Live usage Ok"` |
| `A_poll_without_a_reading_has_nothing_to_keep` | result `(…, ProviderState.Expired, null, "Expired")`: `Keep` is null, `Log == "Live usage Expired"` |
| `The_reading_reaches_usage_json_only_through_keep` | after `PollAsync`, `usage.json` does not exist and `Limits(Now)` is empty; after `Keep(Now)`, `Limits(Now)` has one `Session` limit at 27 % |
| `Keep_merges_over_what_the_file_holds` | the file first holds a snapshot with a weekly limit (written with `UsageFile.Write`); after `Keep`, `Limits` has both the weekly and the session limit |
| `A_reading_replaces_a_higher_stored_one_in_the_same_period` | the file holds session 40 % for the same reset; after `Keep`, session is 27 % (authoritative) |
| `A_held_usage_json_skips_the_reading` | open `usage.json` with `FileShare.None`, call `Keep(Now)`: no throw, trace has "usage.json was busy, so this Live usage reading was not saved"; after closing, the file's bytes are what they were |
| `Limits_keep_the_last_snapshot_when_a_read_fails` | after a good `Limits`, hold the file open with `FileShare.None` (`UsageFile.Read` then returns null): `Limits(Now)` still has the session limit |
| `The_public_constructor_reads_the_sign_in_and_asks` | with a credentials file and a counting `HttpMessageHandler` (as in `LivePollTests`), `PollAsync` then `Keep` gives the session limit, the handler saw one request, and `Log` does not contain the token |
| `Cancellation_passes_through` | a poll lambda that throws `OperationCanceledException` for a cancelled token: `PollAsync` throws it |

- [ ] **Step 2: Run them and see them fail.** Run: `dotnet test -c Release --filter ClaudeProviderTests`. Expected: a build error naming `ClaudeProvider`.

- [ ] **Step 3: Write `ClaudeProvider`.** The public constructor forwards to the internal one with `(s, at, ct) => LivePoll.RunAsync(credentialsPath, client, s, at, ct)`.

- [ ] **Step 4: Run all tests.** Run: `dotnet test -c Release`. Expected: all passed.

- [ ] **Step 5: Commit** as "Put Claude Code's usage behind the provider interface".

---

### Task 4: `CopilotProvider`

**Files:**
- Create: `src/Sobreiro.Core/Copilot/CopilotProvider.cs`
- Test: `tests/Sobreiro.Tests/CopilotProviderTests.cs`

**Interfaces:**
- Consumes: `IProvider`, `ProviderPoll` (Task 2); `CopilotPoll.RunAsync(IReadOnlyList<CopilotSignIn> signIns, CopilotUsageClient client, LiveSchedule schedule, DateTimeOffset now, CancellationToken ct)` returning `CopilotPollResult(Schedule, State, Reading, Log)`; `CopilotCache.Read(string path)`; `CopilotShown.Pick(live, cache)`; `LimitList.FromCopilot(CopilotReading?, DateTimeOffset)`; `Providers.Copilot`.
- Produces:

```csharp
namespace Sobreiro.Core.Copilot;

public sealed class CopilotProvider : IProvider
{
    public CopilotProvider(
        CopilotUsageClient client, IReadOnlyList<CopilotSignIn> signIns, string cachePath, Action<string> log);
    internal CopilotProvider(
        Func<LiveSchedule, DateTimeOffset, CancellationToken, Task<CopilotPollResult>> poll,
        Func<CopilotReading?> readCache, Action<string> log);
    public string Name { get; }               // Providers.Copilot
    public CopilotReading? Shown { get; }     // CopilotShown.Pick(live, cache)
    public CopilotReading? ReadCache();       // the cache file, safe off the caller's thread
    public void TakeEarly(CopilotReading cache);  // keeps it only when no cached reading is held yet
    public Task<ProviderPoll> PollAsync(LiveSchedule schedule, DateTimeOffset now, CancellationToken ct);
    public IReadOnlyList<Limit> Limits(DateTimeOffset now);  // LimitList.FromCopilot(Shown, now)
}
```

- `PollAsync`: await the poll, then read the cache. An exception from either while `ct` is not cancelled gives the result `new CopilotPollResult(schedule.AfterFetch(new LiveResult(LiveOutcome.Failed, null, null), now), ProviderState.BadAnswer, null, "copilot BadAnswer Failed")` with no cache; with `ct` cancelled it passes through. It returns `new ProviderPoll(result.Schedule, result.State, result.Log, keep)`; `keep` is never null.
- `keep`, as `Done` in `src/Sobreiro/CopilotUsage.cs` does today: a reading replaces the live one; a cache that was read replaces the held one, a null keeps it; then the line `result.Log + " cache=" + ("used" when Shown is from the cache, else "none" when no cache is held, else "older")` goes to `log` only when it differs from the last line logged.

- [ ] **Step 1: Write the failing tests** in `CopilotProviderTests`, with the internal constructor, a settable `Func` for the poll result and for the cache, and `logged` as a list. `Live(at)` is `new CopilotReading(15000, 2645, Now.AddDays(18), at, CopilotSource.Live)`; `Cache(at)` the same with `CopilotSource.Cache`. `Poll(state, reading, log)` builds a `CopilotPollResult` on `LiveSchedule.Start(Now)`.

| Test | Asserts |
|---|---|
| `It_carries_copilots_name` | `Name == "GitHub Copilot"` |
| `Nothing_is_shown_before_a_poll` | `Shown` is null, `Limits(Now)` is empty |
| `A_poll_passes_on_the_schedule_the_state_and_the_line` | same schedule instance, `State`, and `Log == "copilot Ok env=ok credential-manager=skipped gh=skipped"` |
| `The_reading_is_shown_only_after_keep` | after `PollAsync` `Shown` is null; after `Keep(Now)` `Shown` is the live reading and `Limits(Now)` has one `Credits` limit |
| `The_later_of_live_and_cache_is_shown` | live at `Now`, cache at `Now.AddMinutes(5)`: `Shown.Source == Cache`; cache at `Now.AddMinutes(-5)`: `Live` |
| `The_line_says_how_the_cache_was_used` | three cases after `Keep`: no cache gives a line ending " cache=none"; an older cache " cache=older"; a newer cache " cache=used" |
| `The_line_is_logged_only_when_it_changes` | two identical polls kept: one line; a third with `NoSignIn`: two lines |
| `A_poll_that_is_not_kept_logs_nothing` | `PollAsync` without `Keep`: `logged` is empty |
| `A_failed_poll_keeps_the_live_reading` | a kept Ok poll, then a kept `Offline` poll without a reading: `Shown` is still the first reading |
| `A_cache_that_cannot_be_read_keeps_the_one_before` | a kept poll with a cache, then one where the cache `Func` returns null: with no live reading, `Shown` is still the cached one |
| `The_early_cache_is_taken_only_when_none_is_held` | `TakeEarly(Cache(Now))` then `Shown.Source == Cache`; after a kept poll brought `Cache(Now.AddMinutes(5))`, `TakeEarly(Cache(Now.AddMinutes(-60)))` leaves the newer one |
| `A_poll_that_throws_counts_as_a_bad_answer` | a poll `Func` throwing `InvalidOperationException("ghp_SECRET")`: `State == BadAnswer`, `Log == "copilot BadAnswer Failed"`, the schedule is due at `Now.AddMinutes(2)` with `Misses == 1`; after `Keep`, `logged` is `["copilot BadAnswer Failed cache=none"]` |
| `A_cache_read_that_throws_counts_as_a_bad_answer` | the cache `Func` throws after a good poll: `State == BadAnswer` and, after `Keep`, `Shown` is null |
| `Cancellation_passes_through` | the poll `Func` throws `OperationCanceledException` for a cancelled token: `PollAsync` throws it |
| `The_public_constructor_asks_github_and_reads_the_cache_file` | with a fake handler and one sign-in (as in `CopilotPollTests`) and a cache path that does not exist: after `Keep`, `Shown.Source == Live`, and neither `Log` nor `logged` contains the token; `ReadCache()` returns null |

- [ ] **Step 2: Run them and see them fail.** Run: `dotnet test -c Release --filter CopilotProviderTests`. Expected: a build error naming `CopilotProvider`.

- [ ] **Step 3: Write `CopilotProvider`.** The public constructor forwards to the internal one with `(s, at, ct) => CopilotPoll.RunAsync(signIns, client, s, at, ct)` and `() => CopilotCache.Read(cachePath)`.

- [ ] **Step 4: Run all tests.** Run: `dotnet test -c Release`. Expected: all passed.

- [ ] **Step 5: Commit** as "Put Copilot's credits behind the provider interface".

---

### Task 5: The tray talks to both through the interface

**Files:**
- Create: `src/Sobreiro/ProviderHost.cs`
- Delete: `src/Sobreiro/LiveUsage.cs`, `src/Sobreiro/CopilotUsage.cs`
- Modify: `src/Sobreiro/TrayController.cs`

**Interfaces:**
- Consumes: `ProviderPoller`, `PollRun`, `IProvider` (Task 2); `ClaudeProvider` (Task 3); `CopilotProvider` (Task 4).
- Produces, in the tray only:

```csharp
namespace Sobreiro;

/// One provider's poller on the UI thread: the HttpClient, the hop off the thread and back, the redraw.
internal sealed class ProviderHost : IDisposable
{
    public ProviderHost(HttpClient http, IProvider provider, Action changed);
    public ProviderState State { get; }
    public int Misses { get; }
    public IReadOnlyList<Limit> Limits(DateTimeOffset now);
    public void Start(DateTimeOffset now);
    public void Tick(DateTimeOffset now);
    public void FlyoutOpened(DateTimeOffset now);
    public void Dispose();            // the poller, then the HttpClient
    public static string UserAgent(); // "Sobreiro/<version>", as both old pollers built it
}
```

- [ ] **Step 1: Write `ProviderHost`.** It captures `Dispatcher.CurrentDispatcher` in its constructor and builds `new ProviderPoller(provider, Run, changed, line => Debug.WriteLine("Sobreiro: " + line))`. `Run(PollRun run)` is the old pollers' hop, unchanged in shape:

```csharp
Task.Run(run.RunAsync).ContinueWith(task =>
{
    _ = task.Exception; // observed here, so a poll that ends after shutdown raises nothing
    _dispatcher.BeginInvoke(() => run.Done(task, DateTimeOffset.Now));
}, TaskScheduler.Default);
```

- [ ] **Step 2: Move `TrayController` onto it.**
  - Fields: `ProviderHost _claude`, `ProviderHost _copilot`, `CopilotProvider _copilotProvider`; `_last` goes (the provider holds it).
  - Build each host with its own `new HttpClient(LiveUsageClient.CreateHandler())`. Claude: `new ClaudeProvider(new LiveUsageClient(http, ProviderHost.UserAgent()), ClaudePaths.CredentialsFile(Environment.GetEnvironmentVariable), _usagePath, line => Debug.WriteLine("Sobreiro: " + line))`. Copilot: `new CopilotProvider(new CopilotUsageClient(http, ProviderHost.UserAgent()), signIns, CopilotCache.DefaultPath(Environment.GetEnvironmentVariable), log ?? (_ => { }))`, where `signIns` is the three-entry list from `CopilotUsage.cs`, moved as it is, in the same order (Env, CredentialManager, Gh).
  - Start: `_claude.Start(now)`, `_copilot.Start(now)`, then the early cache as `CopilotUsage.Start` does it: `Task.Run(_copilotProvider.ReadCache)`, and when it completed with a reading, on the dispatcher and only when not disposed, `_copilotProvider.TakeEarly(cache); Refresh();`. Observe the task's exception as the old code does.
  - `OpenFlyout`, `OnTick` and `Dispose` call both hosts where they called the old pollers, keeping each `Guard` and its name.
  - `Refresh`: inside the existing `try`, `now = DateTimeOffset.Now; claude = _claude.Limits(now);`; in the existing `Guard("copilot limits", …)`, `copilot = _copilot.Limits(now)`.
  - `UpdatePill` reads `_claude.State`, `_claude.Misses`, `_copilot.State`, `_copilot.Misses`; `UpdateFlyout` reads `_copilotProvider.Shown` and `_copilot.State` for the note and `_claude.State` for the footer.

- [ ] **Step 3: Delete `LiveUsage.cs` and `CopilotUsage.cs`,** then check nothing still names them: a search for `LiveUsage\b|CopilotUsage\b` under `src/` finds only `LiveUsageClient`, `CopilotUsageClient`, `LiveUsageMessage` and comments that mean the feature.

- [ ] **Step 4: Build and test.**

Run: `dotnet build -c Release`
Expected: 0 errors, and the same number of warnings as the build at Task 4's commit (note that number before starting this task).

Run: `dotnet test -c Release`
Expected: all passed, the same count as after Task 4.

- [ ] **Step 5: Commit** as "The tray talks to both providers through the interface".

---

### Task 6: Roadmap

**Files:**
- Modify: `docs/roadmap.md`

- [ ] **Step 1: Mark the two 1.0 items done,** in the style the two packaging items use ("Done … on 2026-11-13: …"):
  - "Defined, tested behaviour for each way the data can stop …": done on 2026-11-13; each cause has its own state in Core, and the pill says which in two words once two polls in a row have brought nothing and the last reading is old.
  - "A provider interface in Core …": done on 2026-11-13; both providers sit behind one interface, and the polling loop both used is in Core with tests.
- [ ] **Step 2: Correct the test count.** In "Where it stands", `| Tests | 660, …` takes the number the final `dotnet test -c Release` reports. The 2.0 section's "Core and its 660 tests" takes the same number.
- [ ] **Step 3: Verify.** Run: `npm run check`. Expected: every test passes and the site check reports no problem.
- [ ] **Step 4: Commit** as "Roadmap: the data-stop behaviour and the provider interface are done".
