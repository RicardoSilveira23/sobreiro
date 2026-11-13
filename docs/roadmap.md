# Roadmap: Sobreiro 1.0 to 3.0

Written 2026-11-13. Today's milestones are in [plan.md](plan.md); this covers what comes after
them. An earlier draft made "a Rust core under the C# UI" its own version. That step is dropped:
see [Not in this roadmap](#not-in-this-roadmap).

## Where it stands

Checked on 2026-11-13 against 1.0.0, which publishes the bridge with Native AOT and the tray
without single-file compression. The process figures are the newest row in
[baseline.md](baseline.md), which also says how each one is taken. 0.10.0 as released measured
184 MB private memory, an 80.4 MB `Sobreiro.exe` and a 17.0 MB bridge.

| What | Value |
|---|---|
| Code | 8,304 lines of C#: Core 4,220, tray app 4,059, bridge 25 |
| Core | Plain `net10.0`, no native calls, no runtime packages. Its only Windows ties are paths |
| Tray app | WPF and WinForms, native calls in 12 files (pill, flyout, autostart, Credential Manager, theme) |
| Tests | 956, all against Core. CI runs on Windows only |
| Polling | Every 5 minutes, 2-minute retry, stepped back-off on rate limits |
| Memory | 97 MB private, 173 MB working set, after 10 minutes idle and still rising slowly |
| CPU | 1.1 s in the first minute, then 14.9 s per idle hour |
| Start | 0.40 s from launch to the first pill; 0.43 s on the first start |
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
- **Next.** What is left has not been profiled the way the baseline measures it: signed in,
  with the pill shown. Copies run signed out with the pill hidden measured 3.1 s to 3.7 s, so
  the usage polls and the shown pill are the suspects, not the finding.

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

## 2.0: the new shell on Windows, at parity

- The WPF app keeps shipping from a 1.x branch until the new one matches it. No release mixes
  two languages in one process.
- On a path that changes language, Core is ported in the same step, one area at a time, with
  its tests. The JSON fixtures in `tests/Sobreiro.Tests/Fixtures` are reused as they are.
- Nothing changes for an installed user: `config.json`, `usage.json`, `alerts.json`, the bridge's
  name and the `statusLine` entry stay as they are, and the security rules in
  [plan.md](plan.md) hold word for word.

**Done when:** every feature in the manual works, the 2.0 baseline beats 1.0 by the margin the
spike measured, and upgrading over 1.0 needs no step from the user.

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
| Signing and notarisation cost money and time | Free signing for open source on Windows; macOS waits for demand |
| Linux desktops differ | One desktop and one package format first, scope stated |
| One maintainer, three systems | The demand gate on 3.0 |

## Not in this roadmap

**A Rust core under the C# UI.** The process would still load .NET and WPF, so memory would not
move, while the project would gain a second toolchain, a boundary between two languages and
660 tests to port. If a new language comes, it comes with the shell, in 2.0, and only after the
comparison in 1.x.
