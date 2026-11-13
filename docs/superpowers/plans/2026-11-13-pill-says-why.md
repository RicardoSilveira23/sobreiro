# Say on the pill why the numbers stopped: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each way the usage numbers can stop gets its own tested state in Core, two words on the pill where the figures were, and a flyout sentence that says what to do.

**Architecture:** The two state enums split `Failing` into `Offline` and `BadAnswer`; the polls already know which it was. One new Core type, `PillWords`, holds the rule (state, age of the reading) and the words. `PillModel` carries the words per block and `PillView` draws them instead of the figures.

**Tech Stack:** C# on .NET 10, xUnit, WPF for the pill.

**Spec:** `docs/superpowers/specs/2026-11-13-pill-says-why-design.md`

## Global Constraints

- Logic in `src/Sobreiro.Core` with an xUnit test in `tests/Sobreiro.Tests`, written first and seen failing.
- No new network call. Nothing here reads, stores, logs or prints a token; log lines name outcomes only.
- Claude's block shows words only when Live usage is on, the state is one of Expired, NoSignIn, Refused, Offline, BadAnswer, RateLimited, and the newest session or weekly reading is over 15 minutes old or missing.
- Copilot's block shows words only when it has a credits reading over 24 hours old and its state is one of NoSignIn, Refused, Offline, BadAnswer, RateLimited.
- The tray icon and the tooltips do not change. With Live usage off (`LiveState.Off`) nothing changes.
- Sentences and pill words are exact copy: use the strings in Tasks 1 and 2 letter for letter.
- Never name or link other people's projects in code comments, docs or commit messages.
- Commit messages end with `Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>`.
- Run tests with `dotnet test -c Release` from the repo root; all must pass before each commit.

## Review Focus

1. Live usage fails while the status line still delivers fresh numbers: the pill keeps its figures (Task 2, `Fresh_numbers_keep_the_figures`).
2. A reading stamped in the future (clock change): counts as fresh, figures stay (Task 2, `A_reading_from_the_future_is_fresh`).
3. Only the weekly reading is fresh and the session one is missing: figures stay, the missing one reads a dash (Task 2, `The_newest_of_the_two_readings_decides`).
4. Copilot with no usable sign-in and a two-hour-old figure from the CLI's cache: the figure stays (Task 2, `A_copilot_figure_from_today_stays`).
5. The longest words (`sign-in` / `refused`, `asked` / `to wait`) drawn in both themes, alone and beside a Copilot block: nothing clipped (Task 3, the snapshot sheet).

---

### Task 1: The states and their sentences

**Files:**
- Modify: `src/Sobreiro.Core/Live/LiveState.cs`, `src/Sobreiro.Core/Live/LivePoll.cs:47-52`
- Modify: `src/Sobreiro.Core/Copilot/CopilotState.cs`, `src/Sobreiro.Core/Copilot/CopilotPoll.cs:59-63`
- Modify: `src/Sobreiro/LiveUsage.cs:104`, `src/Sobreiro/CopilotUsage.cs:121-122` (a poll that threw is `BadAnswer`)
- Modify: `site/manual/index.html` where it quotes the old sentences
- Test: `tests/Sobreiro.Tests/LiveStateTests.cs`, `LivePollTests.cs`, `CopilotStateTests.cs`, `CopilotPollTests.cs`, `FlyoutModelTests.cs`

**Interfaces:**
- Produces: `enum LiveState { Off, Ok, Expired, NoSignIn, Refused, RateLimited, Offline, BadAnswer }` and `enum CopilotState { Ok, NoSignIn, Refused, RateLimited, Offline, BadAnswer }`. `Failing` no longer exists in either.

- [ ] **Step 1: Change the tests first.** In `LiveStateTests.Each_problem_has_a_sentence` the rows become:

```csharp
[InlineData(LiveState.Expired, "Claude Code's sign-in has expired. Use Claude Code in a terminal and it renews it.")]
[InlineData(LiveState.NoSignIn, "Claude Code isn't signed in on this PC. Run claude in a terminal and sign in.")]
[InlineData(LiveState.Refused, "Anthropic didn't accept Claude Code's sign-in. Use Claude Code in a terminal so it gets a new one.")]
[InlineData(LiveState.RateLimited, "Anthropic asked Sobreiro to wait. Nothing to do: it asks again by itself.")]
[InlineData(LiveState.Offline, "Sobreiro can't reach Anthropic. Check the connection; it tries again every 2 minutes.")]
[InlineData(LiveState.BadAnswer, "Anthropic's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.")]
```

  In `CopilotStateTests.Each_problem_has_its_sentence`:

```csharp
[InlineData(CopilotState.NoSignIn, "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh.")]
[InlineData(CopilotState.Refused, "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh.")]
[InlineData(CopilotState.RateLimited, "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself.")]
[InlineData(CopilotState.Offline, "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes.")]
[InlineData(CopilotState.BadAnswer, "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.")]
```

  In `LivePollTests`: `Other_replies_are_failing...` (500 and 302) now expects `LiveState.BadAnswer`; `An_unreachable_host_is_failing` expects `LiveState.Offline`; add a test that a 200 whose body is `{"unexpected":true}` gives `LiveState.BadAnswer` with outcome `"Failed"`, and one that a request timing out gives `LiveState.Offline`. Rename the tests to say what they now assert. In `CopilotPollTests`: the failing case at line 184 splits in two, a 500 giving `CopilotState.BadAnswer` with log `copilot BadAnswer env=failed credential-manager=skipped gh=skipped`, and a thrown `HttpRequestException` giving `CopilotState.Offline` with log `copilot Offline env=unreachable credential-manager=skipped gh=skipped`. Update every other test that names `Failing` or quotes an old sentence (`FlyoutModelTests` lines 113-123, `CopilotStateTests` line 31-34).

- [ ] **Step 2: Run `dotnet test -c Release`.** Expected: does not compile, `LiveState` has no `Offline`.
- [ ] **Step 3: Change the two enums, the two `...StateText` switches, and the two polls' mappings** (`LiveOutcome.Unreachable` and `CopilotOutcome.Unreachable` give `Offline`; every other failure gives `BadAnswer`; `CopilotPoll` writes `unreachable` for the sign-in that could not be reached). Change the two tray lines that build a result for a poll that threw.
- [ ] **Step 4: Run `dotnet test -c Release`.** Expected: all pass. Then `dotnet build -c Release`: no warnings about unhandled enum values.
- [ ] **Step 5: Update `site/manual/index.html`** wherever it quotes one of the old sentences or lists the states, with the new sentences. Run `npm run check` if `package.json` has that script; otherwise skip.
- [ ] **Step 6: Commit** as "Tell a lost network from an answer Sobreiro can't read".

### Task 2: The rule and the words

**Files:**
- Create: `src/Sobreiro.Core/Display/PillWords.cs`
- Modify: `src/Sobreiro.Core/Display/PillModel.cs`
- Test: `tests/Sobreiro.Tests/PillWordsTests.cs` (new), `tests/Sobreiro.Tests/PillModelTests.cs`

**Interfaces:**
- Consumes: the two enums from Task 1.
- Produces:

```csharp
namespace Sobreiro.Core.Display;

/// <summary>The two words a pill block shows in place of its figures once the numbers have stopped.</summary>
public sealed record PillWords(string Top, string Bottom)
{
    public static readonly TimeSpan ClaudeAfter = TimeSpan.FromMinutes(15);
    public static readonly TimeSpan CopilotAfter = TimeSpan.FromHours(24);
    public static PillWords? ForClaude(LiveState state, IReadOnlyList<Limit> limits, DateTimeOffset now);
    public static PillWords? ForCopilot(CopilotState state, Limit? credits, DateTimeOffset now);
}
```

  `PillModel` gains `PillWords? ClaudeWords = null` as its last parameter and `PillCopilot` gains `PillWords? Words = null`. A new overload `PillModel.From(IReadOnlyList<Limit> limits, TimeZoneInfo zone, bool showClaude, LiveState live, CopilotState copilot, DateTimeOffset now)` fills both; the existing three-argument `From` stays and calls it with `LiveState.Off`, `CopilotState.Ok`.

- [ ] **Step 1: Write `PillWordsTests`.** `Now` is `2026-11-13T12:00:00Z`; a helper builds a `Limit` of a kind observed a given time ago (`new("Claude Code", kind, kind.ToString(), 40, null, Now - age, false)`). The words, as a theory over `ForClaude(state, [], Now)`:

| State | Top | Bottom |
|---|---|---|
| Expired | `sign-in` | `expired` |
| NoSignIn | `no` | `sign-in` |
| Refused | `sign-in` | `refused` |
| Offline | `no` | `network` |
| BadAnswer | `no` | `answer` |
| RateLimited | `asked` | `to wait` |

  Further tests, each one assertion or two:
  - `Off_and_ok_never_have_words`: null for `Off` and `Ok`, with no limits and with limits 3 days old.
  - `Fresh_numbers_keep_the_figures`: `Expired` with session and weekly 15 minutes old exactly gives null; 15 minutes and 1 second old gives `sign-in` / `expired`.
  - `The_newest_of_the_two_readings_decides`: `Offline`, session 3 hours old, weekly 1 minute old, gives null; and with only a weekly reading 1 minute old, null.
  - `A_spend_reading_does_not_count`: `Offline`, only a `LimitKind.Spend` reading 1 minute old, gives `no` / `network`.
  - `A_reading_from_the_future_is_fresh`: `Offline`, session observed 10 minutes after `Now`, gives null.
  - Copilot: the same five words for its five states with a credits reading 25 hours old; `Ok` gives null; `A_copilot_figure_from_today_stays`: `NoSignIn` with a reading 2 hours old gives null, and exactly 24 hours old gives null; `No_reading_no_words`: null credits gives null for every state.

- [ ] **Step 2: Add to `PillModelTests`:**
  - `Stopped_claude_block_carries_the_words_and_empty_rings`: `From([session 20 min old, weekly 20 min old], Utc, true, LiveState.Expired, CopilotState.Ok, Now)` has `ClaudeWords == new PillWords("sign-in", "expired")` and `Rings == RingsModel.From([])`.
  - `Fresh_claude_block_has_no_words`: the same with readings 1 minute old has `ClaudeWords == null` and its usual rings.
  - `Stopped_copilot_block_carries_the_words_and_an_empty_bar`: credits 25 hours old with `CopilotState.Offline` has `Copilot.Words == new PillWords("no", "network")` and `Copilot.Fraction == 0`.
  - `The_three_argument_From_never_has_words`: with readings 3 days old, both are null.
- [ ] **Step 3: Run `dotnet test -c Release`.** Expected: does not compile, `PillWords` does not exist.
- [ ] **Step 4: Write `PillWords` and extend `PillModel`.** The age test is `now - newest > threshold`, where newest is the latest `ObservedAt` among the `Session` and `Weekly` limits; no such limit counts as stopped.
- [ ] **Step 5: Run `dotnet test -c Release`.** Expected: all pass.
- [ ] **Step 6: Commit** as "Work out the pill's words for numbers that stopped".

### Task 3: Draw the words and wire the tray

**Files:**
- Modify: `src/Sobreiro/Pill/PillView.cs`
- Modify: `src/Sobreiro/TrayController.cs:332`
- Modify: `src/Sobreiro/Snapshot.cs` (`PillStrip`, near line 306)

**Interfaces:**
- Consumes: `PillModel.ClaudeWords`, `PillCopilot.Words`, and the six-argument `PillModel.From` from Task 2.

- [ ] **Step 1: `PillView.Build`.** When `model.ClaudeWords` is set, the Claude block's two lines are the two words, each a right-aligned `TextBlock` in Inter at `LabelSize`, in the ink colour; the rings are drawn from the model as before (they are empty). When `copilot.Words` is set, the Copilot block's two lines are its two words, drawn the same way, and its bar is drawn from the model as before. Nothing else in the layout changes.
- [ ] **Step 2: `TrayController.UpdatePill`** calls the six-argument `PillModel.From` with `_live.State`, `_copilot.State` and `now`.
- [ ] **Step 3: `Snapshot.PillStrip`** gains, after the four pills it draws today, one pill per Claude state that has words (six), built with readings 3 days old, and one pill with a stopped Claude block beside a stopped Copilot block (`sign-in` / `refused` and `no` / `network`).
- [ ] **Step 4: Build and render.** `dotnet build -c Release`, then run the built `Sobreiro.exe --snapshot <scratch>\pills.png` with `SOBREIRO_DATA_DIR` set to a scratch folder. Expected: exit 0 and a sheet whose two pill strips show the seven new pills with no word clipped. Look at the sheet.
- [ ] **Step 5: Run `dotnet test -c Release`.** Expected: all pass.
- [ ] **Step 6: Commit** as "Say on the pill why the numbers stopped".
