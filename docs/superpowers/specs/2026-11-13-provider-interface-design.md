# A provider interface in Core

Issue 15, part of 1.0. Written 2026-11-13.

## Why

The plan said to build no provider interface until a second provider existed. Copilot is the
second. Today the two share a shape by convention only: each has a static poll in Core, its own
state enum, and its own poller class in the tray. The two tray pollers repeat the same loop
(when a poll is due, one poll at a time, what a stopped or thrown poll counts as), and that loop
has no test because it sits beside the UI.

## Goals

- One interface in `Sobreiro.Core` says what a provider is: how it polls, what limits it gives,
  how it reports that it has stopped.
- The loop both tray pollers repeat lives once in Core, with xUnit tests.
- The tray app starts, ticks and reads both providers the same way.

## Not goals

- No change to what the pill, the flyout, the tray icons, the tooltips, the alerts or
  `diagnostics.log` show.
- No new network call, and no change to how a token is read. A token is still read per poll,
  sent to its one address, and never renewed, stored, cached or logged.
- The pill and the flyout keep their provider-specific layout (rings for Claude, a bar for
  Copilot). `PillModel.From` and `FlyoutModel.From` keep their parameters.
- `LiveSchedule` keeps its name and its place, although both providers use it.
- No third provider, and nothing built for one beyond the interface.

## What would make it a failure

- A change anyone could see on the taskbar or in the log.
- A poll that runs twice at once, or a stopped poll's reading being kept.
- An existing test whose assertion had to change.

## Approaches

- **A, chosen by Ricardo: the interface and the loop both in Core.** `IProvider` plus a
  `ProviderPoller` that holds the loop; the tray keeps one thin host for both providers. The
  larger diff, and the loop's threading has to stay exactly as it is. Chosen because the project
  rule puts logic in Core with a test, and the loop is the part with none today.
- **B: the interface only.** The two tray pollers stay and call their provider through
  `IProvider`. Smaller and safer, but the repeated loop stays in the tray, untested, and a third
  provider would need a third poller.
- **An interface at the tray's edge only** (state, misses, start, tick) was dropped: the issue
  asks for it in Core, and it would not cover how a provider polls.

## The design

New folder and namespace `Sobreiro.Core.Polling` (`Providers` is already a class in `Limits`):

```csharp
public enum ProviderState { Off, Ok, Expired, NoSignIn, Refused, RateLimited, Offline, BadAnswer }

/// One poll's answer. Keep takes the reading in, on the caller's thread; a dropped poll's is never called.
public sealed record ProviderPoll(
    LiveSchedule Schedule, ProviderState State, string Log, Action<DateTimeOffset>? Keep = null);

public interface IProvider
{
    string Name { get; }                                   // the name its limits carry
    Task<ProviderPoll> PollAsync(LiveSchedule schedule, DateTimeOffset now, CancellationToken ct);
    IReadOnlyList<Limit> Limits(DateTimeOffset now);
}
```

- **How it reports that it has stopped:** the `ProviderState` on each poll. `ProviderState`
  replaces `LiveState` and `CopilotState`, which had the same members but for `Off` and
  `Expired`. `LiveStateText`, `CopilotStateText`, `PillModel.From` and `FlyoutModel.From` take
  the new enum and are otherwise unchanged.
- **`ClaudeProvider`** (in `Live/`) wraps `LivePoll.RunAsync`, unchanged. Its `Keep` merges the
  reading into `usage.json` as the tray does today (authoritative, one retry when the bridge
  holds the file, a failure skips the reading). `Limits` reads `usage.json` and keeps the last
  snapshot when a read loses the race with the bridge.
- **`CopilotProvider`** (in `Copilot/`) wraps `CopilotPoll.RunAsync`, unchanged, then reads the
  Copilot CLI's cache. Its `Keep` takes the live and cached readings and writes its line to
  `diagnostics.log` when the line changes, exactly as now. `Shown` stays public for the flyout's
  note. Showing the cache before the first poll answers stays: `ReadCache()` off the UI thread,
  `TakeEarly(cache)` on it.
- **`ProviderPoller`** holds what the two tray pollers repeat: the schedule, `State` (Ok until
  the first poll answers), `Misses`, `Start`, `Stop`, `Tick`, `FlyoutOpened`, one poll at a time,
  a poll stopped while it ran is dropped and the next one starts, a poll that threw counts as
  `BadAnswer` on the two-minute retry. It knows nothing of threads: it hands each poll to the
  host as a `PollRun` (`RunAsync()` to run off the UI thread, `Done(task, now)` to call back on
  it). Tests drive it with a fake provider and complete the runs by hand.
- **`PillWords`** keeps `ForClaude` and `ForCopilot`; the word table they both carried becomes
  one private method. Their tests stand as written.
- **In the tray**, `LiveUsage.cs` and `CopilotUsage.cs` become one `ProviderHost`: the
  `HttpClient`, the hop off and back onto the UI thread, the redraw. `TrayController` builds the
  two providers (the sign-in sources that need Windows stay in the tray) and asks each host for
  `State`, `Misses` and `Limits`.

## Tests

- New: `ProviderPollerTests` (due and not due, on open, one at a time, stop while running,
  thrown poll, state and misses), `ClaudeProviderTests` (a poll's reading reaches `usage.json`
  only through `Keep`; `Limits` keeps the last snapshot), `CopilotProviderTests` (live against
  cache, the log line only when it changes, the early cache).
- Existing tests change only where they name the enum: `LiveState.` and `CopilotState.` become
  `ProviderState.` in 7 files. No assertion changes.

## Left to the builder

- Member names inside the new classes, and whether `PollRun` is a class or a record.
- Debug-only output (`Debug.WriteLine`) may be worded differently; `diagnostics.log` may not.
