# Baseline

How heavy each release is: one row per release, measured the same way each time, so a change
that makes Sobreiro heavier or slower shows up as a number. The 1.0 targets in
[roadmap.md](roadmap.md) are checked against the newest row.

## Run it

```powershell
./scripts/baseline.ps1
```

It packages a release build, measures it and writes the row under [Releases](#releases). It
takes about 12 minutes. An experiment gets a name and goes under [Experiments](#experiments):

```powershell
./scripts/baseline.ps1 -Label '0.10.0, what was tried' -Section Experiments
```

Packaging needs the MSVC linker, for the bridge (see the README's "Building from source"). On a
PC without it, download the two exes CI built and pass them with `-Exe` and `-Bridge`. With
`-BridgeOnly` the tray is not started: only the two sizes and the bridge's run time are taken,
in seconds.

Quit any running Sobreiro first, and start it again afterwards. A second pill finds no room
beside the first one and stays hidden, so the script stops when it sees a pill in the taskbar.
The copy it starts keeps its data in a scratch folder: an installed copy's data, its
`settings.json` and its Run key are never touched.

## What each figure is

- **Private memory, working set:** at the end of a run left alone for 10 minutes. This run uses
  the PC's own sign-ins, so it polls like an installed copy.
- **CPU per idle hour:** the processor time that run used from the end of its first minute to
  the end of its tenth, scaled to an hour. The first minute is left out because it holds the
  start.
- **First pill, first start:** from launching the exe to a visible pill inside the taskbar, on
  the first start after packaging. That start also unpacks the exe's native libraries.
- **First pill, later starts:** the median of the four starts after it. The five timed starts
  run signed out of Claude Code (an empty config folder), so starting five times in a row does
  not ask for usage five times in a row.
- **Sobreiro.exe, bridge exe:** file sizes. MB is 1024 × 1024 bytes, as Windows shows it.
- **Bridge run:** the bridge's whole run on one status line, launch to exit: the median of 20.

The pill is recognised from outside: a visible window of the measured process, owned by the
taskbar and wholly inside it. Nothing in the app is switched on for the measurement, so the
same tool can time any shell that draws a pill there. The logic is in
`src/Sobreiro.Core/Baseline`, the measuring in `src/Sobreiro.Baseline`.

The rows below were measured on one PC: Ryzen 7 7700X, 32 GB, Windows 11 Pro 26200, a
2560 × 1440 display at 100%. Figures from another PC are not comparable with them.

## Releases

| Build | Date | Private memory | Working set | CPU per idle hour | First pill, first start | First pill, later starts | Sobreiro.exe | Bridge exe | Bridge run |
|---|---|---|---|---|---|---|---|---|---|
| 0.10.0 | 2026-11-13 | 184 MB | 331 MB | 13.9 s | 0.62 s | 0.55 s | 80.4 MB | 17.0 MB | 43 ms |
| 1.0.0 | 2026-11-13 | 97 MB | 173 MB | 14.9 s | 0.43 s | 0.40 s | 168.9 MB | 2.2 MB | 9 ms |

Notes on a row:

- **0.10.0:** private memory was still rising at the end, from 169 MB after the first minute to
  184 MB after the tenth, so 10 minutes is not yet its resting level. Processor time was steady
  at about 0.2 s a minute, with a little more in the minute that held a poll.

## Experiments

Changes that were tried, each beside the release it was tried on, kept or not.

| Build | Date | Private memory | Working set | CPU per idle hour | First pill, first start | First pill, later starts | Sobreiro.exe | Bridge exe | Bridge run |
|---|---|---|---|---|---|---|---|---|---|
| 0.10.0, bridge as a trimmed single file | 2026-11-13 | not measured | not measured | not measured | not measured | not measured | 80.4 MB | 17.0 MB | 43 ms |
| 0.10.0, bridge with Native AOT | 2026-11-13 | not measured | not measured | not measured | not measured | not measured | 73.7 MB | 2.2 MB | 9 ms |
| 0.10.0, tray compressed | 2026-11-13 | 181 MB | 314 MB | 15.8 s | 0.64 s | 0.54 s | 73.7 MB | 2.2 MB | 8 ms |
| 0.10.0, tray without compression | 2026-11-13 | 98 MB | 174 MB | 13.0 s | 0.84 s | 0.38 s | 168.9 MB | 2.2 MB | 9 ms |
| 1.0.0, taskbar asked on change | 2026-11-13 | 94 MB | 170 MB | 7.3 s | 0.48 s | 0.39 s | 168.9 MB | 2.2 MB | 9 ms |
| 1.0.0, pill on a plain window (spike) | 2026-11-13 | 27 MB | 96 MB | 5.4 s | 0.33 s | 0.25 s | 170.2 MB | 2.2 MB | 8 ms |
| 1.0.0, tiered compilation off | 2026-11-13 | 103 MB | 173 MB | 3.0 s | 0.57 s | 0.47 s | 168.9 MB | 2.2 MB | 9 ms |

Notes on an experiment:

- **Bridge with Native AOT: kept.** Both figures improved: 17.0 MB to 2.2 MB, and a run from
  43 ms to 9 ms. `Sobreiro.exe`, which carries the bridge, went from 80.4 MB to 73.7 MB. The
  Native AOT bridge in the row was built by CI and timed on the PC above, which had no MSVC
  linker then; the 73.7 MB is the size CI printed for its own build. A build on that PC later
  the same day, with the linker installed, gave the same three figures. Both rows are
  bridge-only runs: the tray was not started, since the change does not touch it.
- **Tray without compression: kept.** Both tray rows are the same commit, packaged twice with
  `scripts/package.ps1 -Compression On` and `-Compression Off` and measured back to back.
  Compressed, the single file is unpacked into memory at every start, and that copy stays for
  as long as the app runs. Without compression Windows maps the file instead:
  - private memory fell from 181 MB to 98 MB and the working set from 314 MB to 174 MB, which
    meets the 1.0 target of 150 MB;
  - later starts went from 0.54 s to 0.38 s;
  - the first start went from 0.64 s to 0.84 s. That is one start of each, still inside the
    1 s target, and why it is slower was not looked into;
  - `Sobreiro.exe` grew from 73.7 MB to 168.9 MB. That is the price: a download more than
    twice the size, paid once per release, against 83 MB of memory for as long as the app is
    open.

  The two CPU figures, 15.8 s and 13.0 s, are not a finding: two runs of compressed builds on
  this day gave 13.9 s and 15.8 s. Memory was still rising slowly at the end of both runs.
  `scripts/package.ps1` now publishes without compression unless told otherwise.
- **Taskbar asked on change: kept. 14.9 s to 7.3 s of processor time per idle hour; the 5 s
  target is not met.** The row is `main` at 1.0.0 plus this one change, measured the same day
  as the 1.0.0 row.
  - What the profile found. Copies of 1.0.0 were run beside an installed one, each on its own
    data folder, signed out of Claude, their pills hidden because the installed pill had the
    place. Processor time from the end of minute 1 to the end of minute 7, scaled to an hour:

    | Copy | CPU per idle hour | Private memory |
    |---|---|---|
    | As shipped, three runs | 12.2 s, 15.2 s, 8.4 s | 91 MB, 105 MB, 93 MB |
    | Same exe, pill switched off in its config | 1.4 s | 19 MB |
    | The tick as shipped, without its question to UI Automation | 1.9 s | 76 MB |
    | With this change, two runs | 3.7 s, 3.1 s | 77 MB, 81 MB |

    So the question the pill put to UI Automation on every 2-second tick (where the tray icons
    and the task buttons are) was 85 to 90% of the idle time: 1,800 questions an hour. The
    window-event hooks cost nothing when nobody is at the PC, and one question costs the same
    whether it fetches the taskbar in one call or in several. The second as-shipped run leaves
    out one minute of 1.27 s that looks like a flyout opening and was not checked.
  - What changed. The pill keeps its 2-second tick for the cheap checks and asks UI Automation
    only on a sign of change (start, a display or taskbar change, a foreground switch, an
    answer that differs from the last) and every 15 seconds otherwise.
  - What it costs. A tray icon that appears or leaves while no window is switched can leave
    the pill out of place for up to 16 seconds.
  - Why 7.3 s and not the 3 s to 4 s of the hidden copies is not known. The baseline run is
    signed in and shows its pill; the copies were neither. Its quiet minutes used 0.05 s to
    0.10 s and four of its nine minutes used 0.14 s to 0.25 s, one of them the minute the
    5-minute usage poll falls in. That run has not been profiled.
  - Memory, start time and sizes did not move: 94 MB against 97 MB is inside what two runs of
    the same build differ by.
- **Pill on a plain window: a spike, not merged.** The first spike of the next shell
  ([shell-comparison.md](shell-comparison.md#f-c-the-pill-on-a-plain-system-window)): the row
  above it with only the pill window taken off WPF, drawn by hand on a plain system window,
  and the taskbar asked through UI Automation's own interface. Measured the same evening.
  - Private memory 94 MB to 27 MB, working set 170 MB to 96 MB, later starts 0.39 s to 0.25 s.
  - 7.3 s to 5.4 s of processor time per idle hour, one run of each. Minute by minute: 0.06,
    0.17, 0.06, 0.07, 0.15, 0.08, 0.08, 0.04 and 0.10 s.
  - The run never opens the flyout, which is still WPF. In a separate copy, opening it once
    took private memory from 24.5 MB to 94 MB, and it stayed at 92 to 94 MB for the five
    minutes it was watched. The row is an app whose flyout has not been opened yet.
  - `Sobreiro.exe` is 1.3 MB larger because the spike carries its fonts twice.
  - The code is on the branch `spike/shell-f-plain-window`, cut from `main` at `f6c7df4`: the
    row above it is the same code without the spike.
- **Tiered compilation off: kept. 7.3 s to 3.0 s of processor time per idle hour; the 5 s
  target is met, on one run.** The row is the "taskbar asked on change" build plus one line in
  the tray's project file (`main` at `c205007`, WPF pill, no spike), measured the same evening.
  - What a second profile found. A copy of the "taskbar asked on change" build, signed in and
    showing its pill, used 7.4 s per idle hour, so the 7.3 s repeated. By thread, with someone
    at the PC (2 to 6 window switches a minute):

    | Where | s per idle hour |
    |---|---|
    | The runtime recompiling in the background | 3.2 |
    | Thread-pool workers (the taskbar questions, the usage poll) | 2.1 |
    | UI Automation's own thread | 0.6 |
    | UI thread | 0.5 |
    | Threads that came and went between samples | about 1 |

    By default the runtime compiles code quickly the first time it runs and again, optimised,
    in the background once it has run often. Sobreiro's code runs a little every few seconds
    and every five minutes, so pieces of it kept crossing that line for as long as the tray ran.
  - What was tried. Four copies side by side for 30 minutes, signed out, pills hidden, nobody
    at the PC, the settings given through environment variables. Processor time from the end of
    minute 1 to the end of minute 30, scaled to an hour:

    | Copy | CPU per idle hour | Of it, recompiling | CPU in minute 1 |
    |---|---|---|---|
    | As shipped | 2.6 s | 1.2 s | 0.86 s |
    | Profile-guided recompiling off | 2.3 s | 0.6 s | 0.83 s |
    | Tiered compilation off | 1.7 s | 0 | 0.78 s |
    | Precompiled when published, settings as shipped | 2.8 s | 1.3 s | 0.67 s |

    The recompiling fades slowly and was not gone at 30 minutes: 0.25 s in minutes 2 to 10,
    0.19 s in 11 to 20, 0.16 s in 21 to 30. Precompiling alone does not cut it, since
    precompiled code is recompiled too once it runs often, and it made `Sobreiro.exe` 186.8 MB.
    Another session's test copies ran beside these for their first seven minutes.
  - What changed. `<TieredCompilation>false</TieredCompilation>` in `src/Sobreiro/Sobreiro.csproj`:
    code is compiled once, optimised, the first time it runs. Built that way and run beside the
    as-shipped exe for 10 minutes, signed out and hidden: 1.3 s per idle hour against 3.3 s.
  - What it costs. The start is slower by 0.06 s to 0.09 s on each of the five timed starts:
    later starts 0.39 s to 0.47 s, the first start 0.48 s to 0.57 s, inside the 1 s target.
    Private memory was 103 MB against 94 MB at the end of the run, and 95 MB against 80 MB
    after its first minute; one run of each, and it was still rising in both.
  - Minute by minute, this run used 0.08, 0.02, 0.01, 0.05, 0.06, 0.03, 0.07, 0.07 and 0.07 s;
    the run before the change 0.14, 0.10, 0.09, 0.05, 0.25, 0.06, 0.17, 0.06 and 0.18 s.
  - It is one run. Two baseline runs of one build have differed by 1.9 s on this PC (13.9 s and
    15.8 s, above), which is less than the 4.3 s between these two rows, and the copies run
    side by side point the same way.
