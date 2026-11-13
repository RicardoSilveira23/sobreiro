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

Notes on a row:

- **0.10.0:** private memory was still rising at the end, from 169 MB after the first minute to
  184 MB after the tenth, so 10 minutes is not yet its resting level. Processor time was steady
  at about 0.2 s a minute, with a little more in the minute that held a poll.

## Experiments

Packaging changes that were tried, each beside the release it was tried on, kept or not.

| Build | Date | Private memory | Working set | CPU per idle hour | First pill, first start | First pill, later starts | Sobreiro.exe | Bridge exe | Bridge run |
|---|---|---|---|---|---|---|---|---|---|
| 0.10.0, bridge as a trimmed single file | 2026-11-13 | not measured | not measured | not measured | not measured | not measured | 80.4 MB | 17.0 MB | 43 ms |
| 0.10.0, bridge with Native AOT | 2026-11-13 | not measured | not measured | not measured | not measured | not measured | 73.7 MB | 2.2 MB | 9 ms |

Notes on an experiment:

- **Bridge with Native AOT: kept.** Both figures improved: 17.0 MB to 2.2 MB, and a run from
  43 ms to 9 ms. `Sobreiro.exe`, which carries the bridge, went from 80.4 MB to 73.7 MB. The
  Native AOT bridge was built by CI and timed on the PC above, which has no MSVC linker; the
  73.7 MB is the size CI printed for its own build. Both rows are bridge-only runs: the tray was
  not started, since the change does not touch it.
