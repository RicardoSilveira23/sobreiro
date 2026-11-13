# The pill keeps its figures: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A pill block whose numbers have stopped keeps its last figures, each followed by `*`, and its ring or bar keeps its fill in grey; the two words go.

**Architecture:** The rule for "stopped" stays in `Sobreiro.Core/Display` and now answers yes or no instead of two words. `PillModel` keeps a stopped block's figures and fractions and carries a flag per line and per block. `PillView` draws the `*` and the grey from those flags.

**Tech Stack:** C# on .NET 10, xUnit, WPF for the view.

**Spec:** `docs/superpowers/specs/2026-11-13-pill-keeps-figures-design.md`

## Global Constraints

- The rule for when does not change: `MissesNeeded = 2`, `ClaudeAfter = 15 minutes`, `CopilotAfter = 24 hours`, the six stopped states (`Expired`, `NoSignIn`, `Refused`, `Offline`, `BadAnswer`, `RateLimited`).
- The cue is `*` straight after the figure, in the label font and size, and the fill in the taskbar's ink at alpha `0x80`. The track keeps `TaskbarColours.TrackAlpha`.
- A line with no figure (`–`) never gets a `*`. A Claude block with no session or weekly reading is not stopped: dashes, empty rings.
- Not touched: the flyout's sentences, the tray icons, the tooltips, the polling, any network call, how a token is read.
- No other project is named or linked in code comments, docs, commits or the pull request.

## Review Focus

1. Both figures at `100%` and stopped: the block is one `*` wider. Nothing may be clipped (Task 2, a pill on the snapshot sheet).
2. Only one of the two Claude readings exists and the block is stopped: that line has the `*`, the other reads a plain `–` (Task 1, `A_stopped_block_marks_only_the_figures_it_has`).
3. A fresh spend reading beside an old session reading must not keep the block fresh, and a spend reading alone is nothing to keep (Task 1, `A_spend_reading_does_not_count`).
4. Copilot stopped while Claude is fresh, and the reverse: each block decides alone (Task 1, `Each_block_decides_alone`; Task 2, the sheet).
5. The status line still delivers fresh numbers while the poll fails: no cue (Task 1, `Fresh_numbers_are_not_stale`).

---

### Task 1: The rule and the model in Core

**Files:**
- Rename: `src/Sobreiro.Core/Display/PillWords.cs` to `PillStale.cs`
- Modify: `src/Sobreiro.Core/Display/PillModel.cs`, `src/Sobreiro.Core/Colours/TaskbarColours.cs`
- Rename: `tests/Sobreiro.Tests/PillWordsTests.cs` to `PillStaleTests.cs`
- Modify: `tests/Sobreiro.Tests/PillModelTests.cs`

**Interfaces (produces):**
- `public static class PillStale` with `MissesNeeded`, `ClaudeAfter`, `CopilotAfter` as today, and
  `static bool Claude(ProviderState state, IReadOnlyList<Limit> limits, DateTimeOffset now, int misses)`,
  `static bool Copilot(ProviderState state, Limit? credits, DateTimeOffset now, int misses)`.
- `record PillLine(string Label, string Figure, bool Stale = false)`
- `record PillCopilot(PillLine Credits, string ResetDate, double Fraction, UsageLevel Level, bool Stale = false)`
- `record PillModel(PillLine Session, PillLine Weekly, RingsModel Rings, PillCopilot? Copilot = null, bool ShowClaude = true, bool ClaudeStale = false)`; the three `From` overloads keep their signatures.
- `TaskbarColours.StaleAlpha = 0x80`

- [ ] **Step 1: Tests first, `PillStaleTests`.** Same helpers as today (`Now`, `Two`, `Aged`, `Credits`). `Old` is a session and a weekly reading 3 days old.
  - `Every_stopped_state_marks_claude` (theory, the six states): `Claude(state, Old, Now, Two)` is true.
  - `Off_and_ok_are_never_stale` (theory): false with `Old`.
  - `One_missed_poll_is_not_stale` (theory, 0 and 1): false for every stopped state, both blocks.
  - `Two_missed_polls_are_needed`: `MissesNeeded == 2`.
  - `Fresh_numbers_are_not_stale`: `Expired` with both readings exactly 15 minutes old is false; 15 minutes and 1 second is true.
  - `The_newest_of_the_two_readings_decides`: session 3 h old with weekly 1 min old is false; weekly 1 min old alone is false.
  - `A_spend_reading_does_not_count`: `Offline` with a spend reading 1 min old and a session reading 3 h old is true; with the spend reading alone is false.
  - `Nothing_to_keep_is_not_stale` (theory, the six states): `Claude(state, [], Now, Two)` is false.
  - `A_reading_from_the_future_is_fresh`: session 10 minutes ahead is false.
  - `Every_stopped_state_marks_copilot` (theory, the five without `Expired`): true with credits 25 h old. `Copilot_ok_is_not_stale`: false.
  - `A_copilot_figure_from_today_is_not_stale`: 2 h and exactly 24 h are false.
  - `No_reading_is_not_stale` (theory): `Copilot(state, null, Now, Two)` is false.
  - `The_grey_is_the_ink_at_half_strength`: `TaskbarColours.StaleAlpha == 0x80`.
- [ ] **Step 2: Tests first, `PillModelTests`.** Replace the five tests that mention words (from `Stopped_claude_block_carries_the_words_and_empty_rings` to `The_three_argument_From_never_has_words`) with:
  - `Stopped_claude_block_keeps_its_figures_and_rings`: session and weekly 20 min old, `Expired`, 2 misses: `ClaudeStale`, `Session == new PillLine("5h", "40%", true)`, `Weekly == new PillLine("7d", "40%", true)`, `Rings == RingsModel.From(limits)`.
  - `A_stopped_block_marks_only_the_figures_it_has`: only the session reading: `ClaudeStale`, `Session.Stale`, `Weekly == new PillLine("7d", "–")`.
  - `Nothing_to_keep_reads_plain_dashes`: `[]`, `Expired`, 2 misses: not `ClaudeStale`, both lines plain dashes, `Rings == RingsModel.From([])`.
  - `One_missed_poll_marks_nothing`: as today's `One_missed_poll_keeps_the_figures`, asserting `ClaudeStale`, `Session.Stale` and `Copilot.Stale` are false.
  - `Fresh_claude_block_is_not_stale`: readings 1 min old, `Expired`, 2 misses: not `ClaudeStale`, lines not stale.
  - `Stopped_copilot_block_keeps_its_figure_and_bar`: credits (2,644 of 15,000) 25 h old, `Offline`, 2 misses: `Stale`, `Credits == new PillLine("AI", "17%", true)`, `ResetDate == "1 Dec"`, `Fraction` is .176 to 3 places.
  - `Each_block_decides_alone`: Claude readings 1 min old with `Ok`, credits 3 days old with `NoSignIn` and 2 misses: Copilot stale, Claude not; then Claude readings 3 h old with `Refused` and 2 misses beside credits 1 h old with `Ok`: Claude stale, Copilot not.
  - `The_three_argument_From_marks_nothing`: readings 3 days old: nothing stale.
- [ ] **Step 3: Run `dotnet test -c Release`.** Expected: it does not compile (`PillStale` and the `Stale` members do not exist).
- [ ] **Step 4: Implement.** `PillStale` keeps the constants and the age rules; `Claude` also needs at least one session or weekly reading. `PillModel.From` (eight arguments) builds lines and rings from the full list and sets `Stale` on a line only when the block is stale and the line has a reading. `CopilotBlock` keeps the real fraction. Delete the word pairs.
- [ ] **Step 5: Run `dotnet test -c Release`.** Expected: `tests/Sobreiro.Tests` passes; the tray project still fails to build until Task 2 (it reads `ClaudeWords`). Build Core and tests only: `dotnet test tests/Sobreiro.Tests -c Release`.

### Task 2: The view and the snapshot sheet

**Files:**
- Modify: `src/Sobreiro/Pill/PillView.cs`, `src/Sobreiro/Snapshot.cs`

**Interfaces (consumes):** Task 1's `PillLine.Stale`, `PillCopilot.Stale`, `PillModel.ClaudeStale`, `TaskbarColours.StaleAlpha`, `PillStale.MissesNeeded`.

- [ ] **Step 1: `PillView`.** Remove `Word` and both words branches. `Line` adds a `Run("*")` in Inter at `LabelSize` after the figure when `line.Stale`. `Rings` and `Ring` take a `stale` flag; a stale ring's arc is `Solid(colours.Ink, TaskbarColours.StaleAlpha)` in place of the level colour. `LevelBar` does the same when `copilot.Stale`. No mutable static state.
- [ ] **Step 2: `Snapshot.PillStrip`, second row.** Replace the six per-state pills with, in order: Claude stopped (37 and 78, three days old, `Expired`); Claude stopped beside Copilot stopped (`Refused`, `Offline`); Claude fresh beside Copilot stopped (`Ok`, `NoSignIn`); Claude stopped at 100 and 100; nothing to keep (`[]`, `Expired`). Update the comment above the row.
- [ ] **Step 3: Build and render.** `dotnet build -c Release` with 0 warnings, then `Sobreiro.exe --snapshot <file.png>` and look at the two pill strips: every `*` whole, the grey fill visible on both grounds, the 100% pill not clipped, the last pill plain dashes. Delete the picture afterwards.
- [ ] **Step 4: Commit** Tasks 1 and 2 together (the solution builds again).

### Task 3: The words that describe it

**Files:**
- Modify: `README.md:55-56`, `site/manual/index.html:102`, `docs/roadmap.md:19,50-54,102`, `docs/superpowers/specs/2026-11-13-pill-says-why-design.md` (one line under the title pointing at the new spec for the pill half)

- [ ] **Step 1: README.** "When a block's numbers have stopped, it keeps its last figures, each followed by `*`, and its ring or bar turns grey; the details say why and what to do."
- [ ] **Step 2: Manual.** The same fact in the manual's voice, keeping its sentence on when: after two checks in a row brought nothing and the last reading is over 15 minutes old (Copilot: over a day old).
- [ ] **Step 3: Roadmap.** The 1.0 item says the pill marks figures that have stopped and the flyout names the cause; both test counts become the number `dotnet test -c Release` prints.
- [ ] **Step 4: Run `dotnet test -c Release` and `npm run check`.** Expected: both pass.
- [ ] **Step 5: Commit.**
