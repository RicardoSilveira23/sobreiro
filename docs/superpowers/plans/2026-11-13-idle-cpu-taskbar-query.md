# Idle CPU: Taskbar Query Schedule Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The pill host asks UI Automation where the taskbar's parts are only when something may have changed, so an idle tray uses 5 s of processor time or less per hour.

**Architecture:** A small immutable schedule in `Sobreiro.Core/Taskbar` decides whether a pill tick also sends the query. `PillHost` keeps its 2-second tick and feeds the schedule four facts: a question was sent, an answer came, the foreground window switched, the taskbar was attached anew.

**Tech Stack:** C# / .NET 10, xUnit. No new package.

**Spec:** `docs/superpowers/specs/2026-11-13-idle-cpu-taskbar-query-design.md`

## Global Constraints

- The 2-second tick, the placement rules, the event hooks and the pill's look stay as they are.
- The questions sent at once today stay: a move of the tray area, a move of the taskbar while no anchors are known, a display change, a DPI change.
- The slow check is 15 seconds.
- Nothing Sobreiro reads or sends changes; no new network call.
- Logic goes in `Sobreiro.Core` with an xUnit test. No version bump.
- Docs, commit messages and pull request text never name or link other people's projects.
- Do not start, stop or signal any running Sobreiro; `dotnet test` and `dotnet build` only.

## Review Focus

- Answers that keep differing (a taskbar that never settles): the pill keeps asking every tick, which is today's cost, and never throws. Test in Task 1.
- A burst of foreground switches between two ticks: one question at the next tick, not one per switch. Test in Task 1.
- A clock value earlier than the last question (should not happen with a monotonic clock): not due, no exception. Test in Task 1.
- An answer that belongs to an older taskbar or older anchors: `PillHost.OnProbed` must not feed it to the schedule. No unit test; the reviewer of Task 2 checks it.
- Explorer restarting, or the display changing: the schedule starts over, so the next question is not held back 15 seconds. No unit test; the reviewer of Task 2 checks every place `_generation++` appears.

---

### Task 1: `TaskbarQuerySchedule` in Core

**Files:**
- Create: `src/Sobreiro.Core/Taskbar/TaskbarQuerySchedule.cs`
- Test: `tests/Sobreiro.Tests/TaskbarQueryScheduleTests.cs`

**Interfaces:**
- Consumes: `Sobreiro.Core.Taskbar.TaskbarAnchors` (`readonly record struct TaskbarAnchors(int TrayLeft, int ButtonsRight, int BandTop, int BandBottom)`, in `TaskbarLayout.cs`).
- Produces, all in namespace `Sobreiro.Core.Taskbar`:

```csharp
/// <summary>Decides whether a pill tick also asks UI Automation where the taskbar's parts are.</summary>
public readonly record struct TaskbarQuerySchedule
{
    public static readonly TimeSpan SlowCheck = TimeSpan.FromSeconds(15);
    // default(TaskbarQuerySchedule) is the fresh schedule: nothing asked, nothing answered.
    public bool Due(TimeSpan now);
    public TaskbarQuerySchedule Asked(TimeSpan now);
    public TaskbarQuerySchedule Answered(TaskbarAnchors? answer);
    public TaskbarQuerySchedule ForegroundSwitched();
}
```

Rules: `Due` is true when nothing has been asked yet, or a foreground switch came since the last `Asked`, or the last two answers differ, or exactly one answer has come, or `now - lastAsked >= SlowCheck`. A null answer is an answer like any other: two nulls in a row agree. `Asked` records the time and clears the pending switch. Keep the state in private init-only properties, so the record's equality still works and callers cannot build odd states.

- [ ] **Step 1: Write the failing tests**

`now` values are `TimeSpan.FromSeconds(n)`; `A` and `B` are two different `TaskbarAnchors`.

```csharp
public class TaskbarQueryScheduleTests
{
    private static readonly TaskbarAnchors A = new(2200, 900, 1392, 1440);
    private static readonly TaskbarAnchors B = new(2170, 900, 1392, 1440);
    private static TimeSpan S(double seconds) => TimeSpan.FromSeconds(seconds);
    private static TaskbarQuerySchedule Settled(TaskbarAnchors? answer, double askedAt) =>
        default(TaskbarQuerySchedule).Asked(S(askedAt)).Answered(answer).Asked(S(askedAt)).Answered(answer);

    [Fact] public void A_fresh_schedule_is_due() => Assert.True(default(TaskbarQuerySchedule).Due(S(0)));
    [Fact] public void A_question_that_is_out_is_not_asked_again() => Assert.False(default(TaskbarQuerySchedule).Asked(S(0)).Due(S(2)));
    [Fact] public void One_answer_alone_asks_once_more() => Assert.True(default(TaskbarQuerySchedule).Asked(S(0)).Answered(A).Due(S(2)));
    [Fact] public void Two_answers_that_agree_settle_it() => Assert.False(Settled(A, 0).Due(S(2)));
    [Fact] public void Two_missing_answers_agree_too() => Assert.False(Settled(null, 0).Due(S(2)));
    [Fact] public void An_answer_that_differs_asks_again() => Assert.True(Settled(A, 0).Asked(S(16)).Answered(B).Due(S(18)));
    [Fact] public void An_answer_going_missing_differs() => Assert.True(Settled(A, 0).Asked(S(16)).Answered(null).Due(S(18)));
    [Fact] public void Answers_that_never_agree_keep_asking()
    {
        var s = default(TaskbarQuerySchedule);
        for (var i = 0; i < 50; i++)
        {
            s = s.Asked(S(i * 2)).Answered(i % 2 == 0 ? A : B);
            Assert.True(s.Due(S(i * 2 + 2)));
        }
    }
    [Theory, InlineData(14.9, false), InlineData(15, true), InlineData(16, true), InlineData(3600, true)]
    public void A_settled_schedule_checks_after_15_seconds(double later, bool due) =>
        Assert.Equal(due, Settled(A, 100).Due(S(100 + later)));
    [Fact] public void A_foreground_switch_asks_at_the_next_tick() => Assert.True(Settled(A, 0).ForegroundSwitched().Due(S(2)));
    [Fact] public void Many_switches_ask_once() =>
        Assert.False(Settled(A, 0).ForegroundSwitched().ForegroundSwitched().ForegroundSwitched().Asked(S(2)).Answered(A).Due(S(4)));
    [Fact] public void A_switch_while_a_question_is_out_asks_again_after_the_answer() =>
        Assert.True(Settled(A, 0).Asked(S(16)).ForegroundSwitched().Answered(A).Due(S(18)));
    [Fact] public void A_clock_that_went_back_is_not_due() => Assert.False(Settled(A, 100).Due(S(50)));
    [Fact] public void The_slow_check_is_15_seconds() => Assert.Equal(S(15), TaskbarQuerySchedule.SlowCheck);
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `dotnet test -c Release --filter "FullyQualifiedName~TaskbarQueryScheduleTests"`
Expected: a build error, `TaskbarQuerySchedule` not found.

- [ ] **Step 3: Implement `TaskbarQuerySchedule`** as in Interfaces. File-scoped namespace, XML doc comments in the style of `PillShow.cs` beside it.

- [ ] **Step 4: Run the tests**

Run: `dotnet test -c Release --filter "FullyQualifiedName~TaskbarQueryScheduleTests"`
Expected: all pass. Then `dotnet test -c Release`: all pass (956 before this task, plus the new ones).

- [ ] **Step 5: Commit**

```bash
git add src/Sobreiro.Core/Taskbar/TaskbarQuerySchedule.cs tests/Sobreiro.Tests/TaskbarQueryScheduleTests.cs
git commit -m "Core: a schedule for when the pill asks the taskbar"
```

### Task 2: `PillHost` asks by the schedule

**Files:**
- Modify: `src/Sobreiro/Pill/PillHost.cs`

**Interfaces:**
- Consumes: `TaskbarQuerySchedule` from Task 1, exactly as listed there.
- Produces: nothing other code uses.

No unit test: `PillHost` is WPF and Win32 and has none today. The controller checks it afterwards with a scratch run.

- [ ] **Step 1: Add the schedule and a monotonic clock to `PillHost`**

Fields: `private TaskbarQuerySchedule _schedule;`, `private readonly long _clockStart = Stopwatch.GetTimestamp();`, and `private TimeSpan Now => Stopwatch.GetElapsedTime(_clockStart);`.

- [ ] **Step 2: Wire the four facts**

- `Tick`: replace the unconditional `AskUiAutomation();` with `if (_schedule.Due(Now)) AskUiAutomation();`.
- `AskUiAutomation`: where the query is really sent (just before `Task.Run`, after the `_querying` guard), `_schedule = _schedule.Asked(Now);`. A call that only sets `_requery` records nothing.
- `OnProbed`: inside the branch that accepts the answer (`taskbar == _taskbar && generation == _generation`), `_schedule = _schedule.Answered(answer);`, for a null answer too. A stale answer is not fed in.
- `OnForeground`: when `Active`, `_schedule = _schedule.ForegroundSwitched();` before `Place()`. It sends no question itself.
- Everywhere `_generation++` appears (`Stop`, `AttachTo`, `Place`'s DPI branch, `OnDisplaySettingsChanged`): `_schedule = default;` beside it, before any `AskUiAutomation()` that follows.

Every other call to `AskUiAutomation()` stays as it is. Update the class's summary comment and add one short comment at the `Tick` line saying why the question is conditional (the cost: 1,800 questions an hour was most of the idle processor time).

- [ ] **Step 3: Build and test**

Run: `dotnet build -c Release` then `dotnet test -c Release`
Expected: no warnings that were not there before; all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/Sobreiro/Pill/PillHost.cs
git commit -m "Pill: ask the taskbar on a sign of change, and every 15 seconds otherwise"
```

### Task 3: Measure and write it down (the controller, not a subagent)

**Files:**
- Modify: `docs/baseline.md` (a row under Experiments, written by the script, and a note under "Notes on an experiment")
- Modify: `docs/roadmap.md` (the paragraph "Carried over from 1.0: idle CPU")

- [ ] **Step 1: Scratch run beside the installed copy.** Package with `scripts/package.ps1 -OutputDir dist/profile-after`, run the session's sampler for 7 minutes signed out, and compare with the as-shipped runs (12.2 s and 15.2 s). Expected: about 3.5 s per idle hour. A figure above 5 s stops the work: back to the profile.
- [ ] **Step 2: Full baseline run, only with Ricardo's yes in the conversation** (the installed copy is stopped for about 12 minutes and started again): `./scripts/baseline.ps1 -Label '1.0.0, taskbar asked on change' -Section Experiments`.
- [ ] **Step 3: Write the note and the roadmap paragraph** from the measured figures: where the time went (the table in the spec), what changed, the new figure against 14.9 s, and what it costs (a pill up to 16 seconds late when the tray changes with no window switched).
- [ ] **Step 4:** `dotnet test -c Release` and `npm run check` pass; commit; push; open the pull request with its full description (never a draft).
