# Roadmap: Sobreiro 1.0 to 3.0

Written 2026-11-13. Today's milestones are in [plan.md](plan.md); this covers what comes after
them. An earlier draft made "a Rust core under the C# UI" its own version. That step is dropped:
see [Not in this roadmap](#not-in-this-roadmap).

The next shell was chosen on 2026-11-14: Rust, for 2.0. [The decision](#the-decision) has its
numbers. 1.1 came first, released the same day: [today's shell, tuned](#11-todays-shell-tuned).

## Where it stands

Checked on 2026-11-13 against 1.0.0, which publishes the bridge with Native AOT and the tray
without single-file compression. The memory, CPU, start and file rows are the newest row in
[baseline.md](baseline.md), 1.1.0's, measured on 2026-11-14; it also says how each one is
taken. 0.10.0 as released measured 184 MB private memory, an 80.4 MB `Sobreiro.exe` and a
17.0 MB bridge.

| What | Value |
|---|---|
| Code | 8,304 lines of C#: Core 4,220, tray app 4,059, bridge 25 |
| Core | Plain `net10.0`, no native calls, no runtime packages. Its only Windows ties are paths |
| Tray app | WPF and WinForms, native calls in 12 files (pill, flyout, autostart, Credential Manager, theme) |
| Tests | 956, all against Core. CI runs on Windows only |
| Polling | Every 5 minutes, 2-minute retry, stepped back-off on rate limits |
| Memory | 96 MB private, 167 MB working set, after 10 minutes idle and still rising slowly |
| CPU | 0.8 s in the first minute, then 2.0 s per idle hour |
| Start | 0.47 s from launch to the first pill; 0.54 s on the first start |
| Files | `Sobreiro.exe` 168.9 MB, `sobreiro-bridge.exe` 2.2 MB; one bridge run takes 9 ms |

## What decides the order

- **The memory is the runtime and the UI toolkit, not the logic.** Core does one HTTP call per
  provider every 5 minutes. Moving Core alone to another language leaves the 97 MB where it is.
  (It was 184 MB while the exe was compressed: 83 MB of that was the exe unpacked into memory.)
- **Core is small and nearly OS-neutral.** Sharing it across systems is the easy part.
- **The pill is the product, and the part most tied to Windows.** Any new shell is judged on
  the pill first.
- **No rewrite without numbers.** Each step below ends in a measurement, and the next step
  starts only if that measurement justifies it.

## 1.0: a finished Windows app

Stays on C#, WPF and .NET 10.

- A baseline script that records, on a clean start: private memory and working set after
  10 minutes idle, CPU seconds per idle hour, time from start to the first drawn pill, and both
  exe sizes. Results go in `docs/baseline.md`, one row per release.
- Two packaging experiments, each kept only if the baseline improves:
  - publish the bridge with Native AOT, as plan.md describes. Done and kept on 2026-11-13:
    17.0 MB and 43 ms became 2.2 MB and 9 ms;
  - publish the tray without single-file compression. Done and kept on 2026-11-13: private
    memory went from 181 MB to 98 MB and a later start from 0.54 s to 0.38 s, for an exe of
    168.9 MB where it was 73.7 MB.
- Defined, tested behaviour for each way the data can stop: an endpoint that changes or
  refuses, an expired Claude Code token, no sign-in at all. The flyout says which one it is.
  Done on 2026-11-13: each cause has its own state in Core and its own sentence in the
  flyout, and the pill keeps its last figures, marked with `*` and a grey fill, once two
  polls in a row have brought nothing and those figures are over 15 minutes old for Claude
  Code, a day old for Copilot.
- A provider interface in Core, now that there are two providers. Done on 2026-11-13: both
  sit behind one interface, and the polling loop they shared is in Core with tests.
- The Copilot test on a work PC. Done on 2026-11-13. A PC where Copilot is used only inside an
  editor has no sign-in Sobreiro can use: it shows the figure in the Copilot CLI's cache file
  and says so, and the install notes already say to sign in with the Copilot CLI or `gh`. A
  coworker's PC signed in with `gh` showed its numbers. What was found is in
  [the credential reader's design note](superpowers/specs/2026-11-13-copilot-credential-reader-design.md).

Version 1.0.0 was cut on 2026-11-13 with every item above done and the one unmet target below
given up in writing.

**Done when**

| Measure | Baseline (0.10.0) | Target |
|---|---|---|
| Private memory, 10 min idle | 184 MB | 150 MB or less (met without compression: 98 MB) |
| CPU per idle hour | 13.9 s | 5 s or less (not met: given up for 1.0, carried to 1.x) |
| Start to first pill | 0.55 s | 1 s or less, the first start included (without compression: 0.38 s, first start 0.84 s) |
| Bridge exe | 17.0 MB, runs in 43 ms | 5 MB or less, runs in 15 ms or less (met with Native AOT: 2.2 MB, 9 ms) |

The targets were checked against the first baseline row on 2026-11-13:

- **Memory: kept, and met on 2026-11-13.** Publishing the tray without compression brought it
  to 98 MB. That was not expected when the targets were checked: the packaging experiments were
  thought to change size and start time only.
- **CPU: given up for 1.0, in writing, on 2026-11-13 (Ricardo's call).** No item in the list
  above lowers it; the tray measured 13.0 s to 15.8 s per idle hour on 2026-11-13 whichever way
  it was packaged, which is under half a percent of one processor core. 1.0 ships with that.
  The 5 s target is not dropped: it is the first item of 1.x, below. The steady processor
  time, about 0.2 s a minute, most likely comes from checking the taskbar every 2 seconds; it
  has not been profiled.
- **Start to first pill: tightened.** The first run measured 0.55 s, so 2 s would have let the
  start get three times slower unnoticed.
- **Bridge: the 50 ms line is dropped.** The 0.10.0 bridge already ran in 43 ms, so that line
  could not tell the two ways of publishing apart. The Native AOT measurement set it to 15 ms.

**Nice to have, not a condition for 1.0**

Decided on 2026-11-13: 1.0 waits for neither. Both are open in [plan.md](plan.md),
milestone 5.

- Signed builds, so Windows stops warning about an unknown publisher. Until a build is signed,
  the README and the manual keep the step that gets past the warning. When signing comes, it
  goes into the release workflow and that step comes out.
- A winget manifest, so installing and updating is one command.

## 1.x: choose the next shell on numbers

Rust is one candidate, not the answer. No language is chosen until it has been compared with
the others, first on paper and then in running code.

Both were done by 2026-11-14, and Rust was chosen: see [The decision](#the-decision).

### Carried over from 1.0: idle CPU

5 s of processor time or less per idle hour, measured with the baseline script as before. 1.0
shipped at 14.9 s (13.0 s to 15.8 s across the runs of 2026-11-13). It does not wait for the
shell decision: whatever lowers it on the current shell is worth having in a 1.x release.

- **Profiled, 2026-11-13.** 85 to 90% of the idle time was the pill asking UI Automation where
  the taskbar's parts are, on every 2-second tick. The figures are in
  [baseline.md](baseline.md), under Experiments.
- **First change: 7.3 s, not yet 5 s.** The pill now asks on a sign of change and every 15
  seconds otherwise ([the design](superpowers/specs/2026-11-13-idle-cpu-taskbar-query-design.md)).
  The baseline run measured 7.3 s per idle hour, half of 1.0's figure and still over the target.
- **Second profile, 2026-11-13.** Signed in and showing its pill, the tray used 7.4 s per idle
  hour, and 3.2 s of it was the runtime recompiling code in the background, which it kept doing
  for as long as the tray ran.
- **Second change: 3.0 s, the target is met.** The tray's code is now compiled once, when it
  first runs (tiered compilation off, one line in the project file). The baseline run measured
  3.0 s per idle hour against 7.3 s. The price is a start about 0.08 s slower (first pill after
  0.47 s, first start 0.57 s) and 9 MB more private memory (103 MB), both inside their targets.
- **Released, and it holds.** [1.1.0](#11-todays-shell-tuned) came out on 2026-11-14 with both
  changes. Its row under Releases in [baseline.md](baseline.md), measured on the released exe,
  shows 2.0 s per idle hour: a second run under the target, after the 3.0 s on `main`.

### Step 1: compare on paper

A short table, a day or two of work. It is written: [shell-comparison.md](shell-comparison.md),
with its shortlist and what it leaves open for step 2. The candidates it started from:

- **C# with Avalonia, published with Native AOT.** Keeps Core and its 956 tests untouched; only
  the shell is ported.
- **Rust with Slint.** Low expected memory; licence fits GPL-3.0-or-later. Its tray element
  shipped in June 2026 and is still young; the pill and flyout need raw window handles.
- **C++ with Qt.** The longest-proven tray support on all three systems and low memory; a
  heavier build and manual memory management.
- **Go with a small native tray and window library.** Simple toolchain and one binary; thin UI
  toolkits, so most of the pill would be hand-written native calls.
- **A native shell per system** (the current one on Windows, Swift on macOS, a small one on
  Linux) sharing Core or only its file formats. The best fit on each desktop; the most code.

Any other language that looks promising when the comparison is made joins the table. Toolkits
that carry a browser engine or a large runtime of their own are left out unless the table shows
a reason to keep one.

Each candidate is scored on:

- expected idle memory, exe size and start time;
- whether the pill is possible at all, and how much native code it needs;
- how mature its tray, menu bar and notification support is on each of the three systems;
- how much of Core and its tests survive;
- the cost of the toolchain, CI and signing on three systems;
- whether its licence fits GPL-3.0-or-later;
- how well the maintainer knows it.

The table ends in a shortlist of at most three. The candidate that keeps Core is always on it,
as the one the others have to beat.

### Step 2: build the shortlist

A time-boxed spike, one week per candidate, thrown away afterwards. Each builds only the tray
icon, the pill and the flyout on Windows. It must show the pill inside the taskbar (owned by
Explorer's taskbar, transparent, right on mixed-DPI monitors) and the flyout opening beside it,
then run the 1.0 baseline script.

**Decision rule**

- No candidate cuts private memory by 40% against 1.0, and nobody has asked for macOS or Linux:
  stop. Stay on WPF and close this roadmap at 1.x.
- A candidate that needs Core rewritten wins only if its pill works and it uses half the memory
  of the candidate that keeps Core, or less. If more than one does, the one with the smaller
  port and the more mature tray support wins.
- Otherwise: the candidate that keeps the tested Core.

### The decision

Taken on 2026-11-14, by Ricardo, on the three spikes' figures: **B, the shell in Rust, for 2.0.**
Each spike's note is in [shell-comparison.md](shell-comparison.md#what-the-spikes-measured) and
its row in [baseline.md](baseline.md). All were measured on one PC, ten minutes idle, pill
shown; A and B draw in software.

| | 1.0.0 | F: C#, the pill on a plain window | A: C# on a toolkit, Core kept | B: Rust, Core rewritten |
|---|---|---|---|---|
| Private memory | 97 MB | 27 MB | 26 MB, still rising | 7 MB |
| Where it settles | Rising slowly | 94 MB once the flyout has been opened | 33 MB, or 16 to 17 MB with a collection every 30 seconds | 6.5 MB |
| CPU per idle hour | 14.9 s | 5.4 s | 1.4 s | 1.7 s |
| Start to first pill | 0.40 s | 0.25 s | 0.06 s | 0.06 s |
| Files | 168.9 MB | 170.2 MB | 34.9 MB in three files | 9.3 MB in one file |

The rule, line by line:

- **Stay on WPF?** No. 40% under 1.0 is 58 MB, and A and B are far under it. F is not, once its
  flyout has been opened.
- **Does the candidate that rewrites Core win?** Yes. B's pill was seen in the taskbar, with the
  flyout opening beside it on a click. Half of A's memory is 13 MB against its baseline row and
  8 MB against its lowest resting level; B rests at 6.5 MB.
- **Otherwise the candidate that keeps Core.** Not reached.

Known when it was taken:

- A alone takes 64 to 80 MB off 1.0's 97 MB. B takes 10 to 26 MB more, and for that all of
  Core is written again, with its 956 tests, as well as the shell.
- The maintainer has not worked in Rust. The port is checked by its tests and by how the app
  behaves, more than by reading it.
- The spike's rewrite of Core was made by machine in half an hour and was not reviewed. It is
  something to read, not something to build on.
- Not shown yet for B: the pill on monitors of different scale, a tooltip above the taskbar,
  and a pill owned by the taskbar without the toolkit's unstable hook. 2.0 starts there.

### 1.1: today's shell, tuned

The release of the current shell with the idle-CPU work in it: the best the current stack
measured. It is what is on `main`, 1.0.0 with the two changes under
[idle CPU](#carried-over-from-10-idle-cpu).

| | 1.0.0 | 1.1, as measured on `main` | 1.1.0, the released exe |
|---|---|---|---|
| Private memory, 10 minutes idle | 97 MB | 103 MB | 96 MB |
| CPU per idle hour | 14.9 s | 3.0 s | 2.0 s |
| Start to first pill | 0.40 s, first start 0.43 s | 0.47 s, first start 0.57 s | 0.47 s, first start 0.54 s |
| `Sobreiro.exe` | 168.9 MB | 168.9 MB | 168.9 MB |

The two 1.1 columns are one run each of the same code, a day apart: what lies between them is
what two runs differ by, not a change.

A, the C# shell that keeps Core, is not released: it would be a whole shell written to be
replaced by 2.0.

**Done when:** 1.1.0 is released, its row is under Releases in [baseline.md](baseline.md), and
that row shows 5 s or less per idle hour.

Done on 2026-11-14: 1.1.0 is released and its row shows 2.0 s.

## 2.0: the new shell on Windows, at parity

The new shell is B: Rust, on the toolkit its spike used, drawn in software.
[Its note](shell-comparison.md#b-rust-with-slint) says what was built and where it fell short.
In this order:

1. **The pill's three open questions,** before anything is ported: the right size on monitors
   of different scale, a tooltip above the taskbar, and an owner that does not lean on the
   toolkit's unstable hook (or that hook pinned to a version, with a test that fails when it
   changes). If one has no answer, the decision is opened again before more is built.
2. **Core, one area at a time, with its tests.** The spike's rewrite is read, not taken over.
3. **The shell to parity:** pill, flyout, tray icons and menu, Connect, alerts, start at
   sign-in, the migration.
4. **The release:** its baseline row, and the comparison below.

- The WPF app keeps shipping from a 1.x branch until the new one matches it. No release mixes
  two languages in one process.
- On a path that changes language, Core is ported in the same step, one area at a time, with
  its tests. The JSON fixtures in `tests/Sobreiro.Tests/Fixtures` are reused as they are.
- Nothing changes for an installed user: `config.json`, `usage.json`, `alerts.json`, the bridge's
  name and the `statusLine` entry stay as they are, and the security rules in
  [plan.md](plan.md) hold word for word.

**Done when:** every feature in the manual works, the 2.0 baseline beats 1.0 by the margin the
spike measured, upgrading over 1.0 needs no step from the user, and the comparison is published.

**The comparison, published with 2.0.** A table and graphs of 1.0, 1.1 and 2.0, drawn from each
release's row in [baseline.md](baseline.md): memory, idle CPU, start and size. It goes in the
docs and on the landing page, with a download for each of the three releases. 2.0's figures are
a spike's until it is released, so the comparison waits for that row.

## 3.0: macOS, then Linux

Starts only when people have asked for it in issues.

| | Windows | macOS | Linux |
|---|---|---|---|
| Glanceable figure | Pill in the taskbar | Menu bar item with text | Tray icon only; GNOME needs an extension |
| Claude Code sign-in | `.credentials.json` | Keychain (to verify) | `.credentials.json` |
| Copilot sign-in | Credential Manager, `gh` | Keychain, `gh` (to verify) | Secret service or a file, `gh` (to verify) |
| Start at sign-in | Run key | Login item | Autostart entry |
| Shipping | Signed exe, winget | Signed and notarised app; paid developer account | One package format first |

- macOS first: the menu bar gives the pill a natural home.
- Linux ships with a stated smaller scope, written in the manual before release.
- The token rules are rewritten per system before any code reads a keychain.
- CI builds and tests on all three.

## Structure

Two parts, not three layers; at this size a separate infrastructure layer is only indirection.

- **Core:** providers, polling, limits, config, file formats. No UI, no OS calls beyond files
  and HTTP.
- **Shell, one per system,** behind five small interfaces: glanceable figure, flyout,
  credentials, start at sign-in, paths.

```text
core/               providers, polling, limits, config
bridge/             status line bridge
shell/windows/      pill, flyout, Credential Manager
shell/macos/        menu bar item, Keychain
shell/linux/        tray icon, secret service
tests/fixtures/     JSON shared by every test suite
```

## Risks

| Risk | What limits it |
|---|---|
| Either usage endpoint changes or closes; both are undocumented | 1.0's defined failure behaviour; parsers treat every field as optional |
| A Windows update moves the taskbar's layout | The tray icon stays as the fallback; the spike tests the pill first |
| A language is chosen on reputation rather than on this app's numbers | The paper comparison, then the spike, before any port |
| A candidate's tray support is young or thin on one system | Scored in the comparison; the decision rule's fallback |
| Tests lost or weakened in a port | Port tests with the code, area by area; shared fixtures |
| The new shell is in a language the maintainer has not worked in | Tests ported with each area; 1.x keeps shipping until parity; the pill's open questions come first |
| Signing and notarisation cost money and time | Free signing for open source on Windows; macOS waits for demand |
| Linux desktops differ | One desktop and one package format first, scope stated |
| One maintainer, three systems | The demand gate on 3.0 |

## Not in this roadmap

**A Rust core under the C# UI.** The process would still load .NET and WPF, so memory would not
move, while the project would gain a second toolchain, a boundary between two languages and
660 tests to port. If a new language comes, it comes with the shell, in 2.0, and only after the
comparison in 1.x.
