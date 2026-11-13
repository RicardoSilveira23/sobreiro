# Idle CPU: the pill asks the taskbar only when something may have changed

Part of 1.x, the item carried over from 1.0. Written 2026-11-13.

## Why

The roadmap's target is 5 s of processor time or less per idle hour. 1.0.0 measured 14.9 s.
Ricardo gave the target up for 1.0 in writing and made it the first item of 1.x, to be profiled
before anything is changed.

## What the profile found

Measured 2026-11-13 on `main` at 1.0.0, on scratch copies run beside the installed one (so
their pills were hidden) and signed out of Claude. Processor time from the end of minute 1 to
the end of minute 7, scaled to an hour:

| Copy | CPU per idle hour | Private memory |
|---|---|---|
| As shipped, two runs | 12.2 s and 15.2 s | 91 MB and 105 MB |
| Same exe, pill switched off in its config | 1.4 s | 19 MB |
| Throwaway build: the tick as shipped, without its query | 1.9 s | 76 MB |

- 85 to 90% of the idle time is the UI Automation query the pill host makes on every
  2-second tick: 1,800 queries an hour at 6 to 7 ms each. It asks where the tray icons and the
  task buttons are, whether or not anything moved.
- The rest of the pill host costs about 0.5 s an hour, and the rest of the tray 1.4 s.
- The window-event hooks are not the cost: with nobody at the PC, Explorer sent none.
- One query costs the same whether it fetches the taskbar in several calls, as today, or in
  one (3.6 ms and 3.4 ms in a tight loop, about 11 ms of waiting on Explorer either way). So
  the saving has to come from asking less often.
- The second as-shipped run had one minute of 1.27 s, left out of its figure; it looks like a
  flyout opening and was not checked.

## Goals

- 5 s or less per idle hour on a full `scripts/baseline.ps1` run.
- The pill still finds its place within 2 seconds when someone is using the PC.

## Not goals

- The 2-second tick, the placement rules, the event hooks and the pill's look stay.
- Memory. The profile shows the pill window is about 55 MB of it; that belongs to the shell
  comparison.
- Nothing Sobreiro reads or sends changes.

## What would make it a failure

- A pill that sits over a tray icon, or leaves a gap, for long while someone is at the PC.
- A baseline row that is not clearly under 5 s.

## Approaches

1. **Ask on a sign of change, plus a slow check (picked by Ricardo, 2026-11-13, with the
   slow check at 15 seconds).** Quick when something is happening, slow only when nothing is.
2. One fixed, slower interval of 10 seconds. The smallest change, but about 4.5 s an hour is
   too close to the target, and the pill would be up to 10 seconds late even in use.
3. No polling: UI Automation's own change notifications. The lowest cost if they are reliable
   on this taskbar, which is not known; it needs a spike and would still want a slow check.
4. A cheaper query. Ruled out by the measurement above.

## The design

The pill host keeps its 2-second tick for the cheap checks (the taskbar is there, the window
is alive, the pill is in place). Whether a tick also asks UI Automation is decided by a
schedule in `Sobreiro.Core/Taskbar`, plain logic on a clock the host passes in. A tick asks
when any of these holds:

- nothing has been asked yet since the pill attached to this taskbar;
- the foreground window has switched since the last question was sent (new: the host already
  hears this event, and today only re-places the pill on it);
- the last two answers differ, or there has been only one. "No answer" (a hidden or restarting
  taskbar) counts as an answer, so two of them in a row settle it;
- 15 seconds have passed since the last question. With ticks 2 seconds apart, that is every
  16 seconds.

The questions asked at once today stay as they are and are not part of the schedule's
decision, though it hears of them: a move of the tray area, a move of the taskbar while no
anchors are known, a display change, a DPI change.

So, in use: opening, closing or switching a window is seen at the next tick, as today, and one
more question follows if the answer changed. Left alone: 225 questions an hour instead of
1,800, which is about 3.5 s per idle hour by the figures above.

What it costs: a tray icon that appears or leaves while no window is switched can leave the
pill out of place for up to 16 seconds.

## Calls made here, not by Ricardo

- A foreground switch is seen at the next tick, not at once: the task button of a new window
  appears a moment after the switch, and up to 2 seconds is what every change waits today.
- The clock is a monotonic one, so a change of the PC's time cannot delay a check.
- The measurement goes into `docs/baseline.md` under Experiments, as "1.0.0, taskbar asked on
  change", with the finding in a note; the roadmap's paragraph points at it. No version bump in
  this change: when to release it is Ricardo's call.

## Tests

The schedule gets xUnit tests in `tests/Sobreiro.Tests`: each of the four reasons to ask, the
settling rule with answers and with none, a switch that arrives while a question is out, and
the 15 seconds. The pill host has no unit tests; it is checked by a scratch run beside the
installed copy (the query count and the processor time) and then by the full baseline run.
